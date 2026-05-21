//! Storage layout: global home (`$GLEAN_STORAGE_ROOT`) vs per-workspace index (`<workspace>/.glean/`).

use std::env;
use std::path::{Path, PathBuf};

use crate::error::StorageError;

/// User-global home: `config.toml`, model cache, logs (no project index).
#[derive(Debug, Clone)]
pub struct GlobalLayout {
    pub root: PathBuf,
}

/// Per-workspace index root: `<workspace>/.glean/` (`metadata/`, `vectors/` only).
#[derive(Debug, Clone)]
pub struct WorkspaceIndexLayout {
    pub root: PathBuf,
}

/// Index layout alias used by the engine and Lance helpers.
pub type StorageLayout = WorkspaceIndexLayout;

impl GlobalLayout {
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn from_env_or_default() -> Result<Self, StorageError> {
        let root = glean_home_from_env()?;
        Ok(Self { root })
    }

    pub fn global_config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn reranker_cache_dir(&self) -> PathBuf {
        self.root.join("cache").join("reranker")
    }

    /// ONNX + HF artifacts for FastEmbed text embeddings (`model.onnx`, tokenizer, etc.).
    pub fn embedding_model_cache_dir(&self) -> PathBuf {
        self.root.join("cache").join("embedding")
    }

    pub fn logs_directory(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Ensure cache/logs dirs exist (not project index).
    pub fn ensure_global_directories(&self) -> Result<(), StorageError> {
        create_dir(&self.reranker_cache_dir())?;
        create_dir(&self.embedding_model_cache_dir())?;
        create_dir(&self.logs_directory())?;
        Ok(())
    }

    /// Pre-workspace-per-index layout: `metadata/` and `vectors/` under global home.
    pub fn legacy_index_present(&self) -> bool {
        self.root.join("metadata").join("index.db").is_file() || self.root.join("vectors").is_dir()
    }
}

impl WorkspaceIndexLayout {
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn for_workspace(workspace_root: &Path) -> Self {
        Self {
            root: workspace_root.join(".glean"),
        }
    }

    pub fn metadata_db_path(&self) -> PathBuf {
        self.root.join("metadata").join("index.db")
    }

    pub fn lancedb_directory(&self) -> PathBuf {
        self.root.join("vectors")
    }

    pub const DOCUMENT_CHUNKS_TABLE: &'static str = "document_chunks";

    pub fn ensure_directories(&self) -> Result<(), StorageError> {
        create_dir(&self.root.join("metadata"))?;
        create_dir(&self.lancedb_directory())?;
        Ok(())
    }

    pub fn lancedb_uri(&self) -> PathBuf {
        self.lancedb_directory()
    }
}

fn glean_home_from_env() -> Result<PathBuf, StorageError> {
    if let Ok(s) = env::var("GLEAN_STORAGE_ROOT") {
        let t = s.trim();
        if !t.is_empty() {
            return Ok(PathBuf::from(t));
        }
    }

    // Prefer a real home directory — never use cwd-relative "." here: GUI / sidecar processes
    // often have cwd `/` or read-only roots, which would make `/.glean` or `./.glean` unusable.
    if let Some(h) = dirs::home_dir() {
        return Ok(h.join(".glean"));
    }
    if let Ok(h) = env::var("HOME") {
        if !h.trim().is_empty() {
            return Ok(PathBuf::from(h).join(".glean"));
        }
    }
    #[cfg(windows)]
    if let Ok(h) = env::var("USERPROFILE") {
        if !h.trim().is_empty() {
            return Ok(PathBuf::from(h).join(".glean"));
        }
    }

    Ok(env::temp_dir().join("glean-global"))
}

fn create_dir(path: &Path) -> Result<(), StorageError> {
    std::fs::create_dir_all(path).map_err(|source| StorageError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

/// Open global home and ensure cache/logs directories.
pub fn open_global() -> Result<GlobalLayout, StorageError> {
    let layout = GlobalLayout::from_env_or_default()?;
    layout.ensure_global_directories()?;
    Ok(layout)
}

/// Open index layout for `workspace_root` and ensure `metadata/` + `vectors/`.
pub fn open_index_for_workspace(
    workspace_root: &Path,
) -> Result<WorkspaceIndexLayout, StorageError> {
    let layout = WorkspaceIndexLayout::for_workspace(workspace_root);
    layout.ensure_directories()?;
    Ok(layout)
}
