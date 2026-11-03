//! Helper process handshake.
//!
//! The token is a shared secret derived from a seed both sides already have.
//! It is not a signature: it proves "same seed", not who signed anything.
//! Comparison is constant time and a token can be accepted once.

use std::cell::Cell;

const LEN: usize = 32;

/// One derivation of the shared secret, plus the fact that it was used.
pub struct Handshake {
    token: [u8; LEN],
    used: Cell<bool>,
}

impl Handshake {
    /// Deterministic token from a seed (splitmix64). Not a signature.
    pub fn new(seed: u64) -> Self {
        let mut state = seed;
        let mut token = [0u8; LEN];
        for chunk in token.chunks_mut(8) {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            // chunks_mut(8) over 32 bytes always yields 8-byte chunks.
            chunk.copy_from_slice(&z.to_le_bytes());
        }
        Self {
            token,
            used: Cell::new(false),
        }
    }

    pub fn token(&self) -> [u8; LEN] {
        self.token
    }

    /// A one-shot acceptor sharing this handshake's used flag.
    pub fn acceptor(&self) -> Accept<'_> {
        Accept { handshake: self }
    }
}

/// A view that can check a presented token once.
pub struct Accept<'a> {
    handshake: &'a Handshake,
}

impl Accept<'_> {
    /// Constant-time compare, full length always walked. True at most once.
    pub fn verify(&self, presented: &[u8]) -> bool {
        if self.handshake.used.get() || !matches(&self.handshake.token, presented) {
            return false;
        }
        self.handshake.used.set(true);
        true
    }
}

/// A helper asking to be trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelperRequest {
    pub pid: u32,
    pub token: [u8; LEN],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperError {
    BadToken,
    Replay,
    Missing,
}

/// Accept a helper's token once. `pid == 0` means no helper is there.
pub fn accept_request(h: &Handshake, r: &HelperRequest) -> Result<(), HelperError> {
    if r.pid == 0 {
        return Err(HelperError::Missing);
    }
    if !matches(&h.token, &r.token) {
        return Err(HelperError::BadToken);
    }
    if h.used.replace(true) {
        return Err(HelperError::Replay);
    }
    Ok(())
}

/// XOR accumulate over the whole length; no early return on a bad byte.
fn matches(expected: &[u8], presented: &[u8]) -> bool {
    if presented.len() != expected.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in expected.iter().zip(presented) {
        diff |= a ^ b;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(h: &Handshake) -> HelperRequest {
        HelperRequest {
            pid: 42,
            token: h.token(),
        }
    }

    #[test]
    fn accepts_the_right_token_once() {
        let h = Handshake::new(7);
        assert_eq!(accept_request(&h, &request(&h)), Ok(()));
        assert!(!h.acceptor().verify(&h.token()));
    }

    #[test]
    fn refuses_a_replayed_token() {
        let h = Handshake::new(7);
        let r = request(&h);
        assert_eq!(accept_request(&h, &r), Ok(()));
        assert_eq!(accept_request(&h, &r), Err(HelperError::Replay));
    }

    #[test]
    fn refuses_a_wrong_token() {
        let h = Handshake::new(7);
        let mut r = request(&h);
        r.token[0] ^= 1;
        assert_eq!(accept_request(&h, &r), Err(HelperError::BadToken));
        // A refusal does not burn the real token.
        assert_eq!(accept_request(&h, &request(&h)), Ok(()));
    }

    #[test]
    fn refuses_a_wrong_length() {
        let h = Handshake::new(7);
        assert!(!h.acceptor().verify(&h.token()[..31]));
        assert!(!h.acceptor().verify(&[0u8; 33]));
    }

    #[test]
    fn refuses_an_empty_token() {
        let h = Handshake::new(7);
        assert!(!h.acceptor().verify(&[]));
    }

    #[test]
    fn different_seeds_do_not_share_a_token() {
        assert_ne!(Handshake::new(1).token(), Handshake::new(2).token());
        assert_eq!(Handshake::new(1).token(), Handshake::new(1).token());
    }

    // Wrong at the last byte: the compare must reach the end, and failing there
    // must not consume the token.
    #[test]
    fn compares_the_full_length() {
        let h = Handshake::new(7);
        let mut wrong = h.token();
        wrong[LEN - 1] ^= 0xFF;
        assert!(!h.acceptor().verify(&wrong));
        assert!(h.acceptor().verify(&h.token()));
    }

    #[test]
    fn refuses_a_missing_helper() {
        let h = Handshake::new(7);
        let mut r = request(&h);
        r.pid = 0;
        assert_eq!(accept_request(&h, &r), Err(HelperError::Missing));
    }
}
