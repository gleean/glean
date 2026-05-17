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

    pub fn logs_directory(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Ensure cache/logs dirs exist (not project index).
    pub fn ensure_global_directories(&self) -> Result<(), StorageError> {
        create_dir(&self.reranker_cache_dir())?;
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
    let root = env::var("GLEAN_STORAGE_ROOT")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".glean")
        });
    if root.as_os_str().is_empty() {
        return Err(StorageError::EmptyRoot);
    }
    Ok(root)
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
