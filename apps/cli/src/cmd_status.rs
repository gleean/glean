//! `glean status`: human-readable runtime summary on stderr.

use anyhow::{Context, Result};
use glean_core::{GleanConfig, StorageLayout};

use crate::cmd_config;

/// Print version, storage layout, config presence, and reranker readiness hints.
pub async fn run_status() -> Result<()> {
    let workspace = cmd_config::resolve_workspace(None)?;
    let cfg = GleanConfig::load_merged(&workspace).context("load merged config")?;

    crate::logging::init_logging(
        crate::logging::LogRuntime::HumanStatus,
        Some(cfg.log.level.as_str()),
    )?;

    let layout = StorageLayout::from_env_or_default().map_err(|e| anyhow::anyhow!(e))?;
    let global_config = layout.root.join("config.toml");
    let workspace_config = workspace.join(".glean").join("config.toml");
    let rerank_path =
        glean_core::pipeline::reranker::resolve_rerank_model_path(&layout, &cfg.rerank);
    let rerank_ready = glean_core::pipeline::reranker::onnx_model_exists(&rerank_path);

    tracing::info!("glean {}", glean_core::VERSION);
    tracing::info!(storage_root = %layout.root.display(), "GLEAN_STORAGE_ROOT");
    tracing::info!(
        global_config = global_config.is_file(),
        path = %global_config.display(),
        "global config.toml",
    );
    tracing::info!(
        workspace_config = workspace_config.is_file(),
        path = %workspace_config.display(),
        "workspace config.toml",
    );
    tracing::info!(
        rerank_enabled = cfg.rerank.enabled,
        rerank_model_path = %rerank_path.display(),
        rerank_model_ready = rerank_ready,
        "reranker",
    );
    Ok(())
}
