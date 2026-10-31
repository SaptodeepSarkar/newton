//! Energy-based voice-activity detection over 16 kHz mono PCM.
//!
//! Frames are 20 ms of audio. Speech starts when a frame's RMS crosses a high
//! threshold and ends only after the signal stays under a low threshold for the
//! hangover window, so a short pause inside a sentence does not split it.
//!
//! Thresholds are in i16 sample units and are deliberately conservative: they
//! gate what the wake detector is allowed to score, nothing more. Real tuning
//! happens in the wake-word phase against ambient audio.

use std::collections::VecDeque;
use std::fmt;

/// Sample rate the harness captures at.
pub const SAMPLE_RATE: u32 = 16_000;
/// Samples in one 20 ms frame.
pub const FRAME_SAMPLES: usize = 320;

/// RMS at or above which silence turns into speech.
const HIGH_RMS: f64 = 1_000.0;
/// RMS at or below which speech starts counting down to its end. Kept well
/// under [`HIGH_RMS`] so a signal hovering between the two does not chatter.
const LOW_RMS: f64 = 400.0;
/// Quiet frames tolerated inside a sentence (200 ms).
const HANGOVER_FRAMES: usize = 10;

/// What one frame of audio was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// No speech.
    Silence,
    /// The first speech frame after silence.
    SpeechStart,
    /// Speech continues, including quiet frames inside the hangover.
    Speech,
    /// The hangover expired; speech is over.
    SpeechEnd,
}

/// A frame that is not the size the detector expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadError {
    /// Wrong number of samples in a frame.
    WrongFrameLength { got: usize, expected: usize },
}

impl fmt::Display for VadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongFrameLength { got, expected } => {
                write!(f, "expected {expected} samples per frame, got {got}")
            }
        }
    }
}

impl std::error::Error for VadError {}

/// One continuous stretch of speech, in samples since the first frame fed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utterance {
    /// First sample of speech.
    pub start_sample: usize,
    /// One past the last speech sample; the trailing hangover is not included.
    pub end_sample: usize,
}

/// Two-threshold energy detector with hangover.
#[derive(Debug, Clone)]
pub struct Vad {
    high: f64,
    low: f64,
    hangover_frames: usize,
    in_speech: bool,
    hangover: usize,
}

impl Vad {
    /// Detector with the default thresholds.
    pub fn new() -> Self {
        Self::with_thresholds(HIGH_RMS, LOW_RMS, HANGOVER_FRAMES)
    }

    /// Detector with explicit thresholds. `low` should not exceed `high`.
    pub fn with_thresholds(high: f64, low: f64, hangover_frames: usize) -> Self {
        Self {
            high,
            low,
            hangover_frames,
            in_speech: false,
            hangover: 0,
        }
    }

    /// True while an utterance is in progress.
    pub fn is_speech(&self) -> bool {
        self.in_speech
    }

    /// Classify one frame of 16-bit mono PCM.
    pub fn push(&mut self, frame: &[i16]) -> Result<Frame, VadError> {
        if frame.len() != FRAME_SAMPLES {
            return Err(VadError::WrongFrameLength {
                got: frame.len(),
                expected: FRAME_SAMPLES,
            });
        }

        let rms = rms(frame);

        if !self.in_speech {
            if rms >= self.high {
                self.in_speech = true;
                self.hangover = 0;
                return Ok(Frame::SpeechStart);
            }
            return Ok(Frame::Silence);
        }

        if rms >= self.low {
            self.hangover = 0;
            return Ok(Frame::Speech);
        }

        self.hangover += 1;
        if self.hangover >= self.hangover_frames {
            self.in_speech = false;
            self.hangover = 0;
            return Ok(Frame::SpeechEnd);
        }
        Ok(Frame::Speech)
    }
}

impl Default for Vad {
    fn default() -> Self {
        Self::new()
    }
}

fn rms(frame: &[i16]) -> f64 {
    let sum: f64 = frame.iter().map(|&s| f64::from(s) * f64::from(s)).sum();
    (sum / frame.len() as f64).sqrt()
}

/// Runs a [`Vad`] and hands back whole utterances in order.
#[derive(Debug, Clone)]
pub struct UtteranceCollector {
    vad: Vad,
    next_sample: usize,
    start: Option<usize>,
    last_speech_end: usize,
    done: VecDeque<Utterance>,
}

impl UtteranceCollector {
    /// Collector with the default detector.
    pub fn new() -> Self {
        Self {
            vad: Vad::new(),
            next_sample: 0,
            start: None,
            last_speech_end: 0,
            done: VecDeque::new(),
        }
    }

    /// Classify a frame and queue an utterance when speech ends.
    pub fn push(&mut self, frame: &[i16]) -> Result<Frame, VadError> {
        let classified = self.vad.push(frame)?;
        let end = self.next_sample + frame.len();
        match classified {
            Frame::SpeechStart => {
                self.start = Some(self.next_sample);
                self.last_speech_end = end;
            }
            // A quiet hangover frame must not move the end.
            Frame::Speech => {
                if self.vad.hangover == 0 {
                    self.last_speech_end = end;
                }
            }
            Frame::SpeechEnd => {
                let start_sample = self.start.take().unwrap_or(self.next_sample);
                self.done.push_back(Utterance {
                    start_sample,
                    end_sample: self.last_speech_end,
                });
            }
            Frame::Silence => {}
        }
        self.next_sample = end;
        Ok(classified)
    }

    /// Take the next finished utterance, if one has ended.
    pub fn pop(&mut self) -> Option<Utterance> {
        self.done.pop_front()
    }
}

impl Default for UtteranceCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AMPLITUDE: i16 = 8_000;

    fn ms_frames(ms: usize) -> usize {
        ms * SAMPLE_RATE as usize / 1000 / FRAME_SAMPLES
    }

    /// Square wave at 200 Hz: deterministic, above the high threshold.
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

    fn classify(vad: &mut Vad, pcm: &[i16]) -> Vec<Frame> {
        pcm.chunks(FRAME_SAMPLES)
            .map(|chunk| vad.push(chunk).expect("frame of the right length"))
            .collect()
    }

    #[test]
    fn pure_silence_stays_silence() {
        let mut vad = Vad::new();
        let frames = classify(&mut vad, &silence(1000));
        assert_eq!(frames.len(), ms_frames(1000));
        assert!(frames.iter().all(|&f| f == Frame::Silence));
        assert!(!vad.is_speech());
    }

    #[test]
    fn a_burst_starts_lasts_and_ends() {
        let mut collector = UtteranceCollector::new();

        let mut pcm = silence(100);
        pcm.extend(tone(400, AMPLITUDE));
        pcm.extend(silence(400));

        let mut frames = Vec::new();
        for chunk in pcm.chunks(FRAME_SAMPLES) {
            frames.push(collector.push(chunk).expect("frame of the right length"));
        }

        assert_eq!(frames[4], Frame::Silence);
        assert_eq!(frames[5], Frame::SpeechStart);
        assert_eq!(frames[12], Frame::Speech);
        assert_eq!(frames[24], Frame::Speech);
        assert_eq!(frames[34], Frame::SpeechEnd);
        assert_eq!(
            collector.pop(),
            Some(Utterance {
                start_sample: 100 * 16,
                end_sample: 500 * 16,
            })
        );
        assert_eq!(collector.pop(), None);
        assert!(!collector.vad.is_speech());
    }

    #[test]
    fn a_short_gap_does_not_end_the_utterance() {
        let mut collector = UtteranceCollector::new();
        assert!(ms_frames(100) < HANGOVER_FRAMES, "gap must fit in hangover");

        let mut pcm = tone(300, AMPLITUDE);
        pcm.extend(silence(100));
        pcm.extend(tone(200, AMPLITUDE));
        pcm.extend(silence(400));

        let mut frames = Vec::new();
        for chunk in pcm.chunks(FRAME_SAMPLES) {
            frames.push(collector.push(chunk).expect("frame of the right length"));
        }

        assert_eq!(
            frames.iter().filter(|&&f| f == Frame::SpeechStart).count(),
            1
        );
        assert_eq!(frames.iter().filter(|&&f| f == Frame::SpeechEnd).count(), 1);
        assert_eq!(
            collector.pop(),
            Some(Utterance {
                start_sample: 0,
                end_sample: 600 * 16,
            })
        );
        assert_eq!(collector.pop(), None);
    }

    #[test]
    fn rejects_a_frame_of_the_wrong_length() {
        let mut vad = Vad::new();
        assert_eq!(
            vad.push(&[0; FRAME_SAMPLES - 1]),
            Err(VadError::WrongFrameLength {
                got: FRAME_SAMPLES - 1,
                expected: FRAME_SAMPLES,
            })
        );
        assert_eq!(
            vad.push(&[0; FRAME_SAMPLES + 1]),
            Err(VadError::WrongFrameLength {
                got: FRAME_SAMPLES + 1,
                expected: FRAME_SAMPLES,
            })
        );
        assert!(vad.push(&[0; FRAME_SAMPLES]).is_ok());
    }
}
