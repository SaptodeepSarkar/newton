//! When the first audio arrives, measured instead of guessed.
//!
//! No wall clock, no threads, no sleeps: a turn is timed against an injected
//! clock so a test can prove the arithmetic, and a real run can prove the GPU.

use std::cell::Cell;

/// Where the time comes from. Production passes a monotonic clock; tests do not.
pub trait Clock {
    fn now_ms(&self) -> u64;
}

/// A clock a test drives by hand. Interior mutability so a running timer can
/// hold it while the test moves time forward.
#[derive(Debug, Default)]
pub struct ManualClock {
    ms: Cell<u64>,
}

impl ManualClock {
    pub fn new(ms: u64) -> Self {
        Self { ms: Cell::new(ms) }
    }

    pub fn advance(&self, by: u64) {
        self.ms.set(self.ms.get() + by);
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.ms.get()
    }
}

/// The promise made to a waiting user, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub first_audio_ms: u64,
    pub total_ms: u64,
}

/// One completed turn, all offsets from the moment the user stopped speaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnReport {
    pub heard_to_dispatched_ms: u64,
    pub dispatch_to_first_audio_ms: u64,
    pub first_audio_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Within,
    OverFirstAudio,
    OverTotal,
    Both,
}

impl Budget {
    pub fn check(&self, report: &TurnReport) -> Verdict {
        match (
            report.first_audio_ms > self.first_audio_ms,
            report.total_ms > self.total_ms,
        ) {
            (true, true) => Verdict::Both,
            (true, false) => Verdict::OverFirstAudio,
            (false, true) => Verdict::OverTotal,
            (false, false) => Verdict::Within,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TimerError {
    /// Marks must arrive in the order the turn really happens in.
    OutOfOrder(&'static str),
}

pub struct TurnTimer<'a> {
    clock: &'a dyn Clock,
    heard: u64,
    dispatched: Option<u64>,
    first_audio: Option<u64>,
    done: Option<u64>,
}

impl<'a> TurnTimer<'a> {
    /// The user has stopped speaking. This is time zero.
    pub fn start(clock: &'a dyn Clock) -> Self {
        Self {
            clock,
            heard: clock.now_ms(),
            dispatched: None,
            first_audio: None,
            done: None,
        }
    }

    pub fn dispatched(&mut self) -> Result<(), TimerError> {
        if self.dispatched.is_some() {
            return Err(TimerError::OutOfOrder("dispatched twice"));
        }
        self.dispatched = Some(self.clock.now_ms());
        Ok(())
    }

    pub fn first_audio(&mut self) -> Result<(), TimerError> {
        if self.dispatched.is_none() {
            return Err(TimerError::OutOfOrder("audio before dispatch"));
        }
        if self.first_audio.is_some() {
            return Err(TimerError::OutOfOrder("first audio twice"));
        }
        self.first_audio = Some(self.clock.now_ms());
        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), TimerError> {
        if self.first_audio.is_none() {
            return Err(TimerError::OutOfOrder("finish before first audio"));
        }
        self.done = Some(self.clock.now_ms());
        Ok(())
    }

    pub fn report(&self) -> Result<TurnReport, TimerError> {
        let dispatched = self
            .dispatched
            .ok_or(TimerError::OutOfOrder("never dispatched"))?;
        let first_audio = self
            .first_audio
            .ok_or(TimerError::OutOfOrder("never spoke"))?;
        let done = self.done.ok_or(TimerError::OutOfOrder("never finished"))?;
        Ok(TurnReport {
            heard_to_dispatched_ms: dispatched - self.heard,
            dispatch_to_first_audio_ms: first_audio - dispatched,
            first_audio_ms: first_audio - self.heard,
            total_ms: done - self.heard,
        })
    }
}

/// The last 64 first-audio samples, so a p95 can be quoted without a histogram.
#[derive(Debug, Default)]
pub struct TurnStats {
    samples: Vec<u64>,
}

impl TurnStats {
    pub const KEEP: usize = 64;

    pub fn push(&mut self, first_audio_ms: u64) {
        if self.samples.len() == Self::KEEP {
            self.samples.remove(0);
        }
        self.samples.push(first_audio_ms);
    }

    fn percentile(&self, p: usize) -> Option<u64> {
        if self.samples.is_empty() {
            return None;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        // Nearest-rank: the smallest value with at least p% of samples at or below.
        let rank = (p * sorted.len()).div_ceil(100).max(1);
        Some(sorted[rank - 1])
    }

    pub fn p50(&self) -> Option<u64> {
        self.percentile(50)
    }

    pub fn p95(&self) -> Option<u64> {
        self.percentile(95)
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(clock: &ManualClock) -> TurnReport {
        let mut timer = TurnTimer::start(clock);
        timer.dispatched().unwrap();
        timer.first_audio().unwrap();
        timer.finish().unwrap();
        timer.report().unwrap()
    }

    #[test]
    fn a_quick_turn_is_within_budget() {
        let clock = ManualClock::new(1_000);
        let mut timer = TurnTimer::start(&clock);
        clock.advance(40);
        timer.dispatched().unwrap();
        clock.advance(300);
        timer.first_audio().unwrap();
        clock.advance(900);
        timer.finish().unwrap();

        let report = timer.report().unwrap();
        let budget = Budget {
            first_audio_ms: 600,
            total_ms: 4_000,
        };
        assert_eq!(report.heard_to_dispatched_ms, 40);
        assert_eq!(report.dispatch_to_first_audio_ms, 300);
        assert_eq!(report.first_audio_ms, 340);
        assert_eq!(report.total_ms, 1_240);
        assert_eq!(budget.check(&report), Verdict::Within);
    }

    #[test]
    fn a_late_first_word_is_named_as_such() {
        let clock = ManualClock::new(0);
        let mut timer = TurnTimer::start(&clock);
        clock.advance(900);
        timer.dispatched().unwrap();
        clock.advance(800);
        timer.first_audio().unwrap();
        clock.advance(100);
        timer.finish().unwrap();

        let budget = Budget {
            first_audio_ms: 1_000,
            total_ms: 4_000,
        };
        assert_eq!(
            budget.check(&timer.report().unwrap()),
            Verdict::OverFirstAudio
        );
    }

    #[test]
    fn a_long_answer_is_over_total_not_over_first_audio() {
        let clock = ManualClock::new(0);
        let mut timer = TurnTimer::start(&clock);
        clock.advance(10);
        timer.dispatched().unwrap();
        clock.advance(100);
        timer.first_audio().unwrap();
        clock.advance(9_000);
        timer.finish().unwrap();

        let budget = Budget {
            first_audio_ms: 1_000,
            total_ms: 4_000,
        };
        assert_eq!(budget.check(&timer.report().unwrap()), Verdict::OverTotal);
    }

    #[test]
    fn marks_in_the_wrong_order_are_refused_not_panicked() {
        let clock = ManualClock::new(0);
        let mut timer = TurnTimer::start(&clock);
        assert_eq!(
            timer.first_audio(),
            Err(TimerError::OutOfOrder("audio before dispatch"))
        );
        assert_eq!(
            timer.finish(),
            Err(TimerError::OutOfOrder("finish before first audio"))
        );
        assert!(timer.report().is_err());
        timer.dispatched().unwrap();
        assert!(timer.dispatched().is_err());
    }

    #[test]
    fn percentiles_match_a_hand_computed_set() {
        // 1..=50: nearest rank puts p50 at 25 and p95 at 48.
        let mut stats = TurnStats::default();
        for ms in 1..=50 {
            stats.push(ms);
        }
        assert_eq!(stats.p50(), Some(25));
        assert_eq!(stats.p95(), Some(48));

        // Only the last 64 survive: 37..=100, so p50 is 68 and p95 is 97.
        let mut stats = TurnStats::default();
        for ms in 1..=100 {
            stats.push(ms);
        }
        assert_eq!(stats.len(), TurnStats::KEEP);
        assert_eq!(stats.p50(), Some(68));
        assert_eq!(stats.p95(), Some(97));
    }

    #[test]
    fn an_empty_run_has_no_percentile() {
        let stats = TurnStats::default();
        assert!(stats.is_empty());
        assert_eq!(stats.p50(), None);
        assert_eq!(stats.p95(), None);
    }

    #[test]
    fn a_tiny_turn_still_times() {
        let clock = ManualClock::new(5);
        let report = turn(&clock);
        assert_eq!(report.total_ms, 0);
        assert_eq!(report.first_audio_ms, 0);
    }
}
