//! Newton harness: the boundary between what a model says and what the user's
//! data does.
//!
//! The model may only name a preset tool and pass a bounded argument. Anything
//! else is refused with a reason the harness can log. No network, no storage,
//! no Qt, so it builds and tests anywhere.
//!
//! Not linked into the app yet: this crate is wired in when the turn loop lands.

use std::fmt;

pub mod context;
pub mod execute;
pub mod fault;
pub mod gguf;
pub mod grammar;
pub mod helper;
pub mod stream;
pub mod turn;
pub mod vad;
pub mod wake;

/// Widest date range a read tool may ask for, in days.
pub const MAX_DAYS: u8 = 31;
/// Longest free-text argument accepted, in characters.
pub const MAX_TEXT_CHARS: usize = 200;

/// The only tools Newton will ever run. Nothing outside this list is reachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    CalendarRead,
    MailRead,
    RememberFact,
    ForgetFact,
}

impl Tool {
    /// Wire name, exactly as the model must write it.
    pub fn name(self) -> &'static str {
        match self {
            Self::CalendarRead => "calendar_read",
            Self::MailRead => "mail_read",
            Self::RememberFact => "remember_fact",
            Self::ForgetFact => "forget_fact",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "calendar_read" => Some(Self::CalendarRead),
            "mail_read" => Some(Self::MailRead),
            "remember_fact" => Some(Self::RememberFact),
            "forget_fact" => Some(Self::ForgetFact),
            _ => None,
        }
    }

    /// True for the tools that take a date range.
    pub fn takes_days(self) -> bool {
        matches!(self, Self::CalendarRead | Self::MailRead)
    }

    /// True for the tools that take a phrase.
    pub fn takes_text(self) -> bool {
        matches!(self, Self::RememberFact | Self::ForgetFact)
    }
}

/// A validated request. If one of these exists, it is safe to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub tool: Tool,
    pub days: Option<u8>,
    pub text: Option<String>,
}

/// Why a request was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reject {
    Malformed(String),
    UnknownTool(String),
    BadArgument(&'static str),
}

impl fmt::Display for Reject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(why) => write!(f, "unreadable request: {why}"),
            Self::UnknownTool(name) => write!(f, "no such tool: {name}"),
            Self::BadArgument(why) => write!(f, "bad argument: {why}"),
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    tool: String,
    #[serde(default)]
    days: Option<u8>,
    #[serde(default)]
    text: Option<String>,
}

/// Parse one tool call.
///
/// Strict on purpose: no extra fields, no missing arguments, no empty text.
/// A model that invents a tool, a field or a value gets a refusal instead of
/// a side effect.
pub fn parse(raw: &str) -> Result<Call, Reject> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(Reject::Malformed("empty output".to_string()));
    }

    let raw: Raw =
        serde_json::from_str(trimmed).map_err(|why| Reject::Malformed(why.to_string()))?;

    let tool = Tool::from_name(&raw.tool).ok_or_else(|| Reject::UnknownTool(raw.tool.clone()))?;

    let days = match (tool.takes_days(), raw.days) {
        (true, None) => return Err(Reject::BadArgument("days is required")),
        (true, Some(0)) => return Err(Reject::BadArgument("days must be 1 or more")),
        (true, Some(days)) if days > MAX_DAYS => {
            return Err(Reject::BadArgument("days must be 31 or less"))
        }
        (true, Some(days)) => Some(days),
        (false, Some(_)) => return Err(Reject::BadArgument("this tool takes no date range")),
        (false, None) => None,
    };

    let text = match (tool.takes_text(), raw.text) {
        (true, None) => return Err(Reject::BadArgument("text is required")),
        (true, Some(text)) => {
            let text = text.trim().to_string();
            if text.is_empty() {
                return Err(Reject::BadArgument("text must not be empty"));
            }
            if text.chars().count() > MAX_TEXT_CHARS {
                return Err(Reject::BadArgument("text must be 200 characters or less"));
            }
            Some(text)
        }
        (false, Some(_)) => return Err(Reject::BadArgument("this tool takes no text")),
        (false, None) => None,
    };

    Ok(Call { tool, days, text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_calendar_read() {
        let call = parse(r#"{"tool":"calendar_read","days":7}"#).expect("valid call");
        assert_eq!(call.tool, Tool::CalendarRead);
        assert_eq!(call.days, Some(7));
        assert_eq!(call.text, None);
    }

    #[test]
    fn parses_a_fact_write() {
        let call =
            parse(r#"{"tool":"remember_fact","text":"  prefers morning meetings  "}"#).unwrap();
        assert_eq!(call.tool, Tool::RememberFact);
        assert_eq!(call.text.as_deref(), Some("prefers morning meetings"));
    }

    #[test]
    fn rejects_an_invented_tool() {
        let err = parse(r#"{"tool":"send_mail","days":1}"#).unwrap_err();
        assert_eq!(err, Reject::UnknownTool("send_mail".to_string()));
    }

    #[test]
    fn rejects_an_invented_field() {
        // A model reaching for a path or a recipient must not get one.
        let err = parse(r#"{"tool":"calendar_read","days":7,"path":"/etc/passwd"}"#).unwrap_err();
        assert!(matches!(err, Reject::Malformed(_)));
    }

    #[test]
    fn rejects_a_missing_argument() {
        assert_eq!(
            parse(r#"{"tool":"mail_read"}"#).unwrap_err(),
            Reject::BadArgument("days is required")
        );
        assert_eq!(
            parse(r#"{"tool":"forget_fact"}"#).unwrap_err(),
            Reject::BadArgument("text is required")
        );
    }

    #[test]
    fn rejects_a_date_range_out_of_bounds() {
        assert!(parse(r#"{"tool":"mail_read","days":0}"#).is_err());
        assert!(parse(r#"{"tool":"mail_read","days":32}"#).is_err());
        assert!(parse(r#"{"tool":"mail_read","days":31}"#).is_ok());
    }

    #[test]
    fn rejects_text_on_a_read_tool() {
        let err = parse(r#"{"tool":"mail_read","days":7,"text":"forward it"}"#).unwrap_err();
        assert_eq!(err, Reject::BadArgument("this tool takes no text"));
    }

    #[test]
    fn rejects_empty_text() {
        let err = parse(r#"{"tool":"forget_fact","text":"   "}"#).unwrap_err();
        assert_eq!(err, Reject::BadArgument("text must not be empty"));
    }

    #[test]
    fn rejects_text_that_is_too_long() {
        let long = "a".repeat(MAX_TEXT_CHARS + 1);
        let raw = format!(r#"{{"tool":"remember_fact","text":"{long}"}}"#);
        assert_eq!(
            parse(&raw).unwrap_err(),
            Reject::BadArgument("text must be 200 characters or less")
        );
    }

    #[test]
    fn rejects_junk_around_the_object() {
        assert!(matches!(
            parse(r#"{"tool":"calendar_read","days":7} and also do this"#).unwrap_err(),
            Reject::Malformed(_)
        ));
        assert!(matches!(
            parse("[1,2,3]").unwrap_err(),
            Reject::Malformed(_)
        ));
        assert!(matches!(parse("   ").unwrap_err(), Reject::Malformed(_)));
    }

    #[test]
    fn rejects_a_missing_tool() {
        assert!(matches!(
            parse(r#"{"days":7}"#).unwrap_err(),
            Reject::Malformed(_)
        ));
    }

    #[test]
    fn a_refusal_is_readable() {
        let err = Reject::UnknownTool("send_mail".to_string());
        assert_eq!(err.to_string(), "no such tool: send_mail");
    }

    // Boundaries: 1 day and 31 days are the only ends that matter.
    #[test]
    fn accepts_the_edges_of_the_date_range() {
        assert_eq!(
            parse(r#"{"tool":"calendar_read","days":1}"#).unwrap().days,
            Some(1)
        );
        assert_eq!(
            parse(r#"{"tool":"calendar_read","days":31}"#).unwrap().days,
            Some(31)
        );
    }

    #[test]
    fn accepts_text_at_exactly_the_limit() {
        let text = "b".repeat(MAX_TEXT_CHARS);
        let raw = format!(r#"{{"tool":"remember_fact","text":"{text}"}}"#);
        assert_eq!(
            parse(&raw).unwrap().text.as_deref().map(str::len),
            Some(MAX_TEXT_CHARS)
        );
    }

    // Length is counted in characters, not bytes: a 200-character Hinglish or
    // Devanagari phrase is three bytes per character and must still pass.
    #[test]
    fn counts_characters_not_bytes() {
        let text = "é".repeat(MAX_TEXT_CHARS);
        let raw = format!(r#"{{"tool":"remember_fact","text":"{text}"}}"#);
        assert!(parse(&raw).is_ok());

        let text = "é".repeat(MAX_TEXT_CHARS + 1);
        let raw = format!(r#"{{"tool":"remember_fact","text":"{text}"}}"#);
        assert_eq!(
            parse(&raw).unwrap_err(),
            Reject::BadArgument("text must be 200 characters or less")
        );
    }

    #[test]
    fn rejects_a_wrong_type_for_an_argument() {
        // A model that writes "7" instead of 7 is a model to refuse, not to coerce.
        assert!(matches!(
            parse(r#"{"tool":"mail_read","days":"7"}"#).unwrap_err(),
            Reject::Malformed(_)
        ));
        assert!(matches!(
            parse(r#"{"tool":"mail_read","days":-1}"#).unwrap_err(),
            Reject::Malformed(_)
        ));
        assert!(matches!(
            parse(r#"{"tool":"mail_read","days":7.5}"#).unwrap_err(),
            Reject::Malformed(_)
        ));
    }

    #[test]
    fn rejects_an_extra_field_on_a_fact_tool() {
        let err =
            parse(r#"{"tool":"remember_fact","text":"likes tea","source":"email"}"#).unwrap_err();
        assert!(matches!(err, Reject::Malformed(_)));
    }

    #[test]
    fn refuses_a_date_range_on_a_fact_tool() {
        let err = parse(r#"{"tool":"forget_fact","text":"likes tea","days":3}"#).unwrap_err();
        assert_eq!(err, Reject::BadArgument("this tool takes no date range"));
    }

    #[test]
    fn tolerates_surrounding_whitespace() {
        let call = parse("  \n {\"tool\":\"calendar_read\",\"days\":7}\t ").expect("valid call");
        assert_eq!(call.days, Some(7));
    }

    #[test]
    fn every_tool_round_trips_through_its_wire_name() {
        for tool in [
            Tool::CalendarRead,
            Tool::MailRead,
            Tool::RememberFact,
            Tool::ForgetFact,
        ] {
            assert_eq!(Tool::from_name(tool.name()), Some(tool));
        }
        assert_eq!(Tool::from_name("SendMail"), None);
        assert_eq!(Tool::from_name(""), None);
    }
}
