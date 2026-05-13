//! `glean daemon`: periodic reconcile driven by filesystem notifications.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::signal;

use glean_core::pipeline::{run_incremental_sync, DEFAULT_MAX_FILE_BYTES};
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

    let layout = open_storage().context("open storage root")?;
    let engine = GleanEngine::open(layout)
        .await
        .context("open glean engine")?;

    run_incremental_sync(engine.as_ref(), &workspace, DEFAULT_MAX_FILE_BYTES)
        .await
        .context("initial sync")?;

    let dirty = Arc::new(AtomicBool::new(false));
    let dirty_cb = Arc::clone(&dirty);

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if res.is_ok() {
                dirty_cb.store(true, Ordering::Relaxed);
            }
        },
        notify::Config::default(),
    )
    .context("notify watcher")?;

    watcher
        .watch(&workspace, RecursiveMode::Recursive)
        .context("watch workspace")?;

    tracing::info!(
        workspace = %workspace.display(),
        storage_root = %engine.layout().root.display(),
        "glean daemon watching workspace",
    );

    let mut tick = tokio::time::interval(Duration::from_millis(900));

    loop {
        tokio::select! {
            _ = shutdown_signal() => break,
            _ = tick.tick() => {
                if dirty.swap(false, Ordering::Relaxed) {
                    if let Err(e) =
                        run_incremental_sync(engine.as_ref(), &workspace, DEFAULT_MAX_FILE_BYTES).await
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
