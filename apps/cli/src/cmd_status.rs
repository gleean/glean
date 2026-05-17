//! `glean status`: human-readable runtime summary on stderr.

use anyhow::{Context, Result};
use glean_core::{GleanConfig, GlobalLayout, WorkspaceIndexLayout};

use crate::cmd_config;

/// Print version, storage layout, config presence, and reranker readiness hints.
pub async fn run_status() -> Result<()> {
    let workspace = cmd_config::resolve_workspace(None)?;
    let cfg = GleanConfig::load_merged().context("load merged config")?;

    crate::logging::init_logging(
        crate::logging::LogRuntime::HumanStatus,
        Some(cfg.log.level.as_str()),
    )?;

    let global = GlobalLayout::from_env_or_default().map_err(|e| anyhow::anyhow!(e))?;
    let index = WorkspaceIndexLayout::for_workspace(&workspace);
    let global_config = global.global_config_path();
    let index_db = index.metadata_db_path();
    let index_vectors = index.lancedb_directory();
    let rerank_path =
        glean_core::pipeline::reranker::resolve_rerank_model_path(&global, &cfg.rerank);
    let rerank_ready = glean_core::pipeline::reranker::onnx_model_exists(&rerank_path);

    let deprecated_ws_config = workspace.join(".glean").join("config.toml");

    tracing::info!("glean {}", glean_core::VERSION);
    tracing::info!(storage_root = %global.root.display(), "GLEAN_STORAGE_ROOT");
    tracing::info!(
        workspace = %workspace.display(),
        index_root = %index.root.display(),
        "workspace index",
    );
    tracing::info!(
        index_db = index_db.is_file(),
        path = %index_db.display(),
        "workspace index.db",
    );
    tracing::info!(
        index_vectors = index_vectors.is_dir(),
        path = %index_vectors.display(),
        "workspace vectors",
    );
    tracing::info!(
        global_config = global_config.is_file(),
        path = %global_config.display(),
        "global config.toml",
    );
    if deprecated_ws_config.is_file() {
        tracing::warn!(
            path = %deprecated_ws_config.display(),
            "deprecated workspace config.toml is ignored; merge keys into global config.toml and remove this file",
        );
    }
    if global.legacy_index_present() {
        tracing::warn!(
            storage_root = %global.root.display(),
            "legacy index under GLEAN_STORAGE_ROOT (metadata/ or vectors/); move to <workspace>/.glean/ or delete and reindex",
        );
    }
    tracing::info!(
        rerank_enabled = cfg.rerank.enabled,
        rerank_model_path = %rerank_path.display(),
        rerank_model_ready = rerank_ready,
        "reranker",
    );
    Ok(())
}
