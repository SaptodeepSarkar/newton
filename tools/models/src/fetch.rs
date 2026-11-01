//! `newton-fetch`: stage every file in a manifest into a model directory and
//! say, one line per file, what landed and what did not.

use newton_models::{parse_manifest, stage, DefaultSource, DirSource, Manifest, Source};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let source: Box<dyn Source> = match source_dir(&args) {
        Some(dir) => Box::new(DirSource::new(dir)),
        None => Box::new(DefaultSource),
    };
    ExitCode::from(run(&args, source.as_ref()) as u8)
}

/// Fetch and verify every manifest file into `--into`. 0 if all landed,
/// 1 if any failed, 2 if the arguments are wrong.
pub fn run(args: &[String], source: &dyn Source) -> i32 {
    run_with(args, source, &mut io::stdout())
}

fn run_with(args: &[String], source: &dyn Source, out: &mut dyn Write) -> i32 {
    let args = match parse_args(args) {
        Ok(args) => args,
        Err(message) => {
            let _ = writeln!(out, "{message}");
            return 2;
        }
    };

    let json = match fs::read_to_string(&args.manifest) {
        Ok(json) => json,
        Err(e) => {
            let _ = writeln!(
                out,
                "FAIL {} cannot read manifest: {e}",
                args.manifest.display()
            );
            return 1;
        }
    };
    let manifest = match parse_manifest(&json) {
        Ok(manifest) => manifest,
        Err(e) => {
            let _ = writeln!(out, "FAIL {} {e}", args.manifest.display());
            return 1;
        }
    };

    let mut failed = false;
    for file in &manifest.files {
        // One file at a time: a bad checksum must not stop the ones after it.
        let one = Manifest {
            version: manifest.version.clone(),
            files: vec![file.clone()],
        };
        match stage(source, &one, &args.into, args.offline) {
            Ok(_) => {
                let _ = writeln!(out, "ok {} {}", file.name, file.bytes);
            }
            Err(e) => {
                failed = true;
                let _ = writeln!(out, "FAIL {} {e}", file.name);
            }
        }
    }
    if failed {
        1
    } else {
        0
    }
}

struct Args {
    manifest: PathBuf,
    into: PathBuf,
    offline: bool,
}

const USAGE: &str = "usage: newton-fetch --manifest <f> --into <dir> [--offline] [--source <dir>]";

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut manifest = None;
    let mut into = None;
    let mut offline = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--manifest" => manifest = Some(value(&mut it, arg)?),
            "--into" => into = Some(value(&mut it, arg)?),
            // main already turned this into a Source; here it is only syntax.
            "--source" => {
                value(&mut it, arg)?;
            }
            "--offline" => offline = true,
            other => return Err(format!("unexpected argument {other:?}\n{USAGE}")),
        }
    }
    Ok(Args {
        manifest: manifest.ok_or_else(|| format!("--manifest is required\n{USAGE}"))?,
        into: into.ok_or_else(|| format!("--into is required\n{USAGE}"))?,
        offline,
    })
}

fn value<'a>(it: &mut impl Iterator<Item = &'a String>, flag: &str) -> Result<PathBuf, String> {
    it.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} needs a value\n{USAGE}"))
}

fn source_dir(args: &[String]) -> Option<PathBuf> {
    let at = args.iter().position(|arg| arg == "--source")?;
    args.get(at + 1).map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use newton_models::FetchError;
    use sha2::{Digest, Sha256};
    use std::cell::Cell;
    use std::path::Path;
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
                "newton-fetch-{tag}-{}-{}-{nanos}",
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

    /// A source that fails loudly if asked for anything.
    struct CountingSource(Cell<usize>);

    impl Source for CountingSource {
        fn fetch(&self, _url: &str, _into: &mut dyn Write) -> Result<(), FetchError> {
            self.0.set(self.0.get() + 1);
            Err(FetchError::new("source must not be touched"))
        }
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    /// One-entry manifest for `name`, served at `url` with the given hash.
    fn manifest_json(name: &str, url: &str, sha: &str, bytes: usize) -> String {
        format!(
            r#"{{"version":"v1","files":[{{"name":"{name}","url":"{url}","sha256":"{sha}","bytes":{bytes}}}]}}"#
        )
    }

    /// Writes both a manifest and the local source copy of one file.
    fn setup(tag: &str, body: &[u8], sha: &str) -> (TempDir, Vec<String>, PathBuf) {
        let dir = TempDir::new(tag);
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        let url = "https://example.invalid/m.gguf";
        fs::write(src.join("m.gguf"), body).unwrap();
        let manifest = dir.path().join("manifest.json");
        fs::write(&manifest, manifest_json("m.gguf", url, sha, body.len())).unwrap();
        let into = dir.path().join("models");
        let args = vec![
            "--manifest".to_string(),
            manifest.display().to_string(),
            "--into".to_string(),
            into.display().to_string(),
        ];
        (dir, args, src)
    }

    fn text(out: &[u8]) -> String {
        String::from_utf8(out.to_vec()).unwrap()
    }

    #[test]
    fn fetch_good_manifest_writes_the_file_and_exits_zero() {
        let body = b"a whole model";
        let (_dir, args, src) = setup("good", body, &sha256_hex(body));
        let mut out = Vec::new();

        assert_eq!(run_with(&args, &DirSource::new(&src), &mut out), 0);
        assert_eq!(text(&out), format!("ok m.gguf {}\n", body.len()));
    }

    #[test]
    fn fetch_bad_checksum_names_the_file_and_exits_one() {
        let body = b"a whole model";
        let (_dir, args, src) = setup("bad", body, &"0".repeat(64));
        let mut out = Vec::new();

        assert_eq!(run_with(&args, &DirSource::new(&src), &mut out), 1);
        let line = text(&out);
        assert!(line.starts_with("FAIL m.gguf "), "{line}");
        assert!(line.contains("hashes to"), "{line}");
    }

    #[test]
    fn fetch_offline_exits_one_without_touching_the_source() {
        let body = b"a whole model";
        let (dir, mut args, _src) = setup("offline", body, &sha256_hex(body));
        args.push("--offline".to_string());
        let source = CountingSource(Cell::new(0));
        let mut out = Vec::new();

        assert_eq!(run_with(&args, &source, &mut out), 1);
        assert_eq!(source.0.get(), 0);
        assert!(text(&out).starts_with("FAIL m.gguf offline"), "{out:?}");
        assert!(!dir.path().join("models").join("m.gguf").exists());
    }

    #[test]
    fn fetch_without_a_source_names_the_missing_pipeline() {
        let body = b"a whole model";
        let (_dir, args, _src) = setup("no-source", body, &sha256_hex(body));
        let mut out = Vec::new();

        assert_eq!(run_with(&args, &DefaultSource, &mut out), 1);
        let line = text(&out);
        assert!(line.starts_with("FAIL m.gguf "), "{line}");
        assert!(line.contains("release pipeline not wired yet"), "{line}");
    }

    #[test]
    fn fetch_usage_is_two() {
        let mut out = Vec::new();
        let args = vec!["--into".to_string(), "/tmp/x".to_string()];

        assert_eq!(run_with(&args, &DefaultSource, &mut out), 2);
        assert!(text(&out).contains("--manifest is required"));
    }
}
