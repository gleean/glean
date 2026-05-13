//! Integration: deleting a file after indexing triggers Lance + SQLite purge.

use std::sync::Arc;

use glean_core::pipeline::{run_incremental_sync, DEFAULT_MAX_FILE_BYTES};
use glean_core::{DeterministicEmbedder, GleanEngine, StorageLayout};

#[tokio::test]
async fn deleting_workspace_file_purges_index_entry() {
    let storage = tempfile::tempdir().expect("storage tmpdir");
    let workspace = tempfile::tempdir().expect("workspace tmpdir");

    let layout = StorageLayout::from_root(storage.path());
    layout.ensure_directories().expect("dirs");

    let engine = GleanEngine::open_with_embedder(layout, Arc::new(DeterministicEmbedder::new()))
        .await
        .expect("engine");

    let path_txt = workspace.path().join("doc.txt");
    std::fs::write(&path_txt, "uniquepurge789 marker").expect("write");

    run_incremental_sync(engine.as_ref(), workspace.path(), DEFAULT_MAX_FILE_BYTES)
        .await
        .expect("sync after write");

    let hits_before = engine
        .semantic_search("uniquepurge789", 10)
        .await
        .expect("search before purge");
    assert!(
        !hits_before.is_empty(),
        "expected at least one chunk hit before purge"
    );

    std::fs::remove_file(&path_txt).expect("remove");

    run_incremental_sync(engine.as_ref(), workspace.path(), DEFAULT_MAX_FILE_BYTES)
        .await
        .expect("sync after delete");

    let hits_after = engine
        .semantic_search("uniquepurge789", 10)
        .await
        .expect("search after purge");
    assert!(
        hits_after.is_empty(),
        "expected no chunks after file purge, got {hits_after:?}"
    );
}
