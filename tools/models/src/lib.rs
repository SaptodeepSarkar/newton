//! Newton model staging: read a versioned manifest, check every file against
//! its SHA-256 and size, and land it in the model directory only when it is
//! whole.
//!
//! No network code lives here. A [`Source`] does the download so tests can run
//! against memory; the installer supplies the HTTP implementation.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

/// Where bytes come from. One call per manifest entry, written to `into`.
pub trait Source {
    fn fetch(&self, url: &str, into: &mut dyn Write) -> Result<(), FetchError>;
}

/// The downloader until a release pipeline publishes the files. It names the
/// missing piece instead of pretending a network call might work.
pub struct DefaultSource;

impl Source for DefaultSource {
    fn fetch(&self, url: &str, _into: &mut dyn Write) -> Result<(), FetchError> {
        Err(FetchError::new(format!(
            "release pipeline not wired yet: nothing publishes {url}; pass --source <dir> to install from a local copy"
        )))
    }
}

/// Reads model files from a local directory, matched by the URL's file name.
/// This is the offline USB install and what the tests use.
pub struct DirSource {
    dir: PathBuf,
}

impl DirSource {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }
}

impl Source for DirSource {
    fn fetch(&self, url: &str, into: &mut dyn Write) -> Result<(), FetchError> {
        let name = Path::new(url)
            .file_name()
            .ok_or_else(|| FetchError::new(format!("cannot tell a file name from {url}")))?;
        let path = self.dir.join(name);
        let mut file = fs::File::open(&path)
            .map_err(|e| FetchError::new(format!("cannot read {}: {e}", path.display())))?;
        io::copy(&mut file, into).map_err(|e| FetchError::new(e.to_string()))?;
        Ok(())
    }
}

/// A fetch that did not complete. The message is for the log, not the user.
#[derive(Debug)]
pub struct FetchError(String);

impl FetchError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fetch failed: {}", self.0)
    }
}

impl std::error::Error for FetchError {}

/// A parsed manifest, as published beside the release.
#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub files: Vec<FileEntry>,
}

/// One downloadable file: where it lives, and what it must hash to.
#[derive(Debug, Clone, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
}

/// A manifest that cannot be trusted as written.
#[derive(Debug)]
pub enum ManifestError {
    Json(serde_json::Error),
    Duplicate(String),
    UnsafeName(String),
    BadChecksum(String),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(e) => write!(f, "manifest is not valid JSON: {e}"),
            Self::Duplicate(name) => write!(f, "manifest lists {name:?} twice"),
            Self::UnsafeName(name) => {
                write!(f, "manifest name {name:?} is not a plain file name")
            }
            Self::BadChecksum(name) => {
                write!(f, "manifest entry {name:?} needs a 64-hex sha256")
            }
        }
    }
}

impl std::error::Error for ManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}

/// A file on disk that is not what the manifest promised.
#[derive(Debug)]
pub enum VerifyError {
    Io(io::Error),
    Size { expected: u64, actual: u64 },
    Checksum { expected: String, actual: String },
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "cannot read model file: {e}"),
            Self::Size { expected, actual } => {
                write!(f, "model file is {actual} bytes, manifest says {expected}")
            }
            Self::Checksum { expected, actual } => {
                write!(f, "model file hashes to {actual}, manifest says {expected}")
            }
        }
    }
}

impl std::error::Error for VerifyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// Why staging did not finish. Nothing is left half-written on any path.
#[derive(Debug)]
pub enum StageError {
    Offline,
    UnsafeName(String),
    Io(io::Error),
    Fetch(FetchError),
    Verify(VerifyError),
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offline => write!(f, "offline: models are missing and no download was tried"),
            Self::UnsafeName(name) => {
                write!(f, "refusing to stage {name:?} outside the model directory")
            }
            Self::Io(e) => write!(f, "cannot stage model file: {e}"),
            Self::Fetch(e) => write!(f, "{e}"),
            Self::Verify(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Fetch(e) => Some(e),
            Self::Verify(e) => Some(e),
            _ => None,
        }
    }
}

/// Parse a manifest and reject anything that could escape the model directory
/// or collide with another entry.
pub fn parse_manifest(json: &str) -> Result<Manifest, ManifestError> {
    let manifest: Manifest = serde_json::from_str(json).map_err(ManifestError::Json)?;
    {
        let mut seen = HashSet::new();
        for file in &manifest.files {
            if !is_plain_name(&file.name) {
                return Err(ManifestError::UnsafeName(file.name.clone()));
            }
            if !is_sha256(&file.sha256) {
                return Err(ManifestError::BadChecksum(file.name.clone()));
            }
            if !seen.insert(file.name.as_str()) {
                return Err(ManifestError::Duplicate(file.name.clone()));
            }
        }
    }
    Ok(manifest)
}

/// Check a file against the manifest. Size is checked first so a truncated
/// download is reported as truncated, not as a bad hash.
pub fn verify_file(
    path: &Path,
    expected_sha256: &str,
    expected_bytes: u64,
) -> Result<(), VerifyError> {
    let mut file = fs::File::open(path).map_err(VerifyError::Io)?;
    let mut hasher = Sha256::new();
    let actual_bytes = io::copy(&mut file, &mut hasher).map_err(VerifyError::Io)?;
    if actual_bytes != expected_bytes {
        return Err(VerifyError::Size {
            expected: expected_bytes,
            actual: actual_bytes,
        });
    }
    let actual = hex(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(VerifyError::Checksum {
            expected: expected_sha256.to_string(),
            actual,
        });
    }
    Ok(())
}

/// Fetch and verify every file in the manifest into `dest_dir`, returning the
/// paths in manifest order.
///
/// A file already present and valid is kept as is: staging again costs nothing.
/// New bytes go to `<name>.part` and are renamed into place only after the
/// checksum matches, so a reader never sees a half-written model.
pub fn stage(
    source: &dyn Source,
    manifest: &Manifest,
    dest_dir: &Path,
    offline: bool,
) -> Result<Vec<PathBuf>, StageError> {
    if offline {
        return Err(StageError::Offline);
    }
    fs::create_dir_all(dest_dir).map_err(StageError::Io)?;

    let mut staged = Vec::with_capacity(manifest.files.len());
    for file in &manifest.files {
        // A hand-built Manifest skips parse_manifest, so recheck here.
        if !is_plain_name(&file.name) {
            return Err(StageError::UnsafeName(file.name.clone()));
        }
        let dest = dest_dir.join(&file.name);
        if dest.is_file() && verify_file(&dest, &file.sha256, file.bytes).is_ok() {
            staged.push(dest);
            continue;
        }

        let part = dest_dir.join(format!("{}.part", file.name));
        let _ = fs::remove_file(&part);
        let mut out = fs::File::create(&part).map_err(StageError::Io)?;
        let fetched = source.fetch(&file.url, &mut out);
        drop(out);

        if let Err(e) = fetched {
            let _ = fs::remove_file(&part);
            return Err(StageError::Fetch(e));
        }
        if let Err(e) = verify_file(&part, &file.sha256, file.bytes) {
            let _ = fs::remove_file(&part);
            return Err(StageError::Verify(e));
        }
        if dest.exists() {
            fs::remove_file(&dest).map_err(StageError::Io)?;
        }
        fs::rename(&part, &dest).map_err(StageError::Io)?;
        staged.push(dest);
    }
    Ok(staged)
}

/// A name is safe only if it is one ordinary path component: no `..`, no
/// separators, no absolute path.
fn is_plain_name(name: &str) -> bool {
    let mut components = Path::new(name).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Unique tempdir under std::env::temp_dir(), removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!(
                "newton-models-{tag}-{}-{}-{nanos}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Serves fixed bodies by URL and counts how often it was asked.
    struct MemSource {
        bodies: Vec<(&'static str, Vec<u8>)>,
        truncate_to: Option<usize>,
        calls: Cell<usize>,
    }

    impl MemSource {
        fn new(bodies: Vec<(&'static str, Vec<u8>)>) -> Self {
            Self {
                bodies,
                truncate_to: None,
                calls: Cell::new(0),
            }
        }

        fn truncated_to(mut self, len: usize) -> Self {
            self.truncate_to = Some(len);
            self
        }
    }

    impl Source for MemSource {
        fn fetch(&self, url: &str, into: &mut dyn Write) -> Result<(), FetchError> {
            self.calls.set(self.calls.get() + 1);
            let (_, body) = self
                .bodies
                .iter()
                .find(|(u, _)| *u == url)
                .ok_or_else(|| FetchError::new(format!("no fixture for {url}")))?;
            let body = match self.truncate_to {
                Some(len) => &body[..len],
                None => &body[..],
            };
            into.write_all(body)
                .map_err(|e| FetchError::new(e.to_string()))
        }
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        hex(&Sha256::digest(bytes))
    }

    fn json(name: &str, url: &str, sha: &str, bytes: usize) -> String {
        format!(
            r#"{{"version":"v1","files":[{{"name":"{name}","url":"{url}","sha256":"{sha}","bytes":{bytes}}}]}}"#
        )
    }

    fn good_manifest(name: &str, url: &str, body: &[u8]) -> Manifest {
        parse_manifest(&json(name, url, &sha256_hex(body), body.len())).unwrap()
    }

    const URL: &str = "https://github.com/OWNER/REPO/releases/download/v1/x.gguf";

    #[test]
    fn example_manifest_parses() {
        let manifest = parse_manifest(include_str!("../manifest.example.json")).unwrap();
        assert_eq!(manifest.version, "v1.0.0");
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].name, "lfm3-audio.gguf");
        assert!(is_sha256(&manifest.files[0].sha256));
        assert!(manifest.files[0].bytes > 0);
    }

    #[test]
    fn stage_writes_a_verified_file_and_leaves_no_part() {
        let dir = TempDir::new("stage-ok");
        let body = b"fake gguf bytes".to_vec();
        let source = MemSource::new(vec![(URL, body.clone())]);
        let manifest = good_manifest("lfm3-audio.gguf", URL, &body);

        let staged = stage(&source, &manifest, dir.path(), false).unwrap();

        assert_eq!(staged, vec![dir.path().join("lfm3-audio.gguf")]);
        assert_eq!(fs::read(&staged[0]).unwrap(), body);
        assert!(!dir.path().join("lfm3-audio.gguf.part").exists());
        assert_eq!(source.calls.get(), 1);
    }

    #[test]
    fn stage_again_does_not_fetch_a_valid_file() {
        let dir = TempDir::new("stage-skip");
        let body = b"already here".to_vec();
        let source = MemSource::new(vec![(URL, body.clone())]);
        let manifest = good_manifest("lfm3-audio.gguf", URL, &body);

        stage(&source, &manifest, dir.path(), false).unwrap();
        stage(&source, &manifest, dir.path(), false).unwrap();

        assert_eq!(source.calls.get(), 1);
    }

    #[test]
    fn checksum_mismatch_deletes_the_partial_file() {
        let dir = TempDir::new("stage-badsha");
        let body = b"bytes that will not match".to_vec();
        let wrong = "0".repeat(64);
        let manifest = parse_manifest(&json("lfm3-audio.gguf", URL, &wrong, body.len())).unwrap();
        let source = MemSource::new(vec![(URL, body)]);

        let err = stage(&source, &manifest, dir.path(), false).unwrap_err();

        assert!(matches!(
            err,
            StageError::Verify(VerifyError::Checksum { .. })
        ));
        assert!(!dir.path().join("lfm3-audio.gguf").exists());
        assert!(!dir.path().join("lfm3-audio.gguf.part").exists());
    }

    #[test]
    fn truncated_download_leaves_nothing_behind() {
        let dir = TempDir::new("stage-truncated");
        let body = b"a full model that got cut".to_vec();
        let source = MemSource::new(vec![(URL, body.clone())]).truncated_to(5);
        let manifest = good_manifest("lfm3-audio.gguf", URL, &body);

        let err = stage(&source, &manifest, dir.path(), false).unwrap_err();

        match err {
            StageError::Verify(VerifyError::Size { expected, actual }) => {
                assert_eq!(expected, body.len() as u64);
                assert_eq!(actual, 5);
            }
            other => panic!("expected a size error, got {other}"),
        }
        assert!(!dir.path().join("lfm3-audio.gguf").exists());
        assert!(!dir.path().join("lfm3-audio.gguf.part").exists());
    }

    #[test]
    fn offline_refuses_without_touching_the_source() {
        let dir = TempDir::new("stage-offline");
        let body = b"needed".to_vec();
        let source = MemSource::new(vec![(URL, body.clone())]);
        let manifest = good_manifest("lfm3-audio.gguf", URL, &body);

        let err = stage(&source, &manifest, dir.path(), true).unwrap_err();

        assert!(matches!(err, StageError::Offline));
        assert_eq!(source.calls.get(), 0);
        assert!(!dir.path().join("lfm3-audio.gguf").exists());
    }

    #[test]
    fn fetch_failure_leaves_no_partial_file() {
        let dir = TempDir::new("stage-fetch");
        let body = b"never served".to_vec();
        let source = MemSource::new(Vec::new());
        let manifest = good_manifest("lfm3-audio.gguf", URL, &body);

        let err = stage(&source, &manifest, dir.path(), false).unwrap_err();

        assert!(matches!(err, StageError::Fetch(_)));
        assert!(!dir.path().join("lfm3-audio.gguf.part").exists());
    }

    #[test]
    fn duplicate_file_names_are_rejected() {
        let sha = "0".repeat(64);
        let body = format!(
            r#"{{"version":"v1","files":[
                {{"name":"x.gguf","url":"{URL}","sha256":"{sha}","bytes":1}},
                {{"name":"x.gguf","url":"{URL}","sha256":"{sha}","bytes":1}}]}}"#
        );

        assert!(matches!(
            parse_manifest(&body),
            Err(ManifestError::Duplicate(name)) if name == "x.gguf"
        ));
    }

    #[test]
    fn names_outside_the_destination_are_rejected() {
        let sha = "0".repeat(64);
        for name in ["../evil.gguf", "sub/evil.gguf", "/tmp/evil.gguf", "..", "."] {
            let body = json(name, URL, &sha, 1);
            assert!(
                matches!(
                    parse_manifest(&body),
                    Err(ManifestError::UnsafeName(n)) if n == name
                ),
                "{name} should be rejected"
            );
        }
    }

    #[test]
    fn stage_rejects_an_unsafe_name_from_a_hand_built_manifest() {
        let dir = TempDir::new("stage-handbuilt");
        let source = MemSource::new(Vec::new());
        let manifest = Manifest {
            version: "v1".into(),
            files: vec![FileEntry {
                name: "../evil.gguf".into(),
                url: URL.into(),
                sha256: "0".repeat(64),
                bytes: 1,
            }],
        };

        let err = stage(&source, &manifest, dir.path(), false).unwrap_err();

        assert!(matches!(err, StageError::UnsafeName(name) if name == "../evil.gguf"));
        assert_eq!(source.calls.get(), 0);
        assert!(!dir.path().parent().unwrap().join("evil.gguf").exists());
    }

    #[test]
    fn malformed_checksums_are_rejected() {
        for sha in ["", "abc", &"g".repeat(64), &"0".repeat(63)] {
            let body = json("x.gguf", URL, sha, 1);
            assert!(
                matches!(
                    parse_manifest(&body),
                    Err(ManifestError::BadChecksum(name)) if name == "x.gguf"
                ),
                "{sha:?} should be rejected"
            );
        }
    }

    #[test]
    fn verify_file_reports_size_then_checksum() {
        let dir = TempDir::new("verify");
        let path = dir.path().join("model.bin");
        fs::write(&path, b"hello").unwrap();

        assert!(matches!(
            verify_file(&path, &sha256_hex(b"hello"), 5),
            Ok(())
        ));
        assert!(matches!(
            verify_file(&path, &sha256_hex(b"hello"), 9),
            Err(VerifyError::Size {
                expected: 9,
                actual: 5
            })
        ));
        assert!(matches!(
            verify_file(&path, &"0".repeat(64), 5),
            Err(VerifyError::Checksum { .. })
        ));
        assert!(matches!(
            verify_file(&dir.path().join("missing.bin"), &sha256_hex(b""), 0),
            Err(VerifyError::Io(_))
        ));
    }
}
