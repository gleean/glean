//! `glean daemon`: open engine and run the host daemon loop.

use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

use glean_core::{open_global, GleanConfig, GleanEngine};
use glean_host::daemon::{run_daemon_loop, DaemonRunOptions};

/// Watch `workspace`, debounce via timer, and apply incremental sync tasks.
pub async fn run_daemon(workspace: Option<PathBuf>, runtime_config: GleanConfig) -> Result<()> {
    let workspace =
        glean_host::workspace::resolve_workspace_root(workspace).map_err(|e| anyhow::anyhow!(e))?;

    tracing::info!(
        workspace = %workspace.display(),
        "starting glean daemon",
    );

    let global = open_global().context("open GLEAN_STORAGE_ROOT")?;
    tracing::info!(
        storage_root = %global.root.display(),
        "opened global storage layout",
    );

    let engine = GleanEngine::open_for_workspace(
        &workspace,
        global,
        glean_host::parsers::build_default_registry(),
        runtime_config,
    )
    .await
    .context("open glean engine")?;

    tracing::info!(
        index_root = %engine.index_layout().root.display(),
        "workspace index layout",
    );

    let cancel = CancellationToken::new();
    let cancel_child = cancel.clone();

    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => cancel_child.cancel(),
                    _ = sigterm.recv() => cancel_child.cancel(),
                }
                return;
            }
        }
        let _ = tokio::signal::ctrl_c().await;
        cancel_child.cancel();
    });

    run_daemon_loop(DaemonRunOptions {
        engine,
        workspace,
        cancel,
    })
    .await
    .map_err(|e| anyhow::anyhow!(e))
}
