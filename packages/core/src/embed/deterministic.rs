//! Deterministic pseudo-embeddings for tests and environments without ONNX.

use sha2::{Digest, Sha256};

use super::{assert_embedding_rows, Embedder};
use crate::error::CoreError;
use crate::store::lance_chunks::EMBEDDING_DIM;

/// L2-normalized pseudo vectors expanded from a SHA-256 digest of the input text.
/// **Tests / offline CI only** — not a semantic embedding model.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeterministicEmbedder;

impl DeterministicEmbedder {
    pub const fn new() -> Self {
        Self
    }
}

fn vector_from_text(text: &str, dim: usize) -> Vec<f32> {
    let digest = Sha256::digest(text.as_bytes());
    let mut out = vec![0.0_f32; dim];
    for (i, slot) in out.iter_mut().enumerate().take(dim) {
        let base = (i * 7) % (digest.len().saturating_sub(3).max(1));
        let b0 = digest[base];
        let b1 = digest[(base + 1) % digest.len()];
        let b2 = digest[(base + 2) % digest.len()];
        let b3 = digest[(base + 3) % digest.len()];
        let u = u32::from_le_bytes([b0, b1, b2, b3]);
        *slot = (u as f32 / u32::MAX as f32) * 2.0 - 1.0;
    }
    let norm: f32 = out.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 {
        for x in &mut out {
            *x /= norm;
        }
    }
    out
}

impl Embedder for DeterministicEmbedder {
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, CoreError> {
        let dim = EMBEDDING_DIM as usize;
        let rows: Vec<Vec<f32>> = texts.iter().map(|t| vector_from_text(t, dim)).collect();
        assert_embedding_rows(&rows)?;
        Ok(rows)
    }

    fn embed_query(&self, query: &str) -> Result<Vec<f32>, CoreError> {
        self.embed_batch(&[query])?
            .into_iter()
            .next()
            .ok_or_else(|| CoreError::Msg("empty embed_query".into()))
    }
}
