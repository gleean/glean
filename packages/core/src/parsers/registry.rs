use std::collections::HashMap;
use std::sync::Arc;

use super::{CommunityTextParser, DocumentParser};

/// Extension key (lowercase, no dot) → shared [`DocumentParser`] (Dispatcher/registry).
#[derive(Clone)]
pub struct ParserRegistry {
    by_extension: HashMap<String, Arc<dyn DocumentParser>>,
}

impl std::fmt::Debug for ParserRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParserRegistry")
            .field("extensions", &self.by_extension.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl ParserRegistry {
    /// Community parsers only. For enterprise augmentation, CLI builds registry via
    /// `glean-enterprise` and passes it to [`crate::engine::GleanEngine::open_with_registry`].
    pub fn with_builtins() -> Self {
        let mut reg = Self {
            by_extension: HashMap::new(),
        };
        reg.register(Arc::new(CommunityTextParser));
        reg
    }

    /// Register all [`DocumentParser::extensions`]; later wins on collision (per `file-system-rules.md`).
    pub fn register(&mut self, parser: Arc<dyn DocumentParser>) {
        for ext in parser.extensions() {
            self.by_extension
                .insert((*ext).to_ascii_lowercase(), Arc::clone(&parser));
        }
    }

    pub fn with_parser(mut self, parser: Arc<dyn DocumentParser>) -> Self {
        self.register(parser);
        self
    }

    pub fn parser_for_extension(&self, ext: &str) -> Option<&Arc<dyn DocumentParser>> {
        self.by_extension.get(ext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_resolves_rust_and_markdown() {
        let reg = ParserRegistry::with_builtins();
        assert!(reg.parser_for_extension("rs").is_some());
        assert!(reg.parser_for_extension("md").is_some());
        assert!(reg.parser_for_extension("unknown").is_none());
    }
}
