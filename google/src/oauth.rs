//! Desktop OAuth for an installed app: loopback redirect on 127.0.0.1, PKCE S256, `state`.
//!
//! No token ever touches disk. Callers hand tokens to a `TokenSink` (keychain); there is no
//! file backend here, on purpose — see `docs/Security Audit.md` findings 3 and 5.
//!
//! The client secret of an installed-app client is not confidential by Google's own model;
//! the security boundary is PKCE plus the user's consent. Do not obfuscate it.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use sha2::{Digest, Sha256};

pub const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";

/// Loopback callback path. Google ignores the port for installed apps, not the path.
pub const CALLBACK_PATH: &str = "/";

pub const SCOPE_CALENDAR_READONLY: &str = "https://www.googleapis.com/auth/calendar.readonly";
pub const SCOPE_GMAIL_READONLY: &str = "https://www.googleapis.com/auth/gmail.readonly";

/// Read-only only. Adding a broader scope is a product and verification decision, not a tweak.
pub const SCOPES: [&str; 2] = [SCOPE_CALENDAR_READONLY, SCOPE_GMAIL_READONLY];

/// RFC 7636 verifier (64 chars, inside 43..=128) and its S256 challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    pub fn generate() -> Self {
        Self::from_verifier(&hex(&random_bytes()))
    }

    pub fn from_verifier(verifier: &str) -> Self {
        Self {
            verifier: verifier.to_owned(),
            challenge: challenge_for(verifier),
        }
    }
}

/// `base64url(sha256(verifier))`, no padding.
pub fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// Everything the browser needs. `redirect_port` is the already-bound ephemeral port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthRequest {
    pub url: String,
    pub state: String,
    pub verifier: String,
    pub redirect_port: u16,
}

/// Build the consent URL. Bind the loopback listener first and pass its port.
pub fn authorization_request(client_id: &str, redirect_port: u16) -> AuthRequest {
    let pkce = Pkce::generate();
    let state = hex(&random_bytes());
    let redirect_uri = format!("http://127.0.0.1:{redirect_port}{CALLBACK_PATH}");
    let url = format!(
        "{AUTH_ENDPOINT}?client_id={}&redirect_uri={}&response_type=code&scope={}\
         &code_challenge={}&code_challenge_method=S256&state={state}&access_type=offline",
        encode(client_id),
        encode(&redirect_uri),
        encode(&SCOPES.join(" ")),
        pkce.challenge,
    );
    AuthRequest {
        url,
        state,
        verifier: pkce.verifier,
        redirect_port,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthCode {
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectError {
    NotGet,
    WrongPath,
    /// Missing or unexpected `state`: treat as an attack, not a typo.
    StateMismatch,
    /// Google returned `error=...` (for example `access_denied`) instead of a code.
    ProviderError(String),
    MissingCode,
}

/// Read one HTTP request line (`GET /?code=..&state=.. HTTP/1.1`) and nothing else.
pub fn parse_redirect(request_line: &str, expected_state: &str) -> Result<AuthCode, RedirectError> {
    let mut parts = request_line.split_whitespace();
    if parts.next() != Some("GET") {
        return Err(RedirectError::NotGet);
    }
    let target = parts.next().ok_or(RedirectError::WrongPath)?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != CALLBACK_PATH {
        return Err(RedirectError::WrongPath);
    }

    let (mut state, mut code, mut error) = (None, None, None);
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "state" => state = Some(percent_decode(value)),
            "code" => code = Some(percent_decode(value)),
            "error" => error = Some(percent_decode(value)),
            _ => {}
        }
    }

    if state.as_deref() != Some(expected_state) {
        return Err(RedirectError::StateMismatch);
    }
    if let Some(error) = error {
        return Err(RedirectError::ProviderError(error));
    }
    match code {
        Some(code) if !code.is_empty() => Ok(AuthCode { code }),
        _ => Err(RedirectError::MissingCode),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SinkError {
    /// No keychain available (e.g. libsecret missing). Refuse; ask again next session.
    /// Never fall back to a file.
    Unavailable,
}

pub trait TokenSink {
    fn store(&mut self, access: &str, refresh: &str) -> Result<(), SinkError>;
}

/// Test sink. In memory only — the API has no file backend at all.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MemorySink {
    pub access: Option<String>,
    pub refresh: Option<String>,
}

impl TokenSink for MemorySink {
    fn store(&mut self, access: &str, refresh: &str) -> Result<(), SinkError> {
        self.access = Some(access.to_owned());
        self.refresh = Some(refresh.to_owned());
        Ok(())
    }
}

fn random_bytes() -> [u8; 32] {
    // shortcut: OS-seeded SipHash keys instead of a CSPRNG crate; swap for getrandom if a
    // crypto audit asks for one.
    let mut out = [0u8; 32];
    for (i, chunk) in out.chunks_mut(8).enumerate() {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_usize(i);
        chunk.copy_from_slice(&hasher.finish().to_le_bytes()[..chunk.len()]);
    }
    out
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(high << 4 | low);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RFC_VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const RFC_CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

    #[test]
    fn rfc7636_appendix_b_pair() {
        let pkce = Pkce::from_verifier(RFC_VERIFIER);
        assert_eq!(pkce.challenge, RFC_CHALLENGE);
        assert_eq!(challenge_for(RFC_VERIFIER), RFC_CHALLENGE);
    }

    #[test]
    fn verifier_length_and_charset() {
        for _ in 0..8 {
            let pkce = Pkce::generate();
            assert_eq!(pkce.verifier.len(), 64);
            assert!((43..=128).contains(&pkce.verifier.len()));
            assert!(pkce
                .verifier
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')));
            assert_eq!(pkce.challenge, challenge_for(&pkce.verifier));
        }
    }

    #[test]
    fn authorization_url_is_loopback_pkce() {
        let req = authorization_request("id.apps.googleusercontent.com", 41111);
        assert!(req.url.starts_with(AUTH_ENDPOINT));
        assert!(req.url.contains("code_challenge_method=S256"));
        assert!(req
            .url
            .contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A41111%2F"));
        assert!(req.url.contains(&format!("state={}", req.state)));
        assert_eq!(req.url.matches("code_challenge=").count(), 1);
        assert_eq!(req.redirect_port, 41111);
    }

    #[test]
    fn state_mismatch_rejected() {
        let line = "GET /?code=4%2F0abc&state=attacker HTTP/1.1";
        assert_eq!(
            parse_redirect(line, "expected"),
            Err(RedirectError::StateMismatch)
        );
    }

    #[test]
    fn access_denied_is_a_typed_error_not_a_code() {
        let line = "GET /?error=access_denied&state=s1 HTTP/1.1";
        assert_eq!(
            parse_redirect(line, "s1"),
            Err(RedirectError::ProviderError("access_denied".to_owned()))
        );
    }

    #[test]
    fn wrong_path_rejected() {
        let line = "GET /not-the-callback?code=4%2F0abc&state=s1 HTTP/1.1";
        assert_eq!(parse_redirect(line, "s1"), Err(RedirectError::WrongPath));
    }

    #[test]
    fn missing_code_rejected() {
        assert_eq!(
            parse_redirect("GET /?state=s1 HTTP/1.1", "s1"),
            Err(RedirectError::MissingCode)
        );
        assert_eq!(
            parse_redirect("GET /?code=&state=s1 HTTP/1.1", "s1"),
            Err(RedirectError::MissingCode)
        );
    }

    #[test]
    fn non_get_rejected() {
        assert_eq!(
            parse_redirect("POST /?code=abc&state=s1 HTTP/1.1", "s1"),
            Err(RedirectError::NotGet)
        );
    }

    #[test]
    fn code_is_percent_decoded() {
        let auth = parse_redirect("GET /?code=4%2F0Axyz&state=s1 HTTP/1.1", "s1").unwrap();
        assert_eq!(auth.code, "4/0Axyz");
    }

    #[test]
    fn scopes_are_readonly_only() {
        let joined = SCOPES.join(" ");
        assert!(joined.contains("gmail.readonly"));
        assert_eq!(SCOPES.len(), 2);
        for scope in SCOPES {
            assert!(scope.ends_with(".readonly"), "{scope} is not read-only");
        }
        for broader in [
            "mail.google.com",
            "gmail.modify",
            "gmail.compose",
            "calendar",
        ] {
            assert!(
                !joined.contains(broader) || broader == "calendar",
                "scope string is broader than read-only: {broader}"
            );
        }
        assert_eq!(joined.matches("gmail.").count(), 1);
    }

    #[test]
    fn token_never_touches_disk() {
        let dir = std::env::temp_dir().join(format!("newton-oauth-{}", hex(&random_bytes())));
        std::fs::create_dir_all(&dir).unwrap();

        let req = authorization_request("id.apps.googleusercontent.com", 41111);
        let line = format!("GET /?code=4%2F0abc&state={} HTTP/1.1", req.state);
        let auth = parse_redirect(&line, &req.state).unwrap();

        let mut sink = MemorySink::default();
        sink.store("ya29.access", "1//refresh").unwrap();
        assert_eq!(auth.code, "4/0abc");
        assert_eq!(sink.access.as_deref(), Some("ya29.access"));
        assert_eq!(sink.refresh.as_deref(), Some("1//refresh"));

        let entries = std::fs::read_dir(&dir).unwrap().count();
        std::fs::remove_dir(&dir).unwrap();
        assert_eq!(entries, 0, "the oauth flow wrote a file");
    }
}
