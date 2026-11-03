//! Every way a turn can go wrong: one sentence the user hears, and whether
//! trying again can help.

/// Everything that can stop a turn before the user hears an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    WakeLost,
    NoMic,
    ModelMissing,
    ModelCorrupt,
    Quota,
    Revoked,
    Timeout,
    Unknown,
}

/// What the app should do about a fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    Retry,
    AskUser(&'static str),
    Gone,
}

/// Attempts worth making before the user is asked to step in.
const RETRY_LIMIT: u32 = 3;

impl Fault {
    /// The orb state to show. Every fault looks the same: the design has no
    /// per-fault look, the sentence carries the detail.
    pub fn ring(&self) -> &'static str {
        "error"
    }

    /// One short sentence, spoken. No jargon, no stack traces, no promise that
    /// it worked.
    pub fn spoken(&self) -> &'static str {
        match self {
            Self::WakeLost => "I missed the word that wakes me up. Say my name again.",
            Self::NoMic => "I can't hear you. Check the microphone.",
            Self::ModelMissing => "The model file isn't here. Put it back and try again.",
            Self::ModelCorrupt => "The model file is damaged. Download it again.",
            Self::Quota => "I've used today's allowance. Try again later.",
            Self::Revoked => "Access was taken away. You'll need to approve it again.",
            Self::Timeout => "That took too long. Try again.",
            Self::Unknown => "Something went wrong. Try again.",
        }
    }

    /// True only where waiting and trying again can change the outcome.
    pub fn retryable(&self) -> bool {
        matches!(self, Self::Quota | Self::Timeout)
    }

    /// Retry while that can help, then hand it to the user. Never `Gone`: a
    /// fault alone must not end the turn without telling anyone.
    pub fn recover(&self, attempt: u32) -> Recovery {
        if self.retryable() && attempt < RETRY_LIMIT {
            Recovery::Retry
        } else {
            Recovery::AskUser(self.spoken())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Fault; 8] = [
        Fault::WakeLost,
        Fault::NoMic,
        Fault::ModelMissing,
        Fault::ModelCorrupt,
        Fault::Quota,
        Fault::Revoked,
        Fault::Timeout,
        Fault::Unknown,
    ];

    #[test]
    fn every_fault_speaks_and_looks_like_an_error() {
        for f in ALL {
            let s = f.spoken();
            assert!(!s.is_empty(), "{f:?}");
            assert!(s.chars().count() < 90, "{f:?} too long");
            assert_eq!(f.ring(), "error");
        }
    }

    #[test]
    fn quota_retries_revoked_does_not() {
        assert!(Fault::Quota.retryable());
        assert!(!Fault::Revoked.retryable());
    }

    #[test]
    fn retries_turn_into_asking_the_user() {
        assert_eq!(Fault::Timeout.recover(2), Recovery::Retry);
        assert!(matches!(Fault::Timeout.recover(3), Recovery::AskUser(_)));
    }

    #[test]
    fn nothing_is_dropped_silently() {
        for f in ALL {
            for attempt in 0..6 {
                assert_ne!(
                    f.recover(attempt),
                    Recovery::Gone,
                    "{f:?} attempt {attempt}"
                );
            }
        }
    }

    #[test]
    fn sentences_are_distinct() {
        for (i, a) in ALL.iter().enumerate() {
            for b in &ALL[i + 1..] {
                assert_ne!(a.spoken(), b.spoken(), "{a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn sentences_do_not_say_error_or_fail() {
        for f in ALL {
            let s = f.spoken().to_lowercase();
            assert!(!s.contains("error"), "{f:?}");
            assert!(!s.contains("fail"), "{f:?}");
        }
    }
}
