//! Text embeddings: [`Embedder`] trait and pluggable backends (FastEmbed / deterministic stub).

mod deterministic;

#[cfg(feature = "fastembed")]
mod fastembed_backend;

pub use deterministic::DeterministicEmbedder;

#[cfg(feature = "fastembed")]
pub use fastembed_backend::FastembedEmbedder;

use crate::error::CoreError;
use crate::store::lance_chunks::EMBEDDING_DIM;

/// Batch text encoder (thread-safe).
pub trait Embedder: Send + Sync {
    /// Encode UTF-8 snippets into fixed-size vectors; length must match [`EMBEDDING_DIM`].
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, CoreError>;

    /// Encode a retrieval query (FastEmbed uses a `query:` prefix).
    fn embed_query(&self, query: &str) -> Result<Vec<f32>, CoreError>;
}

/// Default backend for CLI / daemon (requires the `fastembed` feature).
#[cfg(feature = "fastembed")]
pub fn default_embedder(
    embedding: &crate::config::EmbeddingConfig,
) -> Result<std::sync::Arc<dyn Embedder>, CoreError> {
    Ok(std::sync::Arc::new(FastembedEmbedder::new_from_config(
        embedding,
    )?))
}

#[cfg(not(feature = "fastembed"))]
pub fn default_embedder(
    _: &crate::config::EmbeddingConfig,
) -> Result<std::sync::Arc<dyn Embedder>, CoreError> {
    Err(CoreError::Msg(
        "glean-core was built without the `fastembed` feature; enable default features or `fastembed` and rebuild"
            .into(),
    ))
}

/// Runtime dimension check for embedding batches vs Lance schema.
pub(crate) fn assert_embedding_rows(rows: &[Vec<f32>]) -> Result<(), CoreError> {
    let expected = EMBEDDING_DIM as usize;
    for (i, row) in rows.iter().enumerate() {
        if row.len() != expected {
            return Err(CoreError::Msg(format!(
                "embedding dimension mismatch: expected {expected}, row {i} has {}",
                row.len()
            )));
        }
    }
    Ok(())
}
