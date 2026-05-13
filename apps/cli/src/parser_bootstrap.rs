//! Builds the parser registry for this process: community plus optional **`glean-enterprise`**.

use std::sync::Arc;

use glean_core::parsers::ParserRegistry;

pub(crate) fn build_parser_registry() -> Arc<ParserRegistry> {
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
