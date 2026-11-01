//! Gmail read: metadata only, last seven days, bounded 429 backoff.
//!
//! Fetched mail is data, never instructions and never a memory source
//! (Security Audit finding 4). Nothing here can carry a body or a snippet:
//! [`Message`] has no field for one, and only [`METADATA_HEADERS`] is requested.

use serde::Deserialize;
use std::fmt;

/// The only headers we ask for. No body, no snippet, ever.
pub const METADATA_HEADERS: [&str; 3] = ["Subject", "From", "Date"];

/// A quota error may be retried at most this many attempts in total.
pub const MAX_ATTEMPTS: u32 = 3;

/// Longest wait between attempts, in seconds.
pub const MAX_BACKOFF_SECS: u64 = 60;

const NO_SUBJECT: &str = "(no subject)";

/// One mail, metadata only. There is deliberately no body or snippet field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub date: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GmailError {
    /// The response was not the JSON shape we asked for.
    Json(String),
    /// A message arrived without the id needed to address it.
    MissingId,
}

impl fmt::Display for GmailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GmailError::Json(e) => write!(f, "gmail response is not usable JSON: {e}"),
            GmailError::MissingId => write!(f, "gmail message has no id"),
        }
    }
}

impl std::error::Error for GmailError {}

/// Seven days back from `today` (`YYYY-MM-DD`), as Gmail's `after:` clause.
pub fn query_for(today: &str) -> String {
    let after = match parse_date(today) {
        Some((mut y, mut m, mut d)) => {
            for _ in 0..7 {
                if d > 1 {
                    d -= 1;
                } else if m > 1 {
                    m -= 1;
                    d = days_in_month(y, m);
                } else {
                    y -= 1;
                    m = 12;
                    d = 31;
                }
            }
            format!("{y:04}/{m:02}/{d:02}")
        }
        // Asking `after:today` matches nothing, so a bad date fails closed
        // instead of silently widening the window.
        None => today.to_string(),
    };
    format!("after:{after} -in:spam -in:trash")
}

/// Parse a Gmail list response (`format=metadata`). Extra fields such as
/// `snippet` and `threadId` are ignored, never stored.
pub fn parse_list(json: &str) -> Result<Vec<Message>, GmailError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| GmailError::Json(e.to_string()))?;
    // serde would also accept `[]` as a struct; a list response is always an object.
    if !value.is_object() {
        return Err(GmailError::Json("list response is not an object".into()));
    }
    let list: ListResponse =
        serde_json::from_value(value).map_err(|e| GmailError::Json(e.to_string()))?;
    list.messages.into_iter().map(Message::try_from).collect()
}

/// Wait before the next try: exponential from 1 s, capped at 60 s.
pub fn backoff(attempt: u32) -> u64 {
    // Shift is capped so the multiply cannot overflow; the clamp does the rest.
    (1u64 << attempt.saturating_sub(1).min(6)).min(MAX_BACKOFF_SECS)
}

/// Whether another try is allowed. False from the third attempt on, so a 429
/// can never become a loop.
pub fn should_retry(attempt: u32) -> bool {
    attempt < MAX_ATTEMPTS
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        _ => 28,
    }
}

fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let mut parts = s.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
        return None;
    }
    Some((y, m, d))
}

#[derive(Deserialize)]
struct ListResponse {
    // No `messages` key means an empty window, not an error.
    #[serde(default)]
    messages: Vec<RawMessage>,
}

#[derive(Deserialize)]
struct RawMessage {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    payload: Option<RawPayload>,
}

#[derive(Deserialize)]
struct RawPayload {
    #[serde(default)]
    headers: Vec<RawHeader>,
}

#[derive(Deserialize)]
struct RawHeader {
    name: String,
    #[serde(default)]
    value: String,
}

impl TryFrom<RawMessage> for Message {
    type Error = GmailError;

    fn try_from(raw: RawMessage) -> Result<Self, GmailError> {
        let id = raw.id.ok_or(GmailError::MissingId)?;
        let headers = raw.payload.map(|p| p.headers).unwrap_or_default();
        let header = |want: &str| {
            headers
                .iter()
                .find(|h| h.name.eq_ignore_ascii_case(want))
                .map(|h| h.value.clone())
                .unwrap_or_default()
        };
        let subject = header("Subject");
        Ok(Message {
            id,
            subject: if subject.is_empty() {
                NO_SUBJECT.to_string()
            } else {
                subject
            },
            from: header("From"),
            date: header("Date"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = r#"{
      "messages": [
        {
          "id": "18c1",
          "threadId": "18c1",
          "snippet": "PRIVATE SNIPPET TEXT",
          "payload": { "mimeType": "multipart/alternative", "headers": [
            { "name": "Subject", "value": "Standup notes" },
            { "name": "From", "value": "Ada <ada@example.com>" },
            { "name": "Date", "value": "Mon, 26 Oct 2026 09:00:00 +0000" }
          ] }
        },
        {
          "id": "18c2",
          "snippet": "ANOTHER PRIVATE SNIPPET",
          "payload": { "headers": [
            { "name": "From", "value": "Bob <bob@example.com>" },
            { "name": "Date", "value": "Tue, 27 Oct 2026 10:00:00 +0000" }
          ] }
        }
      ]
    }"#;

    #[test]
    fn window_crosses_a_month_boundary() {
        assert_eq!(
            query_for("2026-03-03"),
            "after:2026/02/24 -in:spam -in:trash"
        );
        assert_eq!(
            query_for("2026-03-01"),
            "after:2026/02/22 -in:spam -in:trash"
        );
    }

    #[test]
    fn window_crosses_a_leap_february() {
        assert_eq!(
            query_for("2028-03-01"),
            "after:2028/02/23 -in:spam -in:trash"
        );
    }

    #[test]
    fn window_crosses_a_year_boundary() {
        assert_eq!(
            query_for("2026-01-03"),
            "after:2025/12/27 -in:spam -in:trash"
        );
        assert_eq!(
            query_for("2026-01-01"),
            "after:2025/12/25 -in:spam -in:trash"
        );
    }

    #[test]
    fn query_is_seven_days_and_skips_spam_and_trash() {
        let q = query_for("2026-10-31");
        assert!(q.starts_with("after:2026/10/24"), "{q}");
        assert!(q.contains("-in:spam"), "{q}");
        assert!(q.contains("-in:trash"), "{q}");
        assert!(!q.contains("before:"), "{q}");
    }

    #[test]
    fn only_three_metadata_headers_are_requested() {
        assert_eq!(METADATA_HEADERS, ["Subject", "From", "Date"]);
        assert_eq!(METADATA_HEADERS.len(), 3);
        for forbidden in ["snippet", "body", "payload", "To", "Cc"] {
            assert!(
                !METADATA_HEADERS
                    .iter()
                    .any(|h| h.eq_ignore_ascii_case(forbidden)),
                "{forbidden} must not be requested"
            );
        }
    }

    #[test]
    fn parses_metadata_into_messages() {
        let msgs = parse_list(LIST).expect("fixture parses");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].id, "18c1");
        assert_eq!(msgs[0].subject, "Standup notes");
        assert_eq!(msgs[0].from, "Ada <ada@example.com>");
        assert_eq!(msgs[0].date, "Mon, 26 Oct 2026 09:00:00 +0000");
    }

    #[test]
    fn missing_subject_defaults() {
        let msgs = parse_list(LIST).expect("fixture parses");
        assert_eq!(msgs[1].subject, "(no subject)");
        assert_eq!(msgs[1].from, "Bob <bob@example.com>");
    }

    #[test]
    fn snippet_is_never_kept() {
        let msgs = parse_list(LIST).expect("fixture parses");
        let dumped = format!("{msgs:?}");
        assert!(!dumped.contains("PRIVATE SNIPPET"), "{dumped}");
        assert!(!dumped.contains("multipart"), "{dumped}");
    }

    #[test]
    fn empty_day_is_not_an_error() {
        assert_eq!(parse_list(r#"{"resultSizeEstimate": 0}"#), Ok(vec![]));
    }

    #[test]
    fn malformed_json_errors() {
        assert!(matches!(parse_list("{not json"), Err(GmailError::Json(_))));
        assert!(matches!(parse_list("[]"), Err(GmailError::Json(_))));
        assert!(matches!(
            parse_list(r#"{"messages":[{"payload":{}}]}"#),
            Err(GmailError::MissingId)
        ));
    }

    #[test]
    fn retry_stops_after_three_attempts() {
        assert!(should_retry(1));
        assert!(should_retry(2));
        assert!(!should_retry(3));
        assert!(!should_retry(4));
        assert!(!should_retry(100));
    }

    #[test]
    fn backoff_grows_then_caps_at_sixty() {
        assert_eq!(backoff(1), 1);
        assert_eq!(backoff(2), 2);
        assert_eq!(backoff(3), 4);
        assert_eq!(backoff(6), 32);
        for attempt in 7..=1000 {
            assert!(backoff(attempt) <= MAX_BACKOFF_SECS);
        }
        assert_eq!(backoff(1000), 60);
    }
}
