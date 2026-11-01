//! Per-turn context budget: which remembered facts reach the model.
//!
//! Facts are pinned or not. Pinned ones go in first, oldest (lowest id) first,
//! because the user asked for them. Everything else goes in by age, oldest
//! first, then by id, so two runs over the same facts always produce the same
//! prompt. A fact that does not fit is dropped whole, never cut in half.
//!
//! When the pinned facts alone do not fit, the oldest pinned ones that fit are
//! kept and [`Fitted::overflowed`] is set: the caller must summarise rather
//! than believe the model saw everything pinned. This crate does not summarise
//! yet; that is the next step.

use std::cmp::Ordering;

/// One remembered fact, as the budget sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    /// Stable identity, also the tie-breaker for ordering. Lower is older.
    pub id: u64,
    /// What the model will read. Counted in characters, not bytes.
    pub text: String,
    /// True when the user pinned it: it is kept before anything else.
    pub pinned: bool,
    /// Turns since the fact was last useful. Lower is fresher.
    pub age: u32,
}

/// What one turn may spend on facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// Most facts that may be kept.
    pub max_facts: usize,
    /// Most characters the kept texts may total.
    pub max_chars: usize,
}

/// The result of one [`fit`], in the order the facts must be presented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fitted {
    /// Fact ids that made it in, in presentation order.
    pub kept: Vec<u64>,
    /// Fact ids that did not, in the order they were considered.
    pub dropped: Vec<u64>,
    /// Characters in the kept texts; never above [`Budget::max_chars`].
    pub used_chars: usize,
    /// True when a pinned fact was dropped for want of room.
    pub overflowed: bool,
}

/// Choose the facts for one turn. Pure: same input, same output.
pub fn fit(facts: &[Fact], budget: &Budget) -> Fitted {
    let mut order: Vec<&Fact> = facts.iter().collect();
    order.sort_by(|a, b| match (a.pinned, b.pinned) {
        (true, true) => a.id.cmp(&b.id),
        (false, false) => a.age.cmp(&b.age).then(a.id.cmp(&b.id)),
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
    });

    let mut used_chars = 0;
    let mut overflowed = false;
    // Once a pinned fact does not fit, the older pinned ones are done: the
    // budget is a prefix, not a knapsack.
    let mut pinned_full = false;
    let mut kept = Vec::with_capacity(facts.len());
    let mut dropped = Vec::new();

    for fact in order {
        let chars = fact.text.chars().count();
        let room = kept.len() < budget.max_facts && used_chars + chars <= budget.max_chars;
        let take = if fact.pinned {
            let take = !pinned_full && room;
            if !take {
                pinned_full = true;
                overflowed = true;
            }
            take
        } else {
            room
        };

        if take {
            kept.push(fact.id);
            used_chars += chars;
        } else {
            dropped.push(fact.id);
        }
    }

    Fitted {
        kept,
        dropped,
        used_chars,
        overflowed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic stand-in for real facts: no randomness, no floats.
    fn synthetic(count: usize, seed: u64) -> Vec<Fact> {
        let mut x = seed | 1;
        (0..count)
            .map(|i| {
                x = x
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                Fact {
                    id: i as u64,
                    text: "x".repeat((x % 60) as usize),
                    pinned: x.is_multiple_of(17),
                    age: ((x >> 32) % 50) as u32,
                }
            })
            .collect()
    }

    fn chars_of(facts: &[Fact], ids: &[u64]) -> usize {
        facts
            .iter()
            .filter(|f| ids.contains(&f.id))
            .map(|f| f.text.chars().count())
            .sum()
    }

    /// Both ceilings hold, every fact is accounted for, and no fact is cut.
    fn assert_invariants(facts: &[Fact], budget: &Budget, fitted: &Fitted) {
        assert!(fitted.kept.len() <= budget.max_facts, "too many facts");
        assert!(fitted.used_chars <= budget.max_chars, "too many chars");
        assert_eq!(fitted.kept.len() + fitted.dropped.len(), facts.len());
        assert_eq!(fitted.used_chars, chars_of(facts, &fitted.kept));
        assert!(fitted.dropped.iter().all(|id| !fitted.kept.contains(id)));
        let mut ids = fitted.kept.clone();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), fitted.kept.len(), "a fact was kept twice");
    }

    #[test]
    fn a_thousand_facts_never_exceed_the_budget() {
        let facts = synthetic(1_000, 7);
        for max_facts in [1, 7, 50, 400, 1_000] {
            for max_chars in [0, 100, 500, 5_000, 50_000] {
                let budget = Budget {
                    max_facts,
                    max_chars,
                };
                let fitted = fit(&facts, &budget);
                assert_invariants(&facts, &budget, &fitted);
            }
        }
    }

    #[test]
    fn pinned_facts_survive_the_crowd() {
        let facts = vec![
            // Old, unpinned, large: would be last in line.
            Fact {
                id: 0,
                text: "x".repeat(40),
                pinned: false,
                age: 0,
            },
            Fact {
                id: 1,
                text: "pinned".to_string(),
                pinned: true,
                age: 90,
            },
            Fact {
                id: 2,
                text: "also pinned".to_string(),
                pinned: true,
                age: 90,
            },
        ];
        let budget = Budget {
            max_facts: 2,
            max_chars: 20,
        };
        let fitted = fit(&facts, &budget);
        assert_eq!(fitted.kept, vec![1, 2]);
        assert_eq!(fitted.dropped, vec![0]);
        assert!(!fitted.overflowed);
        assert_invariants(&facts, &budget, &fitted);
    }

    #[test]
    fn order_is_independent_of_input_order() {
        let facts = synthetic(200, 11);
        let budget = Budget {
            max_facts: 25,
            max_chars: 400,
        };
        let expected = fit(&facts, &budget);
        let mut reversed = facts.clone();
        reversed.reverse();
        let mut rotated = facts.clone();
        rotated.rotate_left(77);
        assert_eq!(fit(&reversed, &budget), expected);
        assert_eq!(fit(&rotated, &budget), expected);
        // Same input twice: same output.
        assert_eq!(fit(&facts, &budget), expected);
    }

    #[test]
    fn more_pinned_facts_than_room_report_overflow() {
        let facts: Vec<Fact> = (0..300)
            .map(|id| Fact {
                id,
                text: "pinned".to_string(),
                pinned: true,
                age: 0,
            })
            .collect();
        let budget = Budget {
            max_facts: 10,
            max_chars: 10_000,
        };
        let fitted = fit(&facts, &budget);
        assert_eq!(fitted.kept.len(), 10);
        assert_eq!(fitted.kept, (0..10).collect::<Vec<u64>>());
        assert!(fitted.overflowed);
        assert_eq!(fitted.dropped.len(), 290);
        assert_invariants(&facts, &budget, &fitted);
    }

    #[test]
    fn empty_input_is_not_an_error() {
        let budget = Budget {
            max_facts: 0,
            max_chars: 0,
        };
        let fitted = fit(&[], &budget);
        assert_eq!(
            fitted,
            Fitted {
                kept: Vec::new(),
                dropped: Vec::new(),
                used_chars: 0,
                overflowed: false,
            }
        );
    }

    #[test]
    fn a_fact_longer_than_the_budget_is_dropped_not_truncated() {
        let facts = vec![
            Fact {
                id: 0,
                text: "y".repeat(51),
                pinned: false,
                age: 0,
            },
            Fact {
                id: 1,
                text: "small".to_string(),
                pinned: false,
                age: 1,
            },
        ];
        let budget = Budget {
            max_facts: 10,
            max_chars: 50,
        };
        let fitted = fit(&facts, &budget);
        assert_eq!(fitted.kept, vec![1]);
        assert_eq!(fitted.dropped, vec![0]);
        assert_eq!(fitted.used_chars, 5);
        assert_invariants(&facts, &budget, &fitted);
    }
}
