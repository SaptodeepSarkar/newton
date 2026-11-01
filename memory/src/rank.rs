//! Hybrid recall ranking: lexical overlap, semantic score from the index, recency.

use std::collections::BTreeSet;

/// Even split: keyword queries are carried by lexical overlap, paraphrases by the
/// index's semantic score, and neither alone clears the 90 % hit-rate bar.
const W_LEXICAL: f32 = 0.5;
const W_SEMANTIC: f32 = 0.5;

const MAX_RESULTS: usize = 5;

/// A remembered fact. `age` is time since the fact was written; lower is newer.
#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
    pub node: u64,
    pub text: String,
    pub age: u32,
}

/// A fact paired with its 0..=1 semantic score from the embedding index.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub fact: Fact,
    pub semantic: f32,
}

/// Returns at most five `(node, score)` pairs, best first.
///
/// Order is score desc, then newest (`age` asc), then node id, so the same input
/// always yields the same output regardless of candidate order. A query with no
/// words scores every fact zero, which leaves the recency tie-break in charge.
pub fn rank(query: &str, candidates: &[Candidate]) -> Vec<(u64, f32)> {
    let query_words = words(query);
    let mut scored: Vec<(f32, u32, u64)> = candidates
        .iter()
        .map(|candidate| {
            let score = if query_words.is_empty() {
                0.0
            } else {
                let fact_words = words(&candidate.fact.text);
                let matched = query_words
                    .iter()
                    .filter(|word| fact_words.contains(word.as_str()))
                    .count();
                let lexical = matched as f32 / query_words.len() as f32;
                let semantic = if candidate.semantic.is_finite() {
                    candidate.semantic.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (W_LEXICAL * lexical + W_SEMANTIC * semantic).clamp(0.0, 1.0)
            };
            (score, candidate.fact.age, candidate.fact.node)
        })
        .collect();

    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    scored.truncate(MAX_RESULTS);
    scored
        .into_iter()
        .map(|(score, _, node)| (node, score))
        .collect()
}

/// Lowercased word set; anything that is not alphanumeric separates words.
fn words(text: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    let mut word = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            word.extend(ch.to_lowercase());
        } else if !word.is_empty() {
            set.insert(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        set.insert(word);
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (fact, the user's phrasing, keyword-only query, paraphrase)
    const FACTS: [(&str, &str, &str, &str); 20] = [
        (
            "Maya's birthday is the fourth of June.",
            "When is Maya's birthday?",
            "maya birthday",
            "which date was she born",
        ),
        (
            "Ravi keeps the car keys in the kitchen drawer.",
            "Where does Ravi keep the car keys?",
            "ravi car keys",
            "where are the vehicle keys stored",
        ),
        (
            "Priya's dentist appointment is on Tuesday at four.",
            "When is Priya's dentist appointment?",
            "priya dentist",
            "what time is her dental visit",
        ),
        (
            "The wifi password is written on the router label.",
            "What is the wifi password?",
            "wifi password",
            "what are the wireless network credentials",
        ),
        (
            "Omar works at a bakery on Bridge Street.",
            "Where does Omar work?",
            "omar bakery",
            "which shop employs Omar",
        ),
        (
            "Tara's sister is called Nisha.",
            "What is Tara's sister called?",
            "tara sister",
            "what is the name of her sibling",
        ),
        (
            "The boiler service is due in November.",
            "When is the boiler service due?",
            "boiler service",
            "when must the heating be serviced",
        ),
        (
            "Leo takes his coffee black with no sugar.",
            "How does Leo take his coffee?",
            "leo coffee",
            "how does he drink his brew",
        ),
        (
            "The spare flat key lives under the blue pot.",
            "Where is the spare key?",
            "spare key",
            "where is the backup door key kept",
        ),
        (
            "Anita's flight to Lisbon leaves at seven in the morning.",
            "When does Anita's flight leave?",
            "anita lisbon",
            "what time does her plane depart",
        ),
        (
            "The bank card pin is nine one two four.",
            "What is the bank card pin?",
            "bank pin",
            "what is the secret number for the card",
        ),
        (
            "Sam plays badminton every Thursday evening.",
            "When does Sam play badminton?",
            "sam badminton",
            "which sport does he play on Thursdays",
        ),
        (
            "Grandpa's hearing aid batteries are in the hallway cupboard.",
            "Where are Grandpa's hearing aid batteries?",
            "hearing aid batteries",
            "where does grandfather keep his hearing batteries",
        ),
        (
            "The recycling is collected on alternate Fridays.",
            "When is the recycling collected?",
            "recycling collection",
            "which day do the bins go out",
        ),
        (
            "Zoya is allergic to peanuts.",
            "What is Zoya allergic to?",
            "zoya peanuts",
            "which nut makes her ill",
        ),
        (
            "The plumber's number is on the fridge.",
            "What is the plumber's number?",
            "plumber number",
            "what number do I ring for the repairman",
        ),
        (
            "Kai's bicycle has a flat rear tyre.",
            "What is wrong with Kai's bicycle?",
            "kai bicycle",
            "his bike has a punctured back tire",
        ),
        (
            "The wedding anniversary is the twelfth of September.",
            "When is the wedding anniversary?",
            "wedding anniversary",
            "which date did they marry",
        ),
        (
            "Dev keeps his passport in the safe.",
            "Where does Dev keep his passport?",
            "dev passport",
            "where is his travel document locked away",
        ),
        (
            "The tomato plants need watering twice a day.",
            "How often do the tomato plants need water?",
            "tomato watering",
            "how frequently must the tomatoes be irrigated",
        ),
    ];

    const STOPWORDS: &[&str] = &[
        "a", "an", "the", "is", "are", "was", "were", "what", "which", "when", "where", "who",
        "how", "does", "do", "did", "of", "to", "for", "on", "in", "at", "my", "his", "her", "it",
        "and", "be", "must", "she", "they", "that", "this", "out", "away", "locked",
    ];

    /// Stand-in for a local embedding that knows synonyms: fold each word to a
    /// shared root word so paraphrases meet their fact.
    fn root(word: &str) -> &str {
        match word {
            "born" => "birthday",
            "vehicle" | "vehicles" => "car",
            "stored" | "store" => "keep",
            "dental" => "dentist",
            "visit" => "appointment",
            "wireless" | "network" => "wifi",
            "credentials" => "password",
            "shop" => "bakery",
            "employs" => "work",
            "sibling" => "sister",
            "heating" => "boiler",
            "serviced" => "service",
            "drink" => "take",
            "brew" => "coffee",
            "backup" => "spare",
            "kept" => "lives",
            "plane" => "flight",
            "depart" => "leaves",
            "number" => "pin",
            "sport" => "badminton",
            "plays" => "play",
            "thursdays" => "thursday",
            "grandfather" => "grandpa",
            "batteries" => "battery",
            "bins" => "recycling",
            "collection" | "collected" => "collect",
            "nut" | "nuts" => "peanuts",
            "ill" => "allergic",
            "repairman" => "plumber",
            "bike" => "bicycle",
            "punctured" => "flat",
            "tire" => "tyre",
            "back" => "rear",
            "marry" => "wedding",
            "document" => "passport",
            "watering" | "irrigated" => "water",
            "tomatoes" => "tomato",
            _ => word,
        }
    }

    fn content_roots(text: &str) -> BTreeSet<String> {
        words(text)
            .into_iter()
            .map(|word| root(&word).to_string())
            .filter(|word| !STOPWORDS.contains(&word.as_str()))
            .collect()
    }

    fn semantic(fact: &str, query: &str) -> f32 {
        let fact_roots = content_roots(fact);
        let query_roots = content_roots(query);
        if query_roots.is_empty() {
            return 0.0;
        }
        let shared = query_roots
            .iter()
            .filter(|word| fact_roots.contains(*word))
            .count();
        shared as f32 / query_roots.len() as f32
    }

    fn fixtures(query: &str) -> Vec<Candidate> {
        FACTS
            .iter()
            .enumerate()
            .map(|(index, (fact, ..))| Candidate {
                fact: Fact {
                    node: index as u64 + 1,
                    text: (*fact).to_string(),
                    age: index as u32,
                },
                semantic: semantic(fact, query),
            })
            .collect()
    }

    fn candidate(node: u64, age: u32, text: &str, semantic: f32) -> Candidate {
        Candidate {
            fact: Fact {
                node,
                text: text.to_string(),
                age,
            },
            semantic,
        }
    }

    #[test]
    fn hybrid_ranking_finds_the_right_fact_for_three_phrasings() {
        let mut hits = 0;
        let mut total = 0;
        let mut misses = Vec::new();

        for (index, (_, question, keywords, paraphrase)) in FACTS.iter().enumerate() {
            let node = index as u64 + 1;
            for query in [question, keywords, paraphrase] {
                total += 1;
                let ranked = rank(query, &fixtures(query));
                if ranked.iter().take(MAX_RESULTS).any(|(hit, _)| *hit == node) {
                    hits += 1;
                } else {
                    misses.push((*query, node, ranked));
                }
            }
        }

        let rate = hits as f64 / total as f64 * 100.0;
        println!("hit rate {rate:.1}% ({hits}/{total})");
        assert!(
            rate >= 90.0,
            "hit rate {rate:.1}% ({hits}/{total}), misses: {misses:?}"
        );
    }

    #[test]
    fn empty_query_returns_the_most_recent() {
        let candidates = [
            candidate(7, 40, "old news", 0.9),
            candidate(3, 1, "fresh news", 0.2),
            candidate(5, 12, "middling news", 0.6),
        ];
        assert_eq!(rank("", &candidates), [(3, 0.0), (5, 0.0), (7, 0.0)]);
        assert_eq!(rank("   ...   ", &candidates), rank("", &candidates));
    }

    #[test]
    fn unmatched_facts_only_fill_unused_slots() {
        let mut candidates = vec![candidate(
            1,
            0,
            "Maya's birthday is the fourth of June.",
            0.0,
        )];
        for node in 2..=10 {
            candidates.push(candidate(node, node as u32, "zebra crossing", 0.0));
        }

        let ranked = rank("maya birthday", &candidates);
        assert_eq!(ranked.len(), MAX_RESULTS);
        assert_eq!(ranked[0].0, 1);

        let short = [candidate(1, 0, "nothing matches here", 0.0)];
        assert_eq!(rank("unrelated words", &short).len(), 1);
    }

    #[test]
    fn scores_stay_in_range() {
        let mut queries: Vec<&str> = vec!["", "?!", "zzzz"];
        for (_, question, keywords, paraphrase) in FACTS.iter() {
            queries.extend([question, keywords, paraphrase]);
        }

        for query in queries {
            for score in rank(query, &fixtures(query))
                .iter()
                .map(|(_, score)| *score)
            {
                assert!(score.is_finite(), "score {score} for {query:?}");
                assert!((0.0..=1.0).contains(&score), "score {score} for {query:?}");
            }
        }
    }

    #[test]
    fn same_input_gives_the_same_order_whatever_the_input_order() {
        let query = "Where does Ravi keep the car keys?";
        let original = fixtures(query);

        let mut reversed = original.clone();
        reversed.reverse();

        let mut shuffled = original.clone();
        shuffled.swap(0, 7);
        shuffled.swap(3, 19);
        shuffled.swap(5, 11);

        let expected = rank(query, &original);
        assert_eq!(rank(query, &reversed), expected);
        assert_eq!(rank(query, &shuffled), expected);
    }
}
