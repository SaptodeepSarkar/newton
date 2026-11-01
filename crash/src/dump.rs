// A local crash dump that cannot carry audio.
//
// The mic buffer is in-memory only; nothing here may ever grow a path back to
// it. Every string goes through `redact`, the note is capped, the directory is
// capped, and nothing leaves the machine (see docs/Security Audit.md §2).

use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// Longest note that is written, in chars.
pub const NOTE_LIMIT: usize = 512;
/// Most dumps kept in one directory.
pub const MAX_DUMPS: usize = 5;
/// Replacement for anything shaped like speech or a sample buffer.
const REDACTED: &str = "[redacted]";

/// What a dump can say. No audio field exists, by design.
#[derive(Debug, Clone)]
pub struct Dump {
    pub at_unix: u64,
    pub version: &'static str,
    pub state: &'static str,
    pub last_tool: Option<String>,
    pub note: String,
}

/// Why a dump was not written. Never a panic.
#[derive(Debug)]
pub enum DumpError {
    /// The directory escapes itself (`..`) or is a symlink.
    BadPath,
    /// The path exists but is not a directory.
    NotADirectory,
    /// The filesystem refused; includes a missing directory.
    Io(io::Error),
}

impl fmt::Display for DumpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DumpError::BadPath => f.write_str("dump directory is outside its own path"),
            DumpError::NotADirectory => f.write_str("dump path is not a directory"),
            DumpError::Io(e) => write!(f, "dump io error: {e}"),
        }
    }
}

impl std::error::Error for DumpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DumpError::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// Write `dump` as `crash-<at_unix>.txt` inside `dir`, then keep the newest
/// [`MAX_DUMPS`] files. Returns the file written.
pub fn write(dump: &Dump, dir: &Path) -> Result<PathBuf, DumpError> {
    check_dir(dir)?;

    // Safe by construction: the name is a u64, never user text.
    let path = dir.join(format!("crash-{}.txt", dump.at_unix));
    let mut body = String::new();
    body.push_str(&format!("at_unix: {}\n", dump.at_unix));
    body.push_str(&format!("version: {}\n", redact(dump.version)));
    body.push_str(&format!("state: {}\n", redact(dump.state)));
    match &dump.last_tool {
        Some(tool) => body.push_str(&format!("last_tool: {}\n", redact(tool))),
        None => body.push_str("last_tool: none\n"),
    }
    body.push_str(&format!("note: {}\n", cut(&redact(&dump.note))));

    fs::write(&path, body).map_err(DumpError::Io)?;
    prune(dir)?;
    Ok(path)
}

/// Drop everything that is not printable ASCII, then whole strings that read
/// like speech or look like a sample buffer.
fn redact(s: &str) -> String {
    let total = s.chars().count();
    let printable: String = s
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .collect();
    let kept = printable.chars().count();

    // A PCM/opus blob loses most of its chars to this filter; prose does not.
    if total > 0 && kept * 5 < total * 4 {
        return REDACTED.to_string();
    }

    let mut out = String::with_capacity(kept);
    for token in printable.split_whitespace() {
        if !looks_like_word(token) {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(token);
    }

    // shortcut: heuristic scrub — a spoken sentence always ends in . ! or ?
    // and is dropped; an unterminated one reads as a debug note. Tighten if
    // notes ever start carrying raw transcripts.
    if out.is_empty() || reads_like_sentence(&out) {
        return REDACTED.to_string();
    }
    out
}

/// Word-shaped, not a run of leftover sample bytes.
fn looks_like_word(token: &str) -> bool {
    let len = token.len();
    // Digits count too, or a version like "0.1.0" is scrubbed to nothing.
    let alnum = token.chars().filter(char::is_ascii_alphanumeric).count();
    (3..=48).contains(&len) && alnum * 2 >= len
}

fn reads_like_sentence(s: &str) -> bool {
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if matches!(c, '.' | '!' | '?') && chars.peek().is_none_or(|n| n.is_whitespace()) {
            return true;
        }
    }
    false
}

/// Truncate to [`NOTE_LIMIT`] chars on a char boundary.
fn cut(s: &str) -> String {
    s.chars().take(NOTE_LIMIT).collect()
}

fn check_dir(dir: &Path) -> Result<(), DumpError> {
    if dir.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(DumpError::BadPath);
    }
    let meta = fs::symlink_metadata(dir).map_err(DumpError::Io)?;
    if meta.file_type().is_symlink() {
        return Err(DumpError::BadPath);
    }
    if !meta.is_dir() {
        return Err(DumpError::NotADirectory);
    }
    Ok(())
}

/// Keep the newest [`MAX_DUMPS`] dumps, deleting the oldest first.
fn prune(dir: &Path) -> Result<(), DumpError> {
    let mut found: Vec<(u64, PathBuf)> = Vec::new();
    for entry in fs::read_dir(dir).map_err(DumpError::Io)? {
        let entry = entry.map_err(DumpError::Io)?;
        let name = entry.file_name();
        if let Some(stamp) = name.to_str().and_then(stamp_of) {
            found.push((stamp, entry.path()));
        }
    }
    found.sort_by_key(|a| std::cmp::Reverse(a.0));
    for (_, path) in found.iter().skip(MAX_DUMPS) {
        fs::remove_file(path).map_err(DumpError::Io)?;
    }
    Ok(())
}

fn stamp_of(name: &str) -> Option<u64> {
    name.strip_prefix("crash-")?
        .strip_suffix(".txt")?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "newton-dump-{}-{}-{}",
                std::process::id(),
                tag,
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
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

    fn dump(at_unix: u64, note: &str) -> Dump {
        Dump {
            at_unix,
            version: "0.1.0",
            state: "idle",
            last_tool: Some("gmail.read".to_string()),
            note: note.to_string(),
        }
    }

    fn field(body: &str, key: &str) -> Option<String> {
        body.lines()
            .find_map(|line| line.strip_prefix(&format!("{key}: ")))
            .map(str::to_string)
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut all: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        all.sort();
        all
    }

    #[test]
    fn writes_only_allowed_keys() {
        let dir = TempDir::new("keys");
        let path = write(&dump(1_700_000_000, "resume failed"), dir.path()).unwrap();

        assert_eq!(path.file_name().unwrap(), "crash-1700000000.txt");
        let body = fs::read_to_string(&path).unwrap();
        let mut keys: Vec<&str> = body
            .lines()
            .map(|line| line.split_once(": ").unwrap().0)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["at_unix", "last_tool", "note", "state", "version"] as [&str; 5]
        );
        assert_eq!(field(&body, "at_unix").unwrap(), "1700000000");
        assert_eq!(field(&body, "version").unwrap(), "0.1.0");
        assert_eq!(field(&body, "state").unwrap(), "idle");
        assert_eq!(field(&body, "last_tool").unwrap(), "gmail.read");
        assert_eq!(field(&body, "note").unwrap(), "resume failed");
    }

    #[test]
    fn pcm_buffer_leaves_no_raw_bytes() {
        let dir = TempDir::new("pcm");
        let pcm: Vec<i16> = (0..160)
            .map(|i| ((i as f32) * 0.4).sin() * 9_000.0)
            .map(|v| v as i16)
            .collect();
        let mut raw = Vec::new();
        for sample in &pcm {
            raw.extend_from_slice(&sample.to_le_bytes());
        }
        assert_eq!(raw.len(), 320);

        let spoken = "I forgot my bank password and then the app closed.";
        let note = format!("{} {}", String::from_utf8_lossy(&raw), spoken);
        let path = write(&dump(1_700_000_001, &note), dir.path()).unwrap();

        let bytes = fs::read(&path).unwrap();
        assert!(!bytes.contains(&0), "NUL byte in dump");
        assert!(
            bytes
                .iter()
                .all(|b| b.is_ascii_graphic() || *b == b'\n' || *b == b' '),
            "raw sample bytes reached the dump"
        );
        let body = String::from_utf8(bytes).unwrap();
        for word in ["forgot", "password", "bank", "closed"] {
            assert!(!body.contains(word), "{word} survived the scrub");
        }
        assert_eq!(field(&body, "note").unwrap(), REDACTED);
    }

    #[test]
    fn long_note_is_truncated() {
        let dir = TempDir::new("long");
        let note = "state ".repeat(400);
        assert!(note.chars().count() > 2_000);
        let path = write(&dump(1_700_000_002, &note), dir.path()).unwrap();

        let body = fs::read_to_string(&path).unwrap();
        let written = field(&body, "note").unwrap();
        assert_eq!(written.chars().count(), NOTE_LIMIT);
    }

    #[test]
    fn sixth_dump_deletes_the_oldest() {
        let dir = TempDir::new("prune");
        for i in 0..6 {
            write(&dump(1_700_000_000 + i, "state resumed"), dir.path()).unwrap();
        }

        let kept = names(dir.path());
        assert_eq!(kept.len(), MAX_DUMPS);
        assert_eq!(
            kept,
            [
                "crash-1700000001.txt",
                "crash-1700000002.txt",
                "crash-1700000003.txt",
                "crash-1700000004.txt",
                "crash-1700000005.txt",
            ]
        );
    }

    #[test]
    fn bad_paths_are_errors_not_panics() {
        let dir = TempDir::new("paths");
        let d = dump(1_700_000_003, "state resumed");

        assert!(matches!(
            write(&d, &dir.path().join("..")),
            Err(DumpError::BadPath)
        ));

        let file = dir.path().join("not-a-dir");
        fs::write(&file, b"x").unwrap();
        assert!(matches!(write(&d, &file), Err(DumpError::NotADirectory)));

        assert!(matches!(
            write(&d, &dir.path().join("missing")),
            Err(DumpError::Io(_))
        ));

        #[cfg(unix)]
        {
            let link = dir.path().join("link");
            std::os::unix::fs::symlink(dir.path(), &link).unwrap();
            assert!(matches!(write(&d, &link), Err(DumpError::BadPath)));
        }
    }
}
