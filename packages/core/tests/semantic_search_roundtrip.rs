//! Integration path for `GleanEngine::semantic_search` with `DeterministicEmbedder` and incremental sync.

use std::sync::Arc;

use glean_core::pipeline::{run_incremental_sync, DEFAULT_MAX_FILE_BYTES};
use glean_core::{DeterministicEmbedder, GleanEngine, StorageLayout};

#[tokio::test]
async fn semantic_search_returns_indexed_chunk_for_query() {
    let storage = tempfile::tempdir().expect("storage tmpdir");
    let workspace = tempfile::tempdir().expect("workspace tmpdir");

    let layout = StorageLayout::from_root(storage.path());
    layout.ensure_directories().expect("dirs");

    let engine = GleanEngine::open_with_embedder(layout, Arc::new(DeterministicEmbedder::new()))
        .await
        .expect("engine");

    std::fs::write(
        workspace.path().join("note.md"),
        "alpha beta gamma unique723",
    )
    .expect("write");

    run_incremental_sync(engine.as_ref(), workspace.path(), DEFAULT_MAX_FILE_BYTES)
        .await
        .expect("sync");

    let hits = engine
        .semantic_search("unique723", 10)
        .await
        .expect("semantic_search");
    assert!(
        hits.iter().any(|(_, text)| text.contains("unique723")),
        "expected semantic hit, got {hits:?}"
    );
}
