//! Errors surfaced by `glean-core`.

use std::path::PathBuf;

/// Top-level error type for storage / indexing operations.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("storage layout error: {0}")]
    Storage(#[from] StorageError),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("LanceDB error: {0}")]
    Lance(String),

    #[error("LanceDB schema mismatch: {detail}")]
    LanceSchemaMismatch { detail: String },

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Arrow error: {0}")]
    Arrow(String),

    #[error("invalid UTF-8 path")]
    NonUtf8Path,

    #[error("path forbidden (outside workspace root): {0}")]
    PathForbidden(PathBuf),

    #[error("file too large (limit {limit} bytes): {path}")]
    FileTooLarge { path: PathBuf, limit: u64 },

    #[error("invalid config TOML at {}: {message}", path.display())]
    InvalidConfigToml { path: PathBuf, message: String },

    #[error("{0}")]
    Msg(String),
}

impl From<arrow_schema::ArrowError> for CoreError {
    fn from(e: arrow_schema::ArrowError) -> Self {
        CoreError::Arrow(e.to_string())
    }
}

impl From<walkdir::Error> for CoreError {
    fn from(e: walkdir::Error) -> Self {
        let msg = e.to_string();
        match e.into_io_error() {
            Some(ioe) => CoreError::Io(ioe),
            None => CoreError::Msg(msg),
        }
    }
}

impl From<ignore::Error> for CoreError {
    fn from(e: ignore::Error) -> Self {
        CoreError::Msg(e.to_string())
    }
}

/// Errors opening or preparing the on-disk storage layout.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("GLEAN_STORAGE_ROOT must be non-empty")]
    EmptyRoot,

    #[error("failed to resolve default storage directory")]
    NoHomeDir,

    #[error("failed to create directory `{}`: {source}", path.display())]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
