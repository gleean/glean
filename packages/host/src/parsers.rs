//! Parser registry assembly (community + optional enterprise).

use std::sync::Arc;

use glean_core::parsers::ParserRegistry;

/// Built-in community parsers, optionally augmented by `glean-enterprise` when feature `enterprise` is enabled.
pub fn build_default_registry() -> Arc<ParserRegistry> {
    #[cfg(not(feature = "enterprise"))]
    {
        Arc::new(ParserRegistry::with_builtins())
    }
    #[cfg(feature = "enterprise")]
    {
        let mut reg = ParserRegistry::with_builtins();
        glean_enterprise::augment_parser_registry(&mut reg);
        Arc::new(reg)
    }
}
