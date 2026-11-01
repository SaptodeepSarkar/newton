//! Read a Google Calendar `events.list` response into what Newton can say.
//!
//! Times are kept exactly as Google returned them (ISO 8601 with offset).
//! No timezone maths happens here: converting to the user's clock needs the
//! calendar's time zone and a tz database, which is owed in a later phase.

use serde::Deserialize;

/// One calendar day, already narrowed to what the orb needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Day {
    /// The day this was asked for, as `YYYY-MM-DD`. Echoed back, not computed.
    pub date: String,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Google's ISO string, or `YYYY-MM-DD` for an all-day event.
    pub start: String,
    pub end: String,
    pub title: String,
    pub all_day: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The response was not JSON we could read. `offset` is the byte offset in
    /// the input when it can be located.
    Malformed { offset: Option<usize> },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Malformed { offset: Some(at) } => {
                write!(f, "malformed calendar JSON at byte {at}")
            }
            Error::Malformed { offset: None } => write!(f, "malformed calendar JSON"),
        }
    }
}

impl std::error::Error for Error {}

/// Parse one day's `events.list` body.
///
/// A day with no events is `Ok` with an empty list. An event with no start or
/// no end is skipped on its own; it never fails the day.
pub fn parse_day(date: &str, json: &str) -> Result<Day, Error> {
    let wire: Wire = serde_json::from_str(json).map_err(|e| Error::Malformed {
        offset: byte_offset(json, &e),
    })?;

    let events = wire
        .items
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.status.as_deref() != Some("cancelled"))
        .filter_map(|e| {
            let start = e.start?.iso()?;
            let end = e.end?.iso()?;
            Some(Event {
                all_day: start.len() == 10,
                start,
                end,
                title: e.summary.unwrap_or_else(|| "busy".to_string()),
            })
        })
        .collect();

    Ok(Day {
        date: date.to_string(),
        events,
    })
}

/// A short sentence for the orb: at most three events, then a count of the rest.
pub fn speakable(day: &Day) -> String {
    const SHOWN: usize = 3;
    let n = day.events.len();
    if n == 0 {
        return "Nothing on the calendar today.".to_string();
    }

    let spoken: Vec<String> = day
        .events
        .iter()
        .take(SHOWN)
        .map(|e| {
            if e.all_day {
                format!("{} all day", e.title)
            } else {
                format!("{} at {}", e.title, hhmm(&e.start))
            }
        })
        .collect();

    let noun = if n == 1 { "thing" } else { "things" };
    let mut said = format!(
        "{} {noun} today: {}",
        capitalize(&word(n)),
        spoken.join(", ")
    );
    if n > SHOWN {
        said.push_str(&format!(" and {} more", word(n - SHOWN)));
    }
    said.push('.');
    said
}

// serde_json exposes line/column only, so rebuild the byte offset from them.
fn byte_offset(input: &str, err: &serde_json::Error) -> Option<usize> {
    let line_start: usize = input
        .split_inclusive('\n')
        .take(err.line().saturating_sub(1))
        .map(str::len)
        .sum();
    let at = line_start + err.column();
    (at <= input.len()).then_some(at)
}

// `2026-06-01T09:30:00+02:00` -> `09:30`. Anything else is shown as it came.
fn hhmm(iso: &str) -> &str {
    match iso.get(10..11) {
        Some("T") => iso.get(11..16).unwrap_or(iso),
        _ => iso,
    }
}

fn word(n: usize) -> String {
    const WORDS: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    WORDS.get(n).copied().unwrap_or("many").to_string()
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[derive(Deserialize)]
struct Wire {
    items: Option<Vec<WireEvent>>,
}

#[derive(Deserialize)]
struct WireEvent {
    summary: Option<String>,
    status: Option<String>,
    start: Option<WireTime>,
    end: Option<WireTime>,
}

#[derive(Deserialize)]
struct WireTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

impl WireTime {
    fn iso(&self) -> Option<String> {
        self.date_time.clone().or_else(|| self.date.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: &str = "2026-06-01";

    fn day(json: &str) -> Day {
        parse_day(DAY, json).expect("parse")
    }

    #[test]
    fn a_fixture_day_renders_three_timed_events_and_one_all_day() {
        let json = r#"{
            "kind": "calendar#events",
            "summary": "work",
            "timeZone": "Europe/Berlin",
            "items": [
                {"summary": "standup", "start": {"dateTime": "2026-06-01T09:30:00+02:00"},
                 "end": {"dateTime": "2026-06-01T09:45:00+02:00"}},
                {"summary": "review", "start": {"dateTime": "2026-06-01T13:00:00+02:00"},
                 "end": {"dateTime": "2026-06-01T14:00:00+02:00"}},
                {"summary": "dinner", "start": {"dateTime": "2026-06-01T19:00:00+02:00"},
                 "end": {"dateTime": "2026-06-01T21:00:00+02:00"}},
                {"summary": "holiday", "start": {"date": "2026-06-01"}, "end": {"date": "2026-06-02"}}
            ]
        }"#;

        let d = day(json);
        assert_eq!(d.date, DAY);
        assert_eq!(d.events.len(), 4);
        assert_eq!(d.events[0].title, "standup");
        assert_eq!(d.events[0].start, "2026-06-01T09:30:00+02:00");
        assert_eq!(d.events[0].end, "2026-06-01T09:45:00+02:00");
        assert!(!d.events[0].all_day);
        assert!(d.events[3].all_day);
        assert_eq!(d.events[3].start, "2026-06-01");
        assert_eq!(
            speakable(&d),
            "Four things today: standup at 09:30, review at 13:00, dinner at 19:00 and one more."
        );
    }

    #[test]
    fn cancelled_events_are_skipped() {
        let json = r#"{"items": [
            {"summary": "gone", "status": "cancelled",
             "start": {"dateTime": "2026-06-01T09:00:00Z"}, "end": {"dateTime": "2026-06-01T10:00:00Z"}},
            {"summary": "kept", "status": "confirmed",
             "start": {"dateTime": "2026-06-01T11:00:00Z"}, "end": {"dateTime": "2026-06-01T12:00:00Z"}}
        ]}"#;

        let d = day(json);
        assert_eq!(d.events.len(), 1);
        assert_eq!(d.events[0].title, "kept");
    }

    #[test]
    fn an_empty_day_is_ok_and_says_nothing_is_on() {
        for json in ["{}", r#"{"items": []}"#] {
            let d = day(json);
            assert!(d.events.is_empty());
            assert_eq!(d.date, DAY);
            assert_eq!(speakable(&d), "Nothing on the calendar today.");
        }
    }

    #[test]
    fn malformed_json_is_an_error_with_an_offset() {
        let err = parse_day(DAY, r#"{"items": [}"#).unwrap_err();
        match err {
            Error::Malformed { offset } => assert!(
                offset.is_some_and(|at| at <= r#"{"items": [}"#.len()),
                "offset should point inside the input"
            ),
        }
    }

    #[test]
    fn an_event_missing_its_end_is_skipped() {
        let json = r#"{"items": [
            {"summary": "half", "start": {"dateTime": "2026-06-01T09:00:00Z"}},
            {"summary": "whole", "start": {"dateTime": "2026-06-01T10:00:00Z"},
             "end": {"dateTime": "2026-06-01T11:00:00Z"}}
        ]}"#;

        let d = day(json);
        assert_eq!(d.events.len(), 1);
        assert_eq!(d.events[0].title, "whole");
    }

    #[test]
    fn eight_events_speak_three_then_a_count() {
        let items: Vec<String> = (0..8)
            .map(|i| {
                format!(
                    r#"{{"summary": "e{i}", "start": {{"dateTime": "2026-06-01T0{i}:00:00Z"}},
                        "end": {{"dateTime": "2026-06-01T0{i}:30:00Z"}}}}"#
                )
            })
            .collect();
        let json = format!(r#"{{"items": [{}]}}"#, items.join(","));

        assert_eq!(
            speakable(&day(&json)),
            "Eight things today: e0 at 00:00, e1 at 01:00, e2 at 02:00 and five more."
        );
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let json = r#"{"nextPageToken": "x", "items": [{
            "id": "abc", "etag": "\"1\"", "htmlLink": "https://example.com",
            "attendees": [{"email": "a@b.c", "responseStatus": "accepted"}],
            "organizer": {"email": "o@b.c"},
            "recurrence": ["RRULE:FREQ=DAILY"],
            "start": {"dateTime": "2026-06-01T09:30:00+02:00", "timeZone": "Europe/Berlin"},
            "end": {"dateTime": "2026-06-01T09:45:00+02:00", "timeZone": "Europe/Berlin"},
            "summary": "standup"
        }]}"#;

        let d = day(json);
        assert_eq!(d.events.len(), 1);
        assert_eq!(d.events[0].title, "standup");
        assert_eq!(d.events[0].start, "2026-06-01T09:30:00+02:00");
    }
}
