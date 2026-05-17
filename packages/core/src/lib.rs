//! Glean core engine library (no binary entrypoint in this crate).
//!
//! Watcher, LanceDB, parsers, and related modules will be filled in iteratively.

#![forbid(unsafe_code)]

mod digest_util;

pub mod chunk;
pub mod config;
pub mod embed;
pub mod engine;
pub mod error;
pub mod parsers;
pub mod pipeline;
pub mod storage;
pub mod store;
pub mod sync;
pub mod watcher;

pub use config::{ConfigLayer, GleanConfig};
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
