//! Provenance, forgetting and export for remembered facts.
//!
//! A forgotten node leaves a tombstone, and tombstones travel in the export, so
//! a later import of an older export cannot bring the fact back.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Where a memory came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    SaidByUser,
    ReadFromCalendar,
    ReadFromGmail,
    Derived,
}

/// Who said it, on which turn, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub source: Source,
    pub turn: u64,
    pub at_unix: u64,
    /// Parent this was derived from; forgetting the parent forgets this too.
    pub derived_from: Option<u64>,
}

/// One remembered fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memory {
    pub node: u64,
    pub text: String,
    pub provenance: Provenance,
}

/// The whole memory store as it goes to disk.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    pub memories: Vec<Memory>,
    pub tombstones: Vec<u64>,
}

/// Why a ledger could not be imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// Not a ledger this build understands.
    Invalid(String),
    /// A derived memory names a parent node that is not in the ledger.
    DanglingParent(u64),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Invalid(why) => write!(f, "invalid ledger: {why}"),
            ImportError::DanglingParent(parent) => write!(
                f,
                "memory derived from node {parent}, which is not in the ledger"
            ),
        }
    }
}

impl std::error::Error for ImportError {}

impl Ledger {
    /// Remove `node` and everything derived from it, transitively. Returns how
    /// many memories went. Forgetting something unknown changes nothing.
    pub fn forget(&mut self, node: u64) -> usize {
        if !self.memories.iter().any(|m| m.node == node) {
            return 0;
        }
        let before = self.memories.len();

        // Grow the dead set until no surviving parent points into it. Linear
        // scan per round: a ledger is a few hundred facts, not a graph store.
        let mut dead = vec![node];
        let mut next = 0;
        while next < dead.len() {
            let parent = dead[next];
            for m in &self.memories {
                if m.provenance.derived_from == Some(parent) && !dead.contains(&m.node) {
                    dead.push(m.node);
                }
            }
            next += 1;
        }

        self.memories.retain(|m| !dead.contains(&m.node));
        for id in dead {
            if !self.tombstones.contains(&id) {
                self.tombstones.push(id);
            }
        }
        before - self.memories.len()
    }

    /// The whole ledger as JSON.
    pub fn export(&self) -> String {
        serde_json::to_string(self).expect("a ledger has no keys serde_json can reject")
    }

    /// Read a ledger back. Tombstoned ids are dropped (and so are their
    /// children), and a derived memory whose parent is missing is refused
    /// rather than accepted as an orphan.
    pub fn import(json: &str) -> Result<Ledger, ImportError> {
        let mut ledger: Ledger =
            serde_json::from_str(json).map_err(|e| ImportError::Invalid(e.to_string()))?;

        for id in ledger.tombstones.clone() {
            ledger.forget(id);
        }
        for m in &ledger.memories {
            if let Some(parent) = m.provenance.derived_from {
                if !ledger.memories.iter().any(|p| p.node == parent) {
                    return Err(ImportError::DanglingParent(parent));
                }
            }
        }
        Ok(ledger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(node: u64) -> Memory {
        Memory {
            node,
            text: format!("fact {node}"),
            provenance: Provenance {
                source: Source::SaidByUser,
                turn: 1,
                at_unix: 1_700_000_000,
                derived_from: None,
            },
        }
    }

    fn derived(node: u64, parent: u64) -> Memory {
        Memory {
            node,
            text: format!("fact {node}"),
            provenance: Provenance {
                source: Source::Derived,
                turn: 2,
                at_unix: 1_700_000_001,
                derived_from: Some(parent),
            },
        }
    }

    fn ledger(memories: Vec<Memory>) -> Ledger {
        Ledger {
            memories,
            tombstones: Vec::new(),
        }
    }

    #[test]
    fn derived_children_are_forgotten_with_their_parent() {
        let mut l = ledger(vec![fact(1), derived(2, 1), derived(3, 2)]);
        assert_eq!(l.forget(1), 3);
        assert!(l.memories.is_empty());
        assert_eq!(l.tombstones, vec![1, 2, 3]);
    }

    #[test]
    fn unrelated_memories_survive_a_forget() {
        let mut l = ledger(vec![fact(1), derived(2, 1), fact(3)]);
        assert_eq!(l.forget(1), 2);
        assert_eq!(l.memories, vec![fact(3)]);
    }

    #[test]
    fn export_import_export_is_byte_identical() {
        let mut l = ledger(vec![fact(1), derived(2, 1), fact(3)]);
        l.forget(3);
        let first = l.export();
        let again = Ledger::import(&first).unwrap().export();
        assert_eq!(first, again);
        assert!(first.contains("\"tombstones\":[3]"));
    }

    #[test]
    fn dangling_parent_is_rejected() {
        let json = serde_json::to_string(&ledger(vec![derived(2, 9)])).unwrap();
        assert_eq!(Ledger::import(&json), Err(ImportError::DanglingParent(9)));
    }

    #[test]
    fn importing_a_tombstoned_id_drops_it() {
        // An older export (node 2 present) merged with a newer tombstone for 2.
        let mut doc = ledger(vec![fact(1), fact(2), derived(3, 2), fact(4)]);
        doc.tombstones.push(2);

        let loaded = Ledger::import(&doc.export()).unwrap();
        assert_eq!(loaded.memories, vec![fact(1), fact(4)]);
        // The child is dropped with the tombstoned parent and stays forgotten.
        assert_eq!(loaded.tombstones, vec![2, 3]);
    }

    #[test]
    fn forgetting_an_unknown_node_is_a_no_op() {
        let mut l = ledger(vec![fact(1)]);
        let before = l.export();
        assert_eq!(l.forget(99), 0);
        assert_eq!(l.export(), before);
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(matches!(
            Ledger::import("not json"),
            Err(ImportError::Invalid(_))
        ));
    }
}
