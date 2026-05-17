//! `glean models pull`: explicit rerank artifact download (offline prep).

use anyhow::{bail, Context, Result};

/// Download rerank ONNX / tokenizer cache into `$GLEAN_STORAGE_ROOT/cache/reranker/`.
pub fn run_models_pull(model: &str) -> Result<()> {
    match model {
        "rerank" => {
            let layout = glean_core::open_storage().map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let cache = glean_core::pipeline::reranker::pull_bge_rerank_model(&layout)
                .context("pull BGE rerank model")?;
            eprintln!(
                "Rerank assets ready under {} (set [rerank].enabled and optional model_path)",
                cache.display()
            );
            Ok(())
        }
        other => bail!("unknown model kind `{other}`; supported: rerank"),
    }
}
