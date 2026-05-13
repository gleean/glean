use std::path::Path;

use super::{DocumentParser, ParseError};

/// Community UTF-8 parser façade: one impl, many extensions (Markdown, code, config, markup).
#[derive(Debug, Default)]
pub struct CommunityTextParser;

impl DocumentParser for CommunityTextParser {
    fn extensions(&self) -> &'static [&'static str] {
        &[
            "md", "markdown", "txt", "rs", "toml", "json", "yaml", "yml", "css", "html", "htm",
            "jsx", "tsx", "ts", "js", "mjs", "cjs", "py", "go", "cpp", "cxx", "cc", "h", "hpp",
            "c", "java", "kt", "swift", "rb", "php", "sh", "bash", "zsh", "ps1", "sql", "vue",
            "svelte", "conf", "ini", "xml",
        ]
    }

    fn parse_bytes(&self, _rel_path: &Path, bytes: &[u8]) -> Result<String, ParseError> {
        std::str::from_utf8(bytes)
            .map(String::from)
            .map_err(|_| ParseError::NotUtf8)
    }
}
