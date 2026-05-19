//! Structured status snapshot for CLI / desktop UI.

use std::path::{Path, PathBuf};

use serde::Serialize;
use glean_core::{
    pipeline::reranker::{onnx_model_exists, resolve_rerank_model_path},
    GleanConfig, GlobalLayout, WorkspaceIndexLayout,
};

use crate::{workspace::resolve_workspace_from_env, HostError};

/// Runtime summary for hosts to format (CLI tracing, Tauri UI, etc.).
#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    pub version: &'static str,
    pub storage_root: PathBuf,
    pub workspace_root: PathBuf,
    pub index_root: PathBuf,
    pub index_db_path: PathBuf,
    pub index_vectors_path: PathBuf,
    pub index_db_exists: bool,
    pub index_vectors_exists: bool,
    pub global_config_path: PathBuf,
    pub global_config_exists: bool,
    pub deprecated_workspace_config: Option<PathBuf>,
    pub legacy_global_index: bool,
    pub rerank_enabled: bool,
    pub rerank_model_path: PathBuf,
    pub rerank_model_ready: bool,
    pub log_level: String,
}

/// Collect status for a specific workspace root (desktop / explicit path).
pub fn collect_status_for_workspace(workspace: &Path) -> Result<StatusReport, HostError> {
    collect_status_inner(workspace)
}

/// Collect status for the resolved workspace and current env layout.
pub fn collect_status() -> Result<StatusReport, HostError> {
    let workspace = resolve_workspace_from_env()?;
    collect_status_inner(&workspace)
}

fn collect_status_inner(workspace: &Path) -> Result<StatusReport, HostError> {
    let cfg = GleanConfig::load_merged()?;
    let global = GlobalLayout::from_env_or_default()?;
    let index = WorkspaceIndexLayout::for_workspace(workspace);
    let global_config = global.global_config_path();
    let index_db = index.metadata_db_path();
    let index_vectors = index.lancedb_directory();
    let rerank_path = resolve_rerank_model_path(&global, &cfg.rerank);
    let deprecated = workspace.join(".glean").join("config.toml");
    let workspace = workspace.to_path_buf();
    let deprecated_ws_config = deprecated.is_file().then_some(deprecated);

    Ok(StatusReport {
        version: glean_core::VERSION,
        storage_root: global.root.clone(),
        workspace_root: workspace.clone(),
        index_root: index.root.clone(),
        index_db_path: index_db.clone(),
        index_vectors_path: index_vectors.clone(),
        index_db_exists: index_db.is_file(),
        index_vectors_exists: index_vectors.is_dir(),
        global_config_path: global_config.clone(),
        global_config_exists: global_config.is_file(),
        deprecated_workspace_config: deprecated_ws_config,
        legacy_global_index: global.legacy_index_present(),
        rerank_enabled: cfg.rerank.enabled,
        rerank_model_path: rerank_path.clone(),
        rerank_model_ready: onnx_model_exists(&rerank_path),
        log_level: cfg.log.level.clone(),
    })
}
