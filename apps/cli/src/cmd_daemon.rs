//! `glean daemon`: periodic reconcile driven by filesystem notifications.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::signal;

use glean_core::pipeline::{run_incremental_sync, DEFAULT_MAX_FILE_BYTES, DEFAULT_MIN_FILE_BYTES};
use glean_core::{open_storage, GleanEngine};

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
pub async fn run_daemon(workspace: Option<PathBuf>) -> Result<()> {
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

    let engine =
        GleanEngine::open_with_registry(layout, crate::parser_bootstrap::build_parser_registry())
            .await
            .context("open glean engine")?;
    tracing::info!("glean engine opened");

    tracing::info!("running initial incremental sync");
    run_incremental_sync(
        engine.as_ref(),
        &workspace,
        DEFAULT_MIN_FILE_BYTES,
        DEFAULT_MAX_FILE_BYTES,
    )
    .await
    .context("initial sync")?;

    tracing::info!("initial incremental sync finished");

    let dirt = Arc::new(AtomicBool::new(false));

    tracing::info!("installing recursive workspace watcher");
    let _watcher_hold =
        glean_core::watcher::install_recursive_workspace_watch(&workspace, Arc::clone(&dirt))
            .context("notify watcher")?;

    tracing::info!(
        workspace = %workspace.display(),
        storage_root = %engine.layout().root.display(),
        debounce_ms = 900_u64,
        "glean daemon running; periodic sync enabled",
    );

    let mut tick = tokio::time::interval(Duration::from_millis(900));

    loop {
        tokio::select! {
            _ = shutdown_signal() => break,
            _ = tick.tick() => {
                if dirt.swap(false, Ordering::Relaxed) {
                    if let Err(e) =
                        run_incremental_sync(
                            engine.as_ref(),
                            &workspace,
                            DEFAULT_MIN_FILE_BYTES,
                            DEFAULT_MAX_FILE_BYTES,
                        )
                            .await
                    {
                        tracing::error!(error = %e, "incremental sync failed");
                    }
                }
            }
        }
    }

    tracing::info!("glean daemon shutting down");
    Ok(())
}
