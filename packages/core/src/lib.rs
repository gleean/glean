//! Glean core engine library (no binary entrypoint in this crate).
//!
//! Watcher, LanceDB, parsers, and related modules will be filled in iteratively.

#![forbid(unsafe_code)]

pub mod chunk;
pub mod embed;
pub mod engine;
pub mod error;
pub mod pipeline;
pub mod storage;
pub mod store;
pub mod sync;

pub use embed::{DeterministicEmbedder, Embedder};
pub use engine::GleanEngine;
pub use error::{CoreError, StorageError};
pub use storage::{open_storage, StorageLayout};

/// Public version string surfaced via CLI / MCP `initialize`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Crate version as a function (handy for dynamic formatting / FFI boundaries).
#[inline]
pub fn version() -> &'static str {
    VERSION
}
