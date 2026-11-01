//! GGUF model files: just enough of the format to refuse a bad one, plus the
//! runtime boundary that lets the app run with no weights at all.

use std::fmt;

/// Every GGUF file starts with these four bytes.
pub const MAGIC: [u8; 4] = *b"GGUF";
/// magic + version + tensor count + metadata kv count.
const HEADER_LEN: usize = 4 + 4 + 8 + 8;
/// Smallest tensor-info record: empty name, zero dims, type, offset.
const MIN_TENSOR_INFO: u64 = 8 + 4 + 4 + 8;
/// Smallest metadata record: empty key, value type, one-byte value.
const MIN_METADATA_KV: u64 = 8 + 4 + 1;

/// The fixed-size head of a GGUF file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GgufHeader {
    pub version: u32,
    pub tensor_count: u64,
    pub metadata_count: u64,
}

/// Why a file is not a GGUF this build can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgufError {
    BadMagic,
    UnsupportedVersion(u32),
    Truncated,
    AbsurdCounts,
}

impl fmt::Display for GgufError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "not a GGUF file"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported GGUF version {v}"),
            Self::Truncated => write!(f, "file ends inside the GGUF header"),
            Self::AbsurdCounts => write!(f, "GGUF counts do not fit in the file"),
        }
    }
}

impl std::error::Error for GgufError {}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, GgufError> {
    let b = bytes.get(at..at + 4).ok_or(GgufError::Truncated)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn u64_at(bytes: &[u8], at: usize) -> Result<u64, GgufError> {
    let b = bytes.get(at..at + 8).ok_or(GgufError::Truncated)?;
    let mut word = [0u8; 8];
    word.copy_from_slice(b);
    Ok(u64::from_le_bytes(word))
}

/// Read the header and reject anything that cannot be a real file.
///
/// Version 1 is refused because its layout differs; everything above 3 is
/// newer than this build knows. Counts are checked against the bytes actually
/// left, so a corrupt count is refused before anything allocates on it.
pub fn read_header(bytes: &[u8]) -> Result<GgufHeader, GgufError> {
    if bytes.get(..4).ok_or(GgufError::Truncated)? != MAGIC {
        return Err(GgufError::BadMagic);
    }

    let version = u32_at(bytes, 4)?;
    if !matches!(version, 2 | 3) {
        return Err(GgufError::UnsupportedVersion(version));
    }

    let tensor_count = u64_at(bytes, 8)?;
    let metadata_count = u64_at(bytes, 16)?;

    let remaining = (bytes.len() - HEADER_LEN) as u64;
    let needed = tensor_count
        .saturating_mul(MIN_TENSOR_INFO)
        .saturating_add(metadata_count.saturating_mul(MIN_METADATA_KV));
    if needed > remaining {
        return Err(GgufError::AbsurdCounts);
    }

    Ok(GgufHeader {
        version,
        tensor_count,
        metadata_count,
    })
}

/// A loaded model, or a placeholder that stands in for one.
pub trait ModelRuntime {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
}

/// The honest answer when no weights are installed: the turn still runs, and
/// the app says the model is missing instead of crashing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MissingModel;

impl ModelRuntime for MissingModel {
    fn name(&self) -> &str {
        "missing"
    }

    fn is_available(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A header followed by `body` zero bytes, so the counts can be honest.
    fn fixture(version: u32, tensors: u64, metadata: u64, body: usize) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MAGIC);
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&tensors.to_le_bytes());
        bytes.extend_from_slice(&metadata.to_le_bytes());
        bytes.resize(bytes.len() + body, 0);
        bytes
    }

    #[test]
    fn reads_a_version_2_header() {
        let bytes = fixture(
            2,
            1,
            2,
            MIN_TENSOR_INFO as usize + 2 * MIN_METADATA_KV as usize,
        );
        assert_eq!(
            read_header(&bytes),
            Ok(GgufHeader {
                version: 2,
                tensor_count: 1,
                metadata_count: 2,
            })
        );
    }

    #[test]
    fn reads_a_version_3_header() {
        let bytes = fixture(
            3,
            291,
            24,
            (291 * MIN_TENSOR_INFO + 24 * MIN_METADATA_KV) as usize,
        );
        assert_eq!(
            read_header(&bytes),
            Ok(GgufHeader {
                version: 3,
                tensor_count: 291,
                metadata_count: 24,
            })
        );
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = fixture(3, 0, 0, 0);
        bytes[0] = b'G';
        bytes[3] = b'X';
        assert_eq!(read_header(&bytes), Err(GgufError::BadMagic));

        assert_eq!(read_header(b"gguf"), Err(GgufError::BadMagic));
    }

    #[test]
    fn rejects_a_truncated_header_at_every_boundary() {
        // Every cutoff inside the 24-byte header is a truncation, never a panic.
        let full = fixture(3, 0, 0, 0);
        for cut in 0..HEADER_LEN {
            assert_eq!(
                read_header(&full[..cut]),
                Err(GgufError::Truncated),
                "{cut}"
            );
        }
        assert!(read_header(&full).is_ok());
    }

    #[test]
    fn rejects_a_truncated_nonzero_header() {
        // A real header cut in the middle of its counts is still truncated.
        let full = fixture(3, 1, 1, (MIN_TENSOR_INFO + MIN_METADATA_KV) as usize);
        for cut in [4, 8, 15, 16, 23] {
            assert_eq!(read_header(&full[..cut]), Err(GgufError::Truncated));
        }
    }

    #[test]
    fn rejects_old_and_future_versions() {
        assert_eq!(
            read_header(&fixture(1, 0, 0, 0)),
            Err(GgufError::UnsupportedVersion(1))
        );
        assert_eq!(
            read_header(&fixture(99, 0, 0, 0)),
            Err(GgufError::UnsupportedVersion(99))
        );
    }

    #[test]
    fn rejects_absurd_counts() {
        assert_eq!(
            read_header(&fixture(3, u64::MAX, 0, 0)),
            Err(GgufError::AbsurdCounts)
        );
        assert_eq!(
            read_header(&fixture(3, 0, u64::MAX, 0)),
            Err(GgufError::AbsurdCounts)
        );
        // One byte short of the minimum the counts demand is still absurd.
        assert_eq!(
            read_header(&fixture(
                3,
                1,
                1,
                (MIN_TENSOR_INFO + MIN_METADATA_KV - 1) as usize
            )),
            Err(GgufError::AbsurdCounts)
        );
    }

    #[test]
    fn accepts_the_smallest_honest_body() {
        let bytes = fixture(3, 1, 1, (MIN_TENSOR_INFO + MIN_METADATA_KV) as usize);
        assert!(read_header(&bytes).is_ok());
    }

    #[test]
    fn missing_model_reports_unavailable() {
        let model = MissingModel;
        assert!(!model.is_available());
        assert_eq!(model.name(), "missing");
    }
}
