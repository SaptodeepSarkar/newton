//! Run one validated call against the host.
//!
//! The host is the only thing that touches user data. A [`Call`] that carries
//! the wrong kind of argument is refused before the host is reached.

use crate::{Call, Tool};

/// Why the host could not answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    Unavailable,
    Failed(String),
}

/// What the user should hear, or what was written to memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Spoken(String),
    Remembered,
    Forgotten(usize),
}

/// Why a call did not produce an outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecError {
    Host(Tool, HostError),
    WrongShape,
}

/// The side of the world Newton is allowed to touch.
pub trait ToolHost {
    fn calendar(&mut self, days: u8) -> Result<String, HostError>;
    fn mail(&mut self, days: u8) -> Result<String, HostError>;
    fn remember(&mut self, text: &str) -> Result<(), HostError>;
    fn forget(&mut self, text: &str) -> Result<usize, HostError>;
}

/// Run a validated call. Every arm calls exactly one host method.
pub fn run(call: &Call, host: &mut dyn ToolHost) -> Result<Outcome, ExecError> {
    let fail = |why| ExecError::Host(call.tool, why);
    match (call.tool, call.days, call.text.as_deref()) {
        (Tool::CalendarRead, Some(days), None) => {
            host.calendar(days).map(Outcome::Spoken).map_err(fail)
        }
        (Tool::MailRead, Some(days), None) => host.mail(days).map(Outcome::Spoken).map_err(fail),
        (Tool::RememberFact, None, Some(text)) => host
            .remember(text)
            .map(|()| Outcome::Remembered)
            .map_err(fail),
        (Tool::ForgetFact, None, Some(text)) => {
            host.forget(text).map(Outcome::Forgotten).map_err(fail)
        }
        _ => Err(ExecError::WrongShape),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        calls: Vec<String>,
        error: Option<HostError>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                calls: Vec::new(),
                error: None,
            }
        }

        fn failing(error: HostError) -> Self {
            Self {
                calls: Vec::new(),
                error: Some(error),
            }
        }

        fn enter(&mut self, call: String) -> Result<(), HostError> {
            self.calls.push(call);
            match &self.error {
                Some(error) => Err(error.clone()),
                None => Ok(()),
            }
        }
    }

    impl ToolHost for Fake {
        fn calendar(&mut self, days: u8) -> Result<String, HostError> {
            self.enter(format!("calendar({days})"))?;
            Ok(format!("{days} quiet days"))
        }

        fn mail(&mut self, days: u8) -> Result<String, HostError> {
            self.enter(format!("mail({days})"))?;
            Ok(format!("{days} quiet inboxes"))
        }

        fn remember(&mut self, text: &str) -> Result<(), HostError> {
            self.enter(format!("remember({text})"))?;
            Ok(())
        }

        fn forget(&mut self, text: &str) -> Result<usize, HostError> {
            self.enter(format!("forget({text})"))?;
            Ok(3)
        }
    }

    fn call(tool: Tool, days: Option<u8>, text: Option<&str>) -> Call {
        Call {
            tool,
            days,
            text: text.map(str::to_string),
        }
    }

    #[test]
    fn each_tool_routes_to_its_own_host_method() {
        let mut fake = Fake::new();

        let out = run(&call(Tool::CalendarRead, Some(7), None), &mut fake).unwrap();
        assert_eq!(out, Outcome::Spoken("7 quiet days".to_string()));
        let out = run(&call(Tool::MailRead, Some(2), None), &mut fake).unwrap();
        assert_eq!(out, Outcome::Spoken("2 quiet inboxes".to_string()));
        let out = run(
            &call(Tool::RememberFact, None, Some("likes tea")),
            &mut fake,
        )
        .unwrap();
        assert_eq!(out, Outcome::Remembered);
        run(&call(Tool::ForgetFact, None, Some("likes tea")), &mut fake).unwrap();

        assert_eq!(
            fake.calls,
            [
                "calendar(7)",
                "mail(2)",
                "remember(likes tea)",
                "forget(likes tea)"
            ]
        );
    }

    #[test]
    fn a_host_error_keeps_the_tool_name() {
        let mut fake = Fake::failing(HostError::Unavailable);
        let err = run(&call(Tool::MailRead, Some(1), None), &mut fake).unwrap_err();
        assert_eq!(err, ExecError::Host(Tool::MailRead, HostError::Unavailable));
    }

    #[test]
    fn a_wrong_shape_never_calls_the_host() {
        let mut fake = Fake::new();
        for bad in [
            call(Tool::CalendarRead, None, None),
            call(Tool::CalendarRead, Some(7), Some("likes tea")),
            call(Tool::MailRead, None, None),
            call(Tool::RememberFact, Some(7), None),
            call(Tool::RememberFact, None, None),
            call(Tool::ForgetFact, Some(3), Some("likes tea")),
            call(Tool::ForgetFact, None, None),
        ] {
            assert_eq!(run(&bad, &mut fake), Err(ExecError::WrongShape));
        }
        assert!(fake.calls.is_empty());
    }

    #[test]
    fn forget_reports_the_count() {
        let mut fake = Fake::new();
        let out = run(&call(Tool::ForgetFact, None, Some("likes tea")), &mut fake).unwrap();
        assert_eq!(out, Outcome::Forgotten(3));
    }

    #[test]
    fn unicode_text_passes_through_unchanged() {
        let text = "Amma ko \"Good morning\" बोलना — 7:30 बजे";
        let mut fake = Fake::new();
        run(&call(Tool::RememberFact, None, Some(text)), &mut fake).unwrap();
        assert_eq!(fake.calls, [format!("remember({text})")]);
    }

    #[test]
    fn a_host_error_string_is_preserved() {
        let why = "mailbox closed: 5 unread\nplease retry".to_string();
        let mut fake = Fake::failing(HostError::Failed(why.clone()));
        let err = run(&call(Tool::ForgetFact, None, Some("likes tea")), &mut fake).unwrap_err();
        assert_eq!(
            err,
            ExecError::Host(Tool::ForgetFact, HostError::Failed(why))
        );
    }
}
