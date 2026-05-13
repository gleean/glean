//! FastEmbed (ONNX) backend — `EmbeddingModel::AllMiniLML6V2`, 384 dimensions.

use std::sync::Mutex;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use super::{assert_embedding_rows, Embedder};
use crate::error::CoreError;
use crate::store::lance_chunks::EMBEDDING_DIM;

/// `AllMiniLM-L6-v2` text embeddings; first run may download model artifacts.
pub struct FastembedEmbedder {
    inner: Mutex<TextEmbedding>,
}

impl FastembedEmbedder {
    /// Initialize the default retrieval model (384 dims).
    pub fn new() -> Result<Self, CoreError> {
        let inner = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(false),
        )
        .map_err(|e| CoreError::Embedding(e.to_string()))?;
        let model = Self {
            inner: Mutex::new(inner),
        };
        model.smoke_check_dim()?;
        Ok(model)
    }

    fn smoke_check_dim(&self) -> Result<(), CoreError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| CoreError::Msg("FastEmbed mutex poisoned".into()))?;
        let out = guard
            .embed(vec!["passage: __dim_check__".to_owned()], None)
            .map_err(|e| CoreError::Embedding(e.to_string()))?;
        let row = out
            .first()
            .ok_or_else(|| CoreError::Embedding("empty embed output".into()))?;
        if row.len() != EMBEDDING_DIM as usize {
            return Err(CoreError::Embedding(format!(
                "model output dimension {} does not match Lance schema {}; align the model or wipe the vectors directory",
                row.len(),
                EMBEDDING_DIM
            )));
        }
        Ok(())
    }
}

impl Embedder for FastembedEmbedder {
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, CoreError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let owned: Vec<String> = texts.iter().map(|s| format!("passage: {s}")).collect();
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| CoreError::Msg("FastEmbed mutex poisoned".into()))?;
        let rows = guard
            .embed(owned, None)
            .map_err(|e| CoreError::Embedding(e.to_string()))?;
        assert_embedding_rows(&rows)?;
        Ok(rows)
    }

    fn embed_query(&self, query: &str) -> Result<Vec<f32>, CoreError> {
        let owned = vec![format!("query: {query}")];
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| CoreError::Msg("FastEmbed mutex poisoned".into()))?;
        let rows = guard
            .embed(owned, None)
            .map_err(|e| CoreError::Embedding(e.to_string()))?;
        assert_embedding_rows(&rows)?;
        rows.into_iter()
            .next()
            .ok_or_else(|| CoreError::Embedding("empty query embedding".into()))
    }
}
