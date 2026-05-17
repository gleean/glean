//! Regression: merged `GleanConfig` survives `GleanEngine::open_with_registry_and_config`.

use std::sync::Arc;

use glean_core::parsers::ParserRegistry;
use glean_core::{GleanConfig, GleanEngine, GlobalLayout, WorkspaceIndexLayout};

#[tokio::test]
async fn engine_exposes_snapshot_from_open_with_registry_and_config() {
    let mut cfg = GleanConfig::default();
    cfg.log.level = "warn".into();
    cfg.rerank.enabled = true;
    cfg.rerank.top_k = 42;
    cfg.indexing.watch_interval = 777;

    let storage = tempfile::tempdir().expect("storage tmpdir");
    let global = GlobalLayout::from_root(storage.path());
    let index = WorkspaceIndexLayout::from_root(storage.path());
    index.ensure_directories().expect("ensure dirs");

    let engine = GleanEngine::open_with_registry_and_config(
        index,
        global,
        Arc::new(ParserRegistry::with_builtins()),
        cfg,
    )
    .await
    .expect("open engine");

    let rc = engine.runtime_config();
    assert_eq!(rc.log.level, "warn");
    assert!(rc.rerank.enabled);
    assert_eq!(rc.rerank.top_k, 42);
    assert_eq!(rc.indexing.watch_interval, 777);
}

#[tokio::test]
async fn reload_runtime_config_updates_engine_snapshot() {
    let mut cfg = GleanConfig::default();
    cfg.log.level = "info".into();

    let storage = tempfile::tempdir().expect("storage tmpdir");
    let global = GlobalLayout::from_root(storage.path());
    let index = WorkspaceIndexLayout::from_root(storage.path());
    index.ensure_directories().expect("ensure dirs");

    let engine = GleanEngine::open_with_registry_and_config(
        index,
        global,
        Arc::new(ParserRegistry::with_builtins()),
        cfg,
    )
    .await
    .expect("open engine");

    assert_eq!(engine.runtime_config().log.level, "info");

    let mut next = engine.runtime_config().clone();
    next.log.level = "trace".into();
    engine
        .reload_runtime_config(next)
        .expect("reload runtime config");

    assert_eq!(engine.runtime_config().log.level, "trace");
}
