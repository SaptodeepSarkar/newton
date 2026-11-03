//! One turn, from hearing the wake word to going quiet again.
//!
//! Pure state machine: no audio, no clock, no threads. The app feeds it events
//! and reads the phase; a move that could not really happen is refused instead
//! of being applied.

use std::fmt;

pub use crate::fault::Fault;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Heard,
    Thinking,
    Speaking,
    Done,
    Faulted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Wake,
    SpeechEnded,
    Dispatched,
    FirstAudio,
    SpokeEnd,
    Fault(Fault),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnError {
    /// A move that cannot happen in the phase the turn is in.
    OutOfOrder(&'static str),
}

impl fmt::Display for TurnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfOrder(why) => write!(f, "turn out of order: {why}"),
        }
    }
}

impl std::error::Error for TurnError {}

#[derive(Debug)]
pub struct Turn {
    phase: Phase,
    dispatched: bool,
}

impl Default for Turn {
    fn default() -> Self {
        Self::new()
    }
}

impl Turn {
    pub fn new() -> Self {
        Self {
            phase: Phase::Idle,
            dispatched: false,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Apply one event. A refused event changes nothing.
    pub fn on(&mut self, event: Event) -> Result<Phase, TurnError> {
        let next = match (self.phase, event) {
            (_, Event::Fault(_)) => Phase::Faulted,
            // Wake from Done is the next turn: Done -> Idle -> Heard.
            (Phase::Idle | Phase::Done, Event::Wake) => {
                self.dispatched = false;
                Phase::Heard
            }
            (Phase::Heard, Event::SpeechEnded) => Phase::Thinking,
            // Dispatch starts the model; the turn stays in Thinking until it speaks.
            (Phase::Thinking, Event::Dispatched) => {
                self.dispatched = true;
                Phase::Thinking
            }
            (Phase::Thinking, Event::FirstAudio) if self.dispatched => Phase::Speaking,
            (Phase::Speaking, Event::SpokeEnd) => Phase::Done,
            (Phase::Faulted, _) => return Err(TurnError::OutOfOrder("recover first")),
            (_, Event::FirstAudio) => return Err(TurnError::OutOfOrder("audio before dispatch")),
            _ => return Err(TurnError::OutOfOrder("event out of order")),
        };
        self.phase = next;
        Ok(self.phase)
    }

    /// The only way out of Faulted. Anything else is refused.
    pub fn recover(&mut self) -> Result<Phase, TurnError> {
        if self.phase != Phase::Faulted {
            return Err(TurnError::OutOfOrder("nothing to recover"));
        }
        self.phase = Phase::Idle;
        Ok(self.phase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn_in(events: &[Event]) -> Turn {
        let mut turn = Turn::new();
        for event in events {
            turn.on(*event).expect("legal move");
        }
        turn
    }

    #[test]
    fn a_turn_runs_end_to_end() {
        let mut turn = Turn::new();
        assert_eq!(turn.phase(), Phase::Idle);
        for (event, expected) in [
            (Event::Wake, Phase::Heard),
            (Event::SpeechEnded, Phase::Thinking),
            (Event::Dispatched, Phase::Thinking),
            (Event::FirstAudio, Phase::Speaking),
            (Event::SpokeEnd, Phase::Done),
        ] {
            assert_eq!(turn.on(event).expect("legal move"), expected);
        }
    }

    #[test]
    fn audio_before_dispatch_is_refused() {
        let mut turn = turn_in(&[Event::Wake, Event::SpeechEnded]);
        assert_eq!(
            turn.on(Event::FirstAudio).unwrap_err(),
            TurnError::OutOfOrder("audio before dispatch")
        );
        assert_eq!(turn.phase(), Phase::Thinking);
    }

    #[test]
    fn a_fault_from_every_live_phase_reaches_faulted() {
        for events in [
            &[Event::Wake][..],
            &[Event::Wake, Event::SpeechEnded][..],
            &[
                Event::Wake,
                Event::SpeechEnded,
                Event::Dispatched,
                Event::FirstAudio,
            ][..],
        ] {
            let mut turn = turn_in(events);
            assert_eq!(
                turn.on(Event::Fault(Fault::Unknown)).expect("fault"),
                Phase::Faulted
            );
        }
    }

    #[test]
    fn recover_returns_to_idle() {
        let mut turn = Turn::new();
        turn.on(Event::Wake).unwrap();
        turn.on(Event::Fault(Fault::Unknown)).unwrap();
        assert_eq!(turn.recover().expect("recover"), Phase::Idle);
        assert!(turn.recover().is_err());
    }

    #[test]
    fn a_second_wake_is_refused() {
        let mut turn = turn_in(&[Event::Wake]);
        assert!(turn.on(Event::Wake).is_err());
        assert_eq!(turn.phase(), Phase::Heard);
    }

    #[test]
    fn spoke_end_without_speaking_is_refused() {
        let mut turn = turn_in(&[Event::Wake, Event::SpeechEnded]);
        assert!(turn.on(Event::SpokeEnd).is_err());
        assert_eq!(turn.phase(), Phase::Thinking);
    }
}
