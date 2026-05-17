//! Host-layer errors (config I/O, JSON-RPC framing, workspace resolution).

use glean_core::{CoreError, StorageError};

#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error(transparent)]
    Core(#[from] CoreError),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error("{0}")]
    Message(String),

    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("TOML: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
}

impl HostError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}
