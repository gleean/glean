//! `glean status`: human-readable runtime summary on stderr.

use anyhow::{Context, Result};

/// Print version, storage layout, config presence, and reranker readiness hints.
pub async fn run_status() -> Result<()> {
    let report = glean_host::status::collect_status().context("collect status")?;

    crate::logging::init_logging(
        crate::logging::LogRuntime::HumanStatus,
        Some(report.log_level.as_str()),
    )?;

    tracing::info!("glean {}", report.version);
    tracing::info!(storage_root = %report.storage_root.display(), "GLEAN_STORAGE_ROOT");
    tracing::info!(
        workspace = %report.workspace_root.display(),
        index_root = %report.index_root.display(),
        "workspace index",
    );
    tracing::info!(
        index_db = report.index_db_exists,
        path = %report.index_db_path.display(),
        "workspace index.db",
    );
    tracing::info!(
        index_vectors = report.index_vectors_exists,
        path = %report.index_vectors_path.display(),
        "workspace vectors",
    );
    tracing::info!(
        global_config = report.global_config_exists,
        path = %report.global_config_path.display(),
        "global config.toml",
    );
    if let Some(path) = &report.deprecated_workspace_config {
        tracing::warn!(
            path = %path.display(),
            "deprecated workspace config.toml is ignored; merge keys into global config.toml and remove this file",
        );
    }
    if report.legacy_global_index {
        tracing::warn!(
            storage_root = %report.storage_root.display(),
            "legacy index under GLEAN_STORAGE_ROOT (metadata/ or vectors/); move to <workspace>/.glean/ or delete and reindex",
        );
    }
    tracing::info!(
        rerank_enabled = report.rerank_enabled,
        rerank_model_path = %report.rerank_model_path.display(),
        rerank_model_ready = report.rerank_model_ready,
        "reranker",
    );
    Ok(())
}
