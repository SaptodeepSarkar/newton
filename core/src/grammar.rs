//! The decoding side of the tool boundary: what a model may even say.
//!
//! `parse` decides whether a finished call is allowed. This module is what runs
//! before that, while the model is still generating: the four wire names it may
//! pick from, a GBNF grammar that admits nothing else, and a hint for the log
//! when a model gets the name almost right.

use crate::{parse, Call, Reject};

/// The only tool names that exist, exactly as the model must write them.
pub const TOOLS: [&str; 4] = ["calendar_read", "mail_read", "remember_fact", "forget_fact"];

/// True for a name the model is allowed to emit.
pub fn allowed(name: &str) -> bool {
    TOOLS.contains(&name)
}

/// The grammar handed to the decoder. One object, one of four names, the
/// argument shape the name implies, nothing else.
pub fn gbnf() -> String {
    GRAMMAR.replace("NAME", &TOOLS.join(" | "))
}

const GRAMMAR: &str = r#"root   ::= object
object ::= "{" ws "\"tool\"" ws ":" ws name ("," ws arg)? ws "}"
name   ::= "\"NAME\""
arg    ::= "\"days\"" ws ":" ws integer | "\"text\"" ws ":" ws string
integer::= [1-9] [0-9]?
string ::= "\"" [^"\\]{1,200} "\""
ws     ::= [ ]?
"#;

/// `parse`, with the model's near-miss named in the log.
pub fn accept(raw: &str) -> Result<Call, Reject> {
    match parse(raw) {
        Err(Reject::UnknownTool(name)) => {
            let near = hint(&name);
            Err(Reject::Malformed(match near {
                Some(tool) => format!("unknown tool {name:?}, did you mean {tool:?}"),
                None => format!("unknown tool {name:?}"),
            }))
        }
        other => other,
    }
}

/// The closest real tool to a name a model made up, if it is close at all.
pub fn hint(name: &str) -> Option<&'static str> {
    let wanted = squash(name);
    TOOLS.iter().copied().find(|tool| squash(tool) == wanted)
}

fn squash(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;

    #[test]
    fn the_grammar_names_every_tool_and_no_other() {
        let grammar = gbnf();
        for tool in TOOLS {
            assert_eq!(grammar.matches(tool).count(), 1, "{tool} must appear once");
        }
        assert!(!grammar.contains("shell"));
        assert!(!grammar.contains("delete_all"));
    }

    #[test]
    fn the_names_match_the_tool_enum() {
        let enum_names: Vec<&str> = [
            Tool::CalendarRead,
            Tool::MailRead,
            Tool::RememberFact,
            Tool::ForgetFact,
        ]
        .iter()
        .map(|t| t.name())
        .collect();
        assert_eq!(enum_names, TOOLS.to_vec());
    }

    #[test]
    fn a_near_miss_is_rejected_and_the_hint_points_at_the_real_name() {
        let raw = r#"{"tool":"CalendarRead","days":7}"#;
        let why = accept(raw).unwrap_err();
        let text = format!("{why}");
        assert!(text.contains("calendar_read"), "{text}");
        assert_eq!(hint("calendar-read"), Some("calendar_read"));
        assert_eq!(hint("mail"), None);
    }

    #[test]
    fn a_valid_call_goes_through_untouched() {
        let raw = r#"{"tool":"mail_read","days":7}"#;
        assert_eq!(accept(raw), parse(raw));
        assert!(accept(raw).is_ok());
    }

    #[test]
    fn every_advertised_tool_actually_parses() {
        for raw in [
            r#"{"tool":"calendar_read","days":1}"#,
            r#"{"tool":"mail_read","days":31}"#,
            r#"{"tool":"remember_fact","text":"the milk is on the second shelf"}"#,
            r#"{"tool":"forget_fact","text":"milk"}"#,
        ] {
            assert!(accept(raw).is_ok(), "{raw} should parse");
        }
    }

    #[test]
    fn allowed_is_exact_not_fuzzy() {
        assert!(allowed("calendar_read"));
        assert!(!allowed("CalendarRead"));
        assert!(!allowed("calendar read"));
        assert!(!allowed(""));
    }
}
