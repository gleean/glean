//! Dispatcher: resolve extension to [`DocumentParser`] — architecture “调度器” 的查表实现。

use std::sync::Arc;

use crate::parsers::{DocumentParser, ParserRegistry};

/// Look up a parser for a lowercase extension (no dot). None ⇒ skip file (no error).
#[inline]
pub fn resolve_parser<'a>(
    registry: &'a ParserRegistry,
    ext: &str,
) -> Option<&'a Arc<dyn DocumentParser>> {
    registry.parser_for_extension(ext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::CommunityTextParser;

    #[test]
    fn resolve_matches_registry() {
        let reg = ParserRegistry::with_builtins();
        assert!(resolve_parser(&reg, "md").is_some());
        assert!(resolve_parser(&reg, "nope").is_none());
        let reg2 = ParserRegistry::with_builtins().with_parser(Arc::new(CommunityTextParser));
        assert_eq!(
            resolve_parser(&reg2, "md").is_some(),
            reg2.parser_for_extension("md").is_some()
        );
    }
}
