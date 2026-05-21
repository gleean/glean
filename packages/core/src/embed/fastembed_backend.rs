//! FastEmbed (ONNX) backend — model and EP from [`crate::config::EmbeddingConfig`].

use std::path::Path;
use std::sync::Mutex;

use fastembed::{
    EmbeddingModel, ExecutionProviderDispatch, ModelTrait, TextEmbedding, TextInitOptions,
};
use ort::ep::{CPU, CUDA};

#[cfg(target_os = "macos")]
use ort::ep::CoreML;

use super::{assert_embedding_rows, Embedder};
use crate::config::EmbeddingConfig;
use crate::error::CoreError;
use crate::store::lance_chunks::EMBEDDING_DIM;

/// Text embeddings via FastEmbed; honors merged TOML [`EmbeddingConfig`].
pub struct FastembedEmbedder {
    inner: Mutex<TextEmbedding>,
}

fn resolve_embedding_model(model: &str) -> Result<EmbeddingModel, CoreError> {
    let t = model.trim();
    if t.is_empty() {
        return Err(CoreError::Msg("embedding.model must not be empty".into()));
    }
    let lower = t.to_ascii_lowercase();
    if lower == "all-minilm-l6-v2"
        || lower.ends_with("/all-minilm-l6-v2")
        || lower == "sentence-transformers/all-minilm-l6-v2"
    {
        return Ok(EmbeddingModel::AllMiniLML6V2);
    }
    t.parse::<EmbeddingModel>()
        .map_err(|e: String| CoreError::Msg(format!("embedding.model: {e}")))
}

fn validate_embedding_dims(cfg: &EmbeddingConfig, model: &EmbeddingModel) -> Result<(), CoreError> {
    let info = EmbeddingModel::get_model_info(model).ok_or_else(|| {
        CoreError::Msg(format!(
            "embedding: missing FastEmbed metadata for model {model:?}"
        ))
    })?;
    if cfg.dimension as usize != info.dim {
        return Err(CoreError::Msg(format!(
            "embedding.dimension {} does not match model {:?} (expected {})",
            cfg.dimension, model, info.dim
        )));
    }
    if info.dim != EMBEDDING_DIM as usize {
        return Err(CoreError::Msg(format!(
            "embedding model dimension {} does not match Lance schema {}; change model or reindex with a matching schema",
            info.dim, EMBEDDING_DIM
        )));
    }
    Ok(())
}

fn execution_providers(device: &str) -> Result<Vec<ExecutionProviderDispatch>, CoreError> {
    match device.trim().to_ascii_lowercase().as_str() {
        "" | "cpu" => Ok(Vec::new()),
        "cuda" => Ok(vec![CUDA::default().build(), CPU::default().build()]),
        "coreml" | "npu" => {
            #[cfg(target_os = "macos")]
            {
                Ok(vec![CoreML::default().build(), CPU::default().build()])
            }
            #[cfg(not(target_os = "macos"))]
            {
                Err(CoreError::Msg(
                    "embedding.device coreml/npu is only supported on macOS".into(),
                ))
            }
        }
        other => Err(CoreError::Msg(format!(
            "embedding.device: unsupported {other:?} (allowed: cpu, cuda, coreml, npu)"
        ))),
    }
}

impl FastembedEmbedder {
    /// Build from merged config (model name, declared dimension, execution providers).
    ///
    /// `model_cache_dir` must be an absolute, writable directory for HF / ONNX artifacts.
    /// FastEmbed's default is a **relative** `.fastembed_cache` (cwd-dependent); GUI apps often
    /// have cwd `/` or read-only, which causes `Failed to retrieve model.onnx`.
    pub fn new_from_config(cfg: &EmbeddingConfig, model_cache_dir: &Path) -> Result<Self, CoreError> {
        std::fs::create_dir_all(model_cache_dir).map_err(|e| {
            CoreError::Msg(format!(
                "mkdir embedding model cache {}: {e}",
                model_cache_dir.display()
            ))
        })?;

        let model = resolve_embedding_model(&cfg.model)?;
        validate_embedding_dims(cfg, &model)?;
        let eps = execution_providers(&cfg.device)?;

        let opts = TextInitOptions::new(model)
            .with_execution_providers(eps)
            .with_show_download_progress(false)
            .with_cache_dir(model_cache_dir.to_path_buf());

        let inner =
            TextEmbedding::try_new(opts).map_err(|e| CoreError::Embedding(e.to_string()))?;
        let slf = Self {
            inner: Mutex::new(inner),
        };
        slf.smoke_check_dim()?;
        Ok(slf)
    }

    /// Same as [`Self::new_from_config`] with [`EmbeddingConfig::default`] and a temp dir (tests / smoke).
    pub fn new() -> Result<Self, CoreError> {
        let dir = std::env::temp_dir().join("glean-fastembed-default");
        std::fs::create_dir_all(&dir).map_err(|e| {
            CoreError::Msg(format!("mkdir {} for FastembedEmbedder::new: {e}", dir.display()))
        })?;
        Self::new_from_config(&EmbeddingConfig::default(), &dir)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_accepts_document_style_model_id() {
        assert_eq!(
            resolve_embedding_model("all-MiniLM-L6-v2").unwrap(),
            EmbeddingModel::AllMiniLML6V2
        );
    }

    #[test]
    fn resolve_accepts_fastembed_debug_name() {
        assert_eq!(
            resolve_embedding_model("AllMiniLML6V2").unwrap(),
            EmbeddingModel::AllMiniLML6V2
        );
    }

    #[test]
    fn dimension_mismatch_errors_without_onnx() {
        let cfg = EmbeddingConfig {
            dimension: 999,
            ..Default::default()
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let err = match FastembedEmbedder::new_from_config(&cfg, dir.path()) {
            Err(e) => e,
            Ok(_) => panic!("expected dimension validation error"),
        };
        let msg = format!("{err:?}");
        assert!(
            msg.contains("384") || msg.contains("999"),
            "unexpected err: {msg}"
        );
    }

    #[test]
    fn resolve_rejects_empty_model_id() {
        let err = resolve_embedding_model("  ").unwrap_err();
        assert!(matches!(err, CoreError::Msg(ref s) if s.contains("empty")));
    }

    #[test]
    fn unsupported_device_errors_before_loading_model() {
        let cfg = EmbeddingConfig {
            device: "tensorrt".into(),
            ..Default::default()
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let err = match FastembedEmbedder::new_from_config(&cfg, dir.path()) {
            Err(e) => e,
            Ok(_) => panic!("expected unsupported device"),
        };
        let msg = format!("{err:?}");
        assert!(
            msg.to_ascii_lowercase().contains("device") || msg.contains("tensorrt"),
            "unexpected err: {msg}"
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn coreml_device_rejected_off_macos() {
        let cfg = EmbeddingConfig {
            device: "coreml".into(),
            ..Default::default()
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let err = match FastembedEmbedder::new_from_config(&cfg, dir.path()) {
            Err(e) => e,
            Ok(_) => panic!("expected CoreML rejection off macOS"),
        };
        let msg = format!("{err:?}");
        assert!(
            msg.to_ascii_lowercase().contains("macos"),
            "unexpected err: {msg}"
        );
    }
}
