//! Per-workspace index isolation under `<workspace>/.glean/`.

use std::sync::Arc;

use glean_core::pipeline::run_incremental_sync;
use glean_core::{DeterministicEmbedder, GleanEngine, GlobalLayout, WorkspaceIndexLayout};

#[tokio::test]
async fn two_workspaces_do_not_share_index_or_search_results() {
    let global_home = tempfile::tempdir().expect("global");
    let global = GlobalLayout::from_root(global_home.path());

    let ws_a = tempfile::tempdir().expect("ws_a");
    let ws_b = tempfile::tempdir().expect("ws_b");

    std::fs::write(
        ws_a.path().join("needle-a.txt"),
        "unique-token-alpha-needle",
    )
    .unwrap();
    std::fs::write(ws_b.path().join("needle-b.txt"), "unique-token-beta-needle").unwrap();

    let embedder = Arc::new(DeterministicEmbedder::new());

    let engine_a = GleanEngine::open_with_embedder_and_registry(
        WorkspaceIndexLayout::for_workspace(ws_a.path()),
        embedder.clone(),
        Arc::new(glean_core::parsers::ParserRegistry::with_builtins()),
    )
    .await
    .expect("open engine A");

    // Re-open with global for rerank paths (embedder test engine uses same root for both in open())
    let _ = global;

    run_incremental_sync(engine_a.as_ref(), ws_a.path())
        .await
        .expect("sync A");

    let engine_b = GleanEngine::open_with_embedder_and_registry(
        WorkspaceIndexLayout::for_workspace(ws_b.path()),
        embedder,
        Arc::new(glean_core::parsers::ParserRegistry::with_builtins()),
    )
    .await
    .expect("open engine B");

    run_incremental_sync(engine_b.as_ref(), ws_b.path())
        .await
        .expect("sync B");

    let hits_a = engine_a
        .semantic_search("unique-token-alpha-needle", 5)
        .await
        .expect("search A");
    assert!(
        hits_a.iter().any(|(p, _)| p.contains("needle-a")),
        "expected hits in workspace A, got {hits_a:?}"
    );
    assert!(
        !hits_a.iter().any(|(p, _)| p.contains("needle-b")),
        "workspace A search must not return B paths: {hits_a:?}"
    );

    let hits_b = engine_b
        .semantic_search("unique-token-beta-needle", 5)
        .await
        .expect("search B");
    assert!(
        hits_b.iter().any(|(p, _)| p.contains("needle-b")),
        "expected hits in workspace B, got {hits_b:?}"
    );
    assert!(
        !hits_b.iter().any(|(p, _)| p.contains("needle-a")),
        "workspace B search must not return A paths: {hits_b:?}"
    );

    assert!(WorkspaceIndexLayout::for_workspace(ws_a.path())
        .metadata_db_path()
        .is_file());
    assert!(WorkspaceIndexLayout::for_workspace(ws_b.path())
        .metadata_db_path()
        .is_file());
    assert_ne!(
        WorkspaceIndexLayout::for_workspace(ws_a.path()).root,
        WorkspaceIndexLayout::for_workspace(ws_b.path()).root
    );
}
