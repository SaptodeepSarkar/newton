//! Graph store: nodes, edges, one JSON file written atomically, one writer per directory.

// shortcut: graph is plaintext at rest, add a keyring-backed envelope when the first real user data lands

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const GRAPH: &str = "graph.json";
const TMP: &str = "graph.json.tmp";
const LOCK: &str = "graph.lock";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: u64,
    pub kind: String,
    pub text: String,
    pub created_turn: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: u64,
    pub to: u64,
    pub label: String,
}

/// Unknown `kind` and `label` values are kept as-is: the schema is deliberately open.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Graph {
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
}

#[derive(Debug)]
pub enum StoreError {
    /// Another `Store` in this process or another one holds `graph.lock`.
    AlreadyOpen,
    /// `graph.json` was not valid JSON for this schema.
    Corrupt(serde_json::Error),
    Io(io::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::AlreadyOpen => write!(f, "graph store is already open in this directory"),
            StoreError::Corrupt(e) => write!(f, "graph.json is corrupt: {e}"),
            StoreError::Io(e) => write!(f, "graph store io error: {e}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::AlreadyOpen => None,
            StoreError::Corrupt(e) => Some(e),
            StoreError::Io(e) => Some(e),
        }
    }
}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        StoreError::Io(e)
    }
}

/// Owns a directory. Holding it holds an exclusive lock; dropping it releases the lock.
#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    graph: Graph,
    _lock: File,
}

impl Store {
    /// Creates `dir` if needed, takes `graph.lock`, and loads `graph.json` (absent file = empty graph).
    pub fn open(dir: impl AsRef<Path>) -> Result<Store, StoreError> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join(LOCK))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(StoreError::AlreadyOpen),
            Err(TryLockError::Error(e)) => return Err(StoreError::Io(e)),
        }

        let graph = match fs::read(dir.join(GRAPH)) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(StoreError::Corrupt)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Graph::default(),
            Err(e) => return Err(StoreError::Io(e)),
        };

        Ok(Store {
            dir,
            graph,
            _lock: lock,
        })
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Replaces the graph on disk first, and only then in memory.
    ///
    /// The new contents go to `graph.json.tmp` and are fsynced before the rename, so a crash
    /// at any point leaves either the previous `graph.json` or the new one, never a torn file.
    pub fn save(&mut self, graph: Graph) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec_pretty(&graph).map_err(StoreError::Corrupt)?;

        let tmp = self.dir.join(TMP);
        let mut file = File::create(&tmp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, self.dir.join(GRAPH))?;
        sync_dir(&self.dir);

        self.graph = graph;
        Ok(())
    }
}

/// The rename is only durable once the directory entry itself is flushed.
#[cfg(unix)]
fn sync_dir(dir: &Path) {
    if let Ok(d) = File::open(dir) {
        let _ = d.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_dir(_: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> TempDir {
            use std::sync::atomic::{AtomicU32, Ordering};
            static N: AtomicU32 = AtomicU32::new(0);
            let n = N.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir()
                .join(format!("newton-memory-{tag}-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
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

    fn sample() -> Graph {
        Graph {
            nodes: vec![
                Node {
                    id: 1,
                    kind: "fact".into(),
                    text: "the user is called Sam".into(),
                    created_turn: 3,
                },
                Node {
                    id: 2,
                    kind: "preference".into(),
                    text: "likes tea".into(),
                    created_turn: 4,
                },
            ],
            edges: vec![Edge {
                from: 1,
                to: 2,
                label: "prefers".into(),
            }],
        }
    }

    #[test]
    fn empty_dir_opens_empty_and_round_trips() {
        let tmp = TempDir::new("roundtrip");
        let mut store = Store::open(tmp.path()).unwrap();
        assert_eq!(store.graph(), &Graph::default());

        store.save(sample()).unwrap();
        assert_eq!(store.graph(), &sample());
        assert!(tmp.path().join(GRAPH).exists());
        assert!(!tmp.path().join(TMP).exists());
    }

    #[test]
    fn reopening_loads_identical_data() {
        let tmp = TempDir::new("reopen");
        let saved = {
            let mut store = Store::open(tmp.path()).unwrap();
            store.save(sample()).unwrap();
            sample()
        };
        let store = Store::open(tmp.path()).unwrap();
        assert_eq!(store.graph(), &saved);
    }

    #[test]
    fn interrupted_write_leaves_the_old_graph_readable() {
        let tmp = TempDir::new("crash");
        let mut store = Store::open(tmp.path()).unwrap();
        store.save(sample()).unwrap();
        drop(store);

        // A writer that died after fsync of the tmp file but before the rename.
        fs::write(tmp.path().join(TMP), b"{\"nodes\":[{\"id\":").unwrap();

        let store = Store::open(tmp.path()).unwrap();
        assert_eq!(store.graph(), &sample());
        drop(store);

        // The leftover tmp file does not stop the next save either.
        let mut store = Store::open(tmp.path()).unwrap();
        store.save(Graph::default()).unwrap();
        drop(store);
        let store = Store::open(tmp.path()).unwrap();
        assert_eq!(store.graph(), &Graph::default());
    }

    #[test]
    fn second_open_fails_while_the_first_lives() {
        let tmp = TempDir::new("lock");
        let first = Store::open(tmp.path()).unwrap();
        match Store::open(tmp.path()) {
            Err(StoreError::AlreadyOpen) => {}
            other => panic!("expected AlreadyOpen, got {other:?}"),
        }
        drop(first);
        Store::open(tmp.path()).expect("lock is released on drop");
    }

    #[test]
    fn corrupt_graph_is_an_error_not_a_panic() {
        let tmp = TempDir::new("corrupt");
        fs::write(tmp.path().join(GRAPH), b"{ not json at all").unwrap();
        match Store::open(tmp.path()) {
            Err(StoreError::Corrupt(_)) => {}
            other => panic!("expected Corrupt, got {other:?}"),
        }
        // Truncated but valid-prefix JSON is equally corrupt, and still no panic.
        fs::write(tmp.path().join(GRAPH), b"").unwrap();
        match Store::open(tmp.path()) {
            Err(StoreError::Corrupt(_)) => {}
            other => panic!("expected Corrupt, got {other:?}"),
        }
    }

    #[test]
    fn unknown_kind_and_label_survive_a_round_trip() {
        let tmp = TempDir::new("open-schema");
        let graph = Graph {
            nodes: vec![Node {
                id: 7,
                kind: "kind-from-the-future".into(),
                text: "kept verbatim".into(),
                created_turn: 0,
            }],
            edges: vec![Edge {
                from: 7,
                to: 7,
                label: "self-reference".into(),
            }],
        };
        Store::open(tmp.path())
            .unwrap()
            .save(graph.clone())
            .unwrap();
        assert_eq!(Store::open(tmp.path()).unwrap().graph(), &graph);
    }
}
