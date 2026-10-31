//! Wake-word state machine fed by [`crate::vad::Frame`]s.
//!
//! The detector itself is injected: a closure scores the speech heard so far
//! in one utterance, so the real model can be dropped in without touching this
//! machine. Nothing here knows what a wake word sounds like.

use crate::vad::{Frame, FRAME_SAMPLES, SAMPLE_RATE};

/// Speech required before the detector is asked anything (300 ms).
pub const MIN_SPEECH_FRAMES: usize = frames(300);
/// Silence required after a wake before another one may fire (1 s).
pub const COOLDOWN_FRAMES: usize = frames(1000);
/// How often the detector is re-asked during one utterance (100 ms).
const SCORE_INTERVAL_FRAMES: usize = frames(100);
/// Score at or above which the wake word is taken as heard.
pub const SCORE_THRESHOLD: f32 = 0.5;
/// Most speech handed to the detector in one utterance (10 s).
const MAX_UTTERANCE_SAMPLES: usize = SAMPLE_RATE as usize * 10;

const fn frames(ms: usize) -> usize {
    ms * SAMPLE_RATE as usize / 1000 / FRAME_SAMPLES
}

/// What the wake machine decided about one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wake {
    /// Nothing heard, or the utterance just ended.
    Idle,
    /// Speech is accumulating; no wake.
    Listening,
    /// The wake word was heard in this frame.
    Woke,
}

/// Consumes VAD frames and calls an injected detector.
pub struct WakeWord<F> {
    score: F,
    armed: bool,
    silence_frames: usize,
    speech_frames: usize,
    next_score_frame: usize,
    fired: bool,
    samples: Vec<i16>,
}

impl<F: FnMut(&[i16]) -> f32> WakeWord<F> {
    /// A machine that asks `score` whether the speech heard so far is a wake
    /// word. It is armed until it fires.
    pub fn new(score: F) -> Self {
        Self {
            score,
            armed: true,
            silence_frames: COOLDOWN_FRAMES,
            speech_frames: 0,
            next_score_frame: MIN_SPEECH_FRAMES,
            fired: false,
            samples: Vec::new(),
        }
    }

    /// Feed one VAD frame and its PCM. `pcm` is only accumulated, never
    /// re-segmented; the VAD owns the framing.
    pub fn push(&mut self, frame: Frame, pcm: &[i16]) -> Wake {
        match frame {
            Frame::Silence => {
                self.end_utterance();
                self.silence_frames += 1;
                if self.silence_frames >= COOLDOWN_FRAMES {
                    self.armed = true;
                }
                Wake::Idle
            }
            Frame::SpeechStart | Frame::Speech => {
                self.silence_frames = 0;
                self.speech_frames += 1;
                // shortcut: buffer capped at 10 s; trim to a window when the real model has one.
                if self.samples.len() < MAX_UTTERANCE_SAMPLES {
                    let room = MAX_UTTERANCE_SAMPLES - self.samples.len();
                    self.samples.extend_from_slice(&pcm[..pcm.len().min(room)]);
                }
                if !self.armed || self.fired || self.speech_frames < self.next_score_frame {
                    return Wake::Listening;
                }
                self.next_score_frame = self.speech_frames + SCORE_INTERVAL_FRAMES;
                if (self.score)(&self.samples) >= SCORE_THRESHOLD {
                    self.fired = true;
                    self.armed = false;
                    return Wake::Woke;
                }
                Wake::Listening
            }
            Frame::SpeechEnd => {
                self.end_utterance();
                Wake::Idle
            }
        }
    }

    fn end_utterance(&mut self) {
        self.speech_frames = 0;
        self.next_score_frame = MIN_SPEECH_FRAMES;
        self.fired = false;
        self.samples.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vad::Vad;

    const AMPLITUDE: i16 = 8_000;

    fn tone(ms: usize, amplitude: i16) -> Vec<i16> {
        let samples = ms * SAMPLE_RATE as usize / 1000;
        (0..samples)
            .map(|i| {
                if (i / 40) % 2 == 0 {
                    amplitude
                } else {
                    -amplitude
                }
            })
            .collect()
    }

    fn silence(ms: usize) -> Vec<i16> {
        vec![0; ms * SAMPLE_RATE as usize / 1000]
    }

    /// Run PCM through the VAD, then the wake machine, returning every decision.
    fn feed<F: FnMut(&[i16]) -> f32>(
        vad: &mut Vad,
        wake: &mut WakeWord<F>,
        pcm: &[i16],
    ) -> Vec<Wake> {
        pcm.chunks(FRAME_SAMPLES)
            .map(|chunk| {
                let frame = vad.push(chunk).expect("frame of the right length");
                wake.push(frame, chunk)
            })
            .collect()
    }

    fn wakes(decisions: &[Wake]) -> usize {
        decisions.iter().filter(|&&d| d == Wake::Woke).count()
    }

    #[test]
    fn wakes_once_per_utterance() {
        let mut vad = Vad::new();
        let mut wake = WakeWord::new(|_| 0.9);

        let mut pcm = silence(100);
        pcm.extend(tone(1000, AMPLITUDE));
        pcm.extend(silence(500));

        let decisions = feed(&mut vad, &mut wake, &pcm);
        assert_eq!(wakes(&decisions), 1);
        // Fires as soon as the minimum speech is in, not at the end.
        let fired_at = decisions.iter().position(|&d| d == Wake::Woke).unwrap();
        assert_eq!(fired_at, 5 + MIN_SPEECH_FRAMES - 1);
    }

    #[test]
    fn two_utterances_within_the_cooldown_wake_once() {
        let mut vad = Vad::new();
        let mut wake = WakeWord::new(|_| 0.9);

        let mut pcm = tone(500, AMPLITUDE);
        pcm.extend(silence(500));
        pcm.extend(tone(500, AMPLITUDE));
        pcm.extend(silence(1200));

        assert_eq!(wakes(&feed(&mut vad, &mut wake, &pcm)), 1);

        // Silence long enough to re-arm, then one more utterance: exactly one.
        let decisions = feed(&mut vad, &mut wake, &tone(500, AMPLITUDE));
        assert_eq!(wakes(&decisions), 1);
    }

    #[test]
    fn a_low_score_never_wakes() {
        let mut vad = Vad::new();
        let mut wake = WakeWord::new(|_| 0.1);

        let decisions = feed(&mut vad, &mut wake, &tone(2000, AMPLITUDE));
        assert_eq!(wakes(&decisions), 0);
    }

    #[test]
    fn silence_alone_never_wakes() {
        let mut vad = Vad::new();
        let mut asked = 0;
        let mut wake = WakeWord::new(|_| {
            asked += 1;
            1.0
        });

        let decisions = feed(&mut vad, &mut wake, &silence(2000));
        assert!(decisions.iter().all(|&d| d == Wake::Idle));
        assert_eq!(asked, 0);
    }
}
