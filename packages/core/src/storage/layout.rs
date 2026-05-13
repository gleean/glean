//! Canonical directory layout under `GLEAN_STORAGE_ROOT`.

use std::env;
use std::path::{Path, PathBuf};

use crate::error::StorageError;

/// Resolved paths for metadata DB and LanceDB assets.
#[derive(Debug, Clone)]
pub struct StorageLayout {
    /// Root directory (e.g. `~/.glean` or `$GLEAN_STORAGE_ROOT`).
    pub root: PathBuf,
}

impl StorageLayout {
    /// Fixed layout rooted at `root` (tests / embeddings bypass env).
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Resolve layout from `GLEAN_STORAGE_ROOT`, falling back to `~/.glean`.
    pub fn from_env_or_default() -> Result<Self, StorageError> {
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
        Ok(Self { root })
    }

    /// `metadata/index.db`
    pub fn metadata_db_path(&self) -> PathBuf {
        self.root.join("metadata").join("index.db")
    }

    /// Directory backing the LanceDB database URI (tables live inside).
    pub fn lancedb_directory(&self) -> PathBuf {
        self.root.join("vectors")
    }

    /// LanceDB table name for chunked documents.
    pub const DOCUMENT_CHUNKS_TABLE: &'static str = "document_chunks";

    /// Create `metadata/` and `vectors/` if missing.
    pub fn ensure_directories(&self) -> Result<(), StorageError> {
        let meta = self.root.join("metadata");
        let vec_dir = self.lancedb_directory();
        create_dir(&meta)?;
        create_dir(&vec_dir)?;
        Ok(())
    }

    /// Full URI/path passed to `lancedb::connect`.
    pub fn lancedb_uri(&self) -> PathBuf {
        self.lancedb_directory()
    }
}

fn create_dir(path: &Path) -> Result<(), StorageError> {
    std::fs::create_dir_all(path).map_err(|source| StorageError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

/// Parse layout from env, ensure dirs exist.
pub fn open_storage() -> Result<StorageLayout, StorageError> {
    let layout = StorageLayout::from_env_or_default()?;
    layout.ensure_directories()?;
    Ok(layout)
}
