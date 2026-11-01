//! Local embedding index.
//!
//! This module ranks vectors and nothing else: it never loads a model and
//! never touches the network. The embedder that produces vectors lands later
//! (see docs/Security Audit.md for the offline promise).

use std::error::Error;
use std::fmt;

/// One node's vector, as produced by the embedder.
#[derive(Debug, Clone, PartialEq)]
pub struct Embedding {
    pub node: u64,
    pub vector: Vec<f32>,
}

impl Embedding {
    pub fn new(node: u64, vector: Vec<f32>) -> Self {
        Self { node, vector }
    }
}

/// Why a vector was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexError {
    /// Every vector in an index shares one dimension.
    DimensionMismatch { expected: usize, got: usize },
    /// A NaN or infinite component would make scores sort as nonsense.
    BadVector,
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch { expected, got } => write!(
                f,
                "vector dimension {got} does not match index dimension {expected}"
            ),
            Self::BadVector => f.write_str("vector contains a NaN or infinite component"),
        }
    }
}

impl Error for IndexError {}

/// Fixed-dimension vectors, scored by cosine similarity.
#[derive(Debug, Clone, Default)]
pub struct Index {
    dim: Option<usize>,
    embeddings: Vec<Embedding>,
}

impl Index {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }

    /// The shared dimension, or `None` until the first vector is inserted.
    pub fn dimensions(&self) -> Option<usize> {
        self.dim
    }

    /// Add a vector. The first one fixes the dimension for the whole index.
    pub fn insert(&mut self, embedding: Embedding) -> Result<(), IndexError> {
        if !is_finite(&embedding.vector) {
            return Err(IndexError::BadVector);
        }
        match self.dim {
            None => self.dim = Some(embedding.vector.len()),
            Some(dim) if dim != embedding.vector.len() => {
                return Err(IndexError::DimensionMismatch {
                    expected: dim,
                    got: embedding.vector.len(),
                })
            }
            Some(_) => {}
        }
        self.embeddings.push(embedding);
        Ok(())
    }

    /// The `k` closest nodes, best score first; equal scores go by node id.
    ///
    /// An empty index returns an empty list, and `k` past the end returns all
    /// of it. The search is brute force: every vector is scored.
    pub fn nearest(&self, query: &[f32], k: usize) -> Result<Vec<(u64, f32)>, IndexError> {
        if self.embeddings.is_empty() {
            return Ok(Vec::new());
        }
        let mut scored = Vec::with_capacity(self.embeddings.len());
        for embedding in &self.embeddings {
            scored.push((embedding.node, cosine(query, &embedding.vector)?));
        }
        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        scored.truncate(k);
        Ok(scored)
    }

    /// Cosine similarity against any vector, without storing it.
    pub fn cosine(&self, a: &[f32], b: &[f32]) -> Result<f32, IndexError> {
        cosine(a, b)
    }
}

/// Cosine similarity in `[-1, 1]`, accumulated in f64 to keep long vectors stable.
pub fn cosine(a: &[f32], b: &[f32]) -> Result<f32, IndexError> {
    if a.len() != b.len() {
        return Err(IndexError::DimensionMismatch {
            expected: a.len(),
            got: b.len(),
        });
    }
    if !is_finite(a) || !is_finite(b) {
        return Err(IndexError::BadVector);
    }
    let (mut dot, mut norm_a, mut norm_b) = (0.0f64, 0.0f64, 0.0f64);
    for (&x, &y) in a.iter().zip(b) {
        let (x, y) = (x as f64, y as f64);
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        // A zero vector has no direction; call it orthogonal so scores stay finite.
        return Ok(0.0);
    }
    Ok((dot / denom).clamp(-1.0, 1.0) as f32)
}

fn is_finite(vector: &[f32]) -> bool {
    vector.iter().all(|component| component.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(results: &[(u64, f32)]) -> Vec<u64> {
        results.iter().map(|(node, _)| *node).collect()
    }

    #[test]
    fn index_ranks_known_vectors_by_hand_computed_score() {
        let mut index = Index::new();
        index.insert(Embedding::new(1, vec![1.0, 0.0])).unwrap(); // cos = 1
        index.insert(Embedding::new(2, vec![1.0, 1.0])).unwrap(); // cos = 1/sqrt(2)
        index.insert(Embedding::new(3, vec![0.0, 1.0])).unwrap(); // cos = 0
        index.insert(Embedding::new(4, vec![-1.0, 0.0])).unwrap(); // cos = -1

        let got = index.nearest(&[1.0, 0.0], 4).unwrap();
        assert_eq!(nodes(&got), vec![1, 2, 3, 4]);
        assert_eq!(got[0].1, 1.0);
        assert!((got[1].1 - 0.5f32.sqrt()).abs() < 1e-6);
        assert_eq!(got[2].1, 0.0);
        assert_eq!(got[3].1, -1.0);
    }

    #[test]
    fn index_ties_break_by_node_id() {
        let mut index = Index::new();
        // All three score 1/sqrt(2) against [1, 0]; inserted out of order.
        index.insert(Embedding::new(9, vec![1.0, 1.0])).unwrap();
        index.insert(Embedding::new(4, vec![1.0, -1.0])).unwrap();
        index.insert(Embedding::new(7, vec![1.0, 1.0])).unwrap();

        let got = index.nearest(&[1.0, 0.0], 3).unwrap();
        assert_eq!(nodes(&got), vec![4, 7, 9]);
        assert_eq!(got[0].1, got[1].1);
        assert_eq!(got[1].1, got[2].1);
    }

    #[test]
    fn index_rejects_dimension_mismatch() {
        let mut index = Index::new();
        index
            .insert(Embedding::new(1, vec![1.0, 0.0, 0.0]))
            .unwrap();
        assert_eq!(index.dimensions(), Some(3));

        let err = index.insert(Embedding::new(2, vec![1.0, 0.0])).unwrap_err();
        assert_eq!(
            err,
            IndexError::DimensionMismatch {
                expected: 3,
                got: 2
            }
        );
        assert_eq!(index.len(), 1);

        let err = index.nearest(&[1.0, 0.0], 1).unwrap_err();
        assert_eq!(
            err,
            IndexError::DimensionMismatch {
                expected: 2,
                got: 3
            }
        );
    }

    #[test]
    fn index_rejects_nan_and_infinite_components() {
        let mut index = Index::new();
        assert_eq!(
            index
                .insert(Embedding::new(1, vec![f32::NAN, 0.0]))
                .unwrap_err(),
            IndexError::BadVector
        );
        assert_eq!(
            index
                .insert(Embedding::new(2, vec![f32::INFINITY, 0.0]))
                .unwrap_err(),
            IndexError::BadVector
        );
        assert_eq!(
            index
                .insert(Embedding::new(3, vec![0.0, f32::NEG_INFINITY]))
                .unwrap_err(),
            IndexError::BadVector
        );
        assert!(index.is_empty());

        index.insert(Embedding::new(4, vec![1.0, 0.0])).unwrap();
        assert_eq!(
            index.nearest(&[f32::NAN, 1.0], 1).unwrap_err(),
            IndexError::BadVector
        );
    }

    #[test]
    fn index_is_empty_before_anything_is_inserted() {
        let index = Index::new();
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        assert_eq!(index.dimensions(), None);
        assert_eq!(index.nearest(&[1.0, 0.0], 5).unwrap(), Vec::new());
    }

    #[test]
    fn index_k_past_the_end_returns_everything() {
        let mut index = Index::new();
        index.insert(Embedding::new(1, vec![1.0, 0.0])).unwrap();
        index.insert(Embedding::new(2, vec![0.0, 1.0])).unwrap();
        index.insert(Embedding::new(3, vec![-1.0, 0.0])).unwrap();

        let got = index.nearest(&[1.0, 0.0], 10).unwrap();
        assert_eq!(nodes(&got), vec![1, 2, 3]);
        assert_eq!(index.nearest(&[1.0, 0.0], 0).unwrap(), Vec::new());
    }

    #[test]
    fn index_top_five_of_a_thousand_matches_a_naive_full_sort() {
        const DIM: usize = 8;
        let mut index = Index::new();
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut all: Vec<(u64, Vec<f32>)> = Vec::new();

        for node in 0..1000u64 {
            let mut vector = Vec::with_capacity(DIM);
            for _ in 0..DIM {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                // Top bits of the state, mapped to [-1, 1).
                vector.push((state >> 40) as f32 / (1u32 << 23) as f32 - 1.0);
            }
            index.insert(Embedding::new(node, vector.clone())).unwrap();
            all.push((node, vector));
        }

        let query = [0.25f32, -0.5, 0.75, 1.0, -1.0, 0.125, 0.5, -0.25];

        // Naive: score every vector with a plain f32 loop, full sort, take five.
        let mut naive: Vec<(u64, f32)> = all
            .iter()
            .map(|(node, vector)| (*node, naive_cosine(&query, vector)))
            .collect();
        naive.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        naive.truncate(5);

        let got = index.nearest(&query, 5).unwrap();
        assert_eq!(nodes(&got), nodes(&naive));
        for (fast, plain) in got.iter().zip(&naive) {
            assert!(
                (fast.1 - plain.1).abs() < 1e-5,
                "score for node {} was {} but naive said {}",
                fast.0,
                fast.1,
                plain.1
            );
        }
        assert!(got.windows(2).all(|pair| pair[0].1 >= pair[1].1));
    }

    fn naive_cosine(a: &[f32], b: &[f32]) -> f32 {
        let (mut dot, mut norm_a, mut norm_b) = (0.0f32, 0.0f32, 0.0f32);
        for (&x, &y) in a.iter().zip(b) {
            dot += x * y;
            norm_a += x * x;
            norm_b += y * y;
        }
        let denom = norm_a.sqrt() * norm_b.sqrt();
        if denom == 0.0 {
            0.0
        } else {
            dot / denom
        }
    }
}
