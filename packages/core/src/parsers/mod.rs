//! `DocumentParser` trait and registry (`architecture.md` / `file-system-rules.md`).
//!
//! Global gates live in [`crate::pipeline::gates`]; extension resolution goes through
//! [`crate::pipeline::dispatcher`].

mod community_text;
mod registry;

pub use community_text::CommunityTextParser;
pub use registry::ParserRegistry;

use std::path::Path;

use thiserror::Error;

/// Parsing bytes into indexable plain text failed for this [`DocumentParser`].
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("plain-text parser requires valid UTF-8")]
    NotUtf8,
}

/// Community or enterprise parser: declares extensions and decodes bytes to text for chunking/embed.
///
/// MVP: synchronous `parse_bytes`; async/chunk-producing variants can evolve per docs.
pub trait DocumentParser: Send + Sync {
    /// Lowercase extensions without dot (e.g. `"md"`).
    fn extensions(&self) -> &'static [&'static str];

    /// Raw file bytes → text consumed by downstream chunking.
    fn parse_bytes(&self, _rel_path: &Path, bytes: &[u8]) -> Result<String, ParseError>;
}
