//! Host runtime for CLI and desktop shells (no `main`).
//!
//! Daemon loops, MCP JSON-RPC routing, config editing, and status aggregation live here.
//! Indexing engine primitives remain in `glean-core`.

#![forbid(unsafe_code)]

mod error;

pub mod config;
pub mod daemon;
pub mod mcp;
pub mod parsers;
pub mod status;
pub mod workspace;

pub use error::HostError;
pub use glean_core::{CoreError, GleanConfig, GleanEngine, VERSION};
