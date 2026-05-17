//! `glean daemon`: periodic reconcile driven by filesystem notifications.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal;

use glean_core::pipeline::run_incremental_sync;
use glean_core::{open_storage, GleanConfig, GleanEngine};

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut sigterm =
            signal::unix::signal(signal::unix::SignalKind::terminate()).expect("sigterm stream");
        tokio::select! {
            _ = signal::ctrl_c() => {},
            _ = sigterm.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        signal::ctrl_c().await.ok();
    }
}

/// Watch `workspace`, debounce via timer, and apply incremental sync tasks.
pub async fn run_daemon(workspace: Option<PathBuf>, runtime_config: GleanConfig) -> Result<()> {
    let workspace = workspace.unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    let workspace = workspace.canonicalize().unwrap_or(workspace);

    tracing::info!(
        workspace = %workspace.display(),
        "starting glean daemon",
    );

    let layout = open_storage().context("open storage root")?;
    tracing::info!(
        storage_root = %layout.root.display(),
        "opened storage layout",
    );

    let engine = GleanEngine::open_with_registry_and_config(
        layout,
        crate::parser_bootstrap::build_parser_registry(),
        runtime_config,
    )
    .await
    .context("open glean engine")?;
    tracing::info!("glean engine opened");

    let indexing = engine.runtime_config().indexing.clone();
    let (min_bytes, max_bytes) = indexing.sync_byte_limits();
    let poll_interval = indexing.watch_poll_interval();

    tracing::info!(
        min_file_bytes = min_bytes,
        max_file_bytes = max_bytes,
        watch_interval_secs = indexing.watch_interval,
        use_gitignore = indexing.use_gitignore,
        "indexing config applied",
    );

    tracing::info!("running initial incremental sync");
    run_incremental_sync(engine.as_ref(), &workspace)
        .await
        .context("initial sync")?;

    tracing::info!("initial incremental sync finished");

    let dirt = Arc::new(AtomicBool::new(false));

    tracing::info!("installing recursive workspace watcher");
    let _watcher_hold =
        glean_core::watcher::install_recursive_workspace_watch(&workspace, Arc::clone(&dirt))
            .context("notify watcher")?;

    let Some(poll) = poll_interval else {
        tracing::info!(
            workspace = %workspace.display(),
            storage_root = %engine.layout().root.display(),
            "watch_interval=0: periodic sync disabled; waiting for shutdown",
        );
        shutdown_signal().await;
        tracing::info!("glean daemon shutting down");
        return Ok(());
    };

    tracing::info!(
        workspace = %workspace.display(),
        storage_root = %engine.layout().root.display(),
        poll_interval_secs = poll.as_secs(),
        "glean daemon running; periodic sync enabled",
    );

    let mut tick = tokio::time::interval(poll);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = shutdown_signal() => break,
            _ = tick.tick() => {
                if dirt.swap(false, Ordering::Relaxed) {
                    if let Err(e) = run_incremental_sync(engine.as_ref(), &workspace).await {
                        tracing::error!(error = %e, "incremental sync failed");
                    }
                }
            }
        }
    }

    tracing::info!("glean daemon shutting down");
    Ok(())
}
