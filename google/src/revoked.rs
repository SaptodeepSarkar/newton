//! Revocation and degraded mode.
//!
//! The recovery path users actually hit. Two rules from the PRD decide everything
//! here: lost access is said out loud, and Newton never guesses an answer it could
//! not fetch.

/// What Newton may do with Google after a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Google answered, so Google is usable.
    Live,
    /// The user took access away. Say so; do not retry.
    Revoked,
    /// The token aged out but consent still stands; the caller refreshes it.
    Expired,
    /// No usable answer from Google. Local work continues.
    Offline,
}

impl Access {
    /// Map a Google HTTP status and body to what Newton may do next.
    ///
    /// A 403 is quota, not consent: it stays `Live` and backing off is the
    /// caller's job. Anything unrecognised (a 500, a proxy page) is `Offline`
    /// because a failed request is never permission to answer from memory.
    pub fn from_http(status: u16, body: &str) -> Access {
        match status {
            401 if body.contains("invalid_grant") => Access::Revoked,
            401 => Access::Expired,
            200..=399 | 403 | 429 => Access::Live,
            _ => Access::Offline,
        }
    }
}

/// One thing Newton can do, and whether it can do it right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capability {
    pub name: &'static str,
    pub available: bool,
}

/// What still works. Memory and talk never depend on Google, which is the whole
/// point of a local assistant.
pub fn capabilities(access: Access) -> Vec<Capability> {
    let google = access == Access::Live;
    vec![
        Capability {
            name: "calendar.read",
            available: google,
        },
        Capability {
            name: "gmail.search",
            available: google,
        },
        Capability {
            name: "memory.search",
            available: true,
        },
        Capability {
            name: "talk",
            available: true,
        },
    ]
}

/// The sentence Newton speaks instead of an answer it does not have.
///
/// `Expired` is silent on purpose: the caller refreshes it, and a refresh that
/// fails comes back as `Revoked`.
pub fn spoken_notice(access: Access) -> Option<&'static str> {
    match access {
        Access::Live | Access::Expired => None,
        Access::Revoked => Some(
            "I lost access to your Google account, so I can't check Gmail or your calendar; I can still use your notes.",
        ),
        Access::Offline => Some(
            "I can't reach Google right now because I'm offline; I can still use your notes.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_map_to_access() {
        assert_eq!(Access::from_http(200, "{}"), Access::Live);
        assert_eq!(
            Access::from_http(401, r#"{"error":"invalid_grant"}"#),
            Access::Revoked
        );
        assert_eq!(
            Access::from_http(401, r#"{"error":"unauthorized"}"#),
            Access::Expired
        );
        assert_eq!(Access::from_http(429, "quota"), Access::Live);
        assert_eq!(Access::from_http(500, "oops"), Access::Offline);
    }

    #[test]
    fn rate_limit_is_not_revocation() {
        let access = Access::from_http(
            403,
            r#"{"error":{"code":403,"message":"Rate Limit Exceeded"}}"#,
        );
        assert_eq!(access, Access::Live);
        assert!(capabilities(access).iter().all(|c| c.available));
    }

    #[test]
    fn unknown_failure_is_not_consent_to_guess() {
        let access = Access::from_http(502, "<html>bad gateway</html>");
        assert_eq!(access, Access::Offline);
        assert!(spoken_notice(access).is_some());
    }

    #[test]
    fn revoked_keeps_local_work() {
        let access = Access::from_http(401, "invalid_grant");
        assert_eq!(access, Access::Revoked);
        let caps = capabilities(access);
        let available = |name: &str| caps.iter().find(|c| c.name == name).unwrap().available;
        assert!(!available("gmail.search"));
        assert!(!available("calendar.read"));
        assert!(available("memory.search"));
        assert!(available("talk"));
    }

    #[test]
    fn notice_is_present_exactly_for_lost_access() {
        for access in [
            Access::Live,
            Access::Revoked,
            Access::Expired,
            Access::Offline,
        ] {
            let has_notice = spoken_notice(access).is_some();
            assert_eq!(
                has_notice,
                matches!(access, Access::Revoked | Access::Offline),
                "{access:?}"
            );
        }
        assert!(spoken_notice(Access::Revoked)
            .unwrap()
            .contains("lost access"));
    }
}
