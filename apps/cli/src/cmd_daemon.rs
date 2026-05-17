//! `glean daemon`: periodic reconcile driven by filesystem notifications.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::signal;
use tokio::time::MissedTickBehavior;

use glean_core::pipeline::run_incremental_sync;
use glean_core::{open_global, GleanConfig, GleanEngine};

const CONFIG_RELOAD_POLL_SECS: u64 = 2;

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

struct ConfigMtimeWatch {
    global: Option<SystemTime>,
}

impl ConfigMtimeWatch {
    fn new(global_path: &Path) -> Self {
        Self {
            global: file_mtime(global_path),
        }
    }

    fn changed(&mut self, global_path: &Path) -> bool {
        let g = file_mtime(global_path);
        let changed = g != self.global;
        self.global = g;
        changed
    }
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

fn reload_daemon_config(engine: &GleanEngine) -> Result<GleanConfig, anyhow::Error> {
    let cfg = GleanConfig::load_merged_with_global(engine.global_layout())
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    engine
        .reload_runtime_config(cfg.clone())
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(cfg)
}

/// Watch `workspace`, debounce via timer, and apply incremental sync tasks.
pub async fn run_daemon(workspace: Option<PathBuf>, runtime_config: GleanConfig) -> Result<()> {
    let workspace = workspace.unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    let workspace = workspace.canonicalize().unwrap_or(workspace);

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
        crate::parser_bootstrap::build_parser_registry(),
        runtime_config,
    )
    .await
    .context("open glean engine")?;

    tracing::info!(
        index_root = %engine.index_layout().root.display(),
        "workspace index layout",
    );

    let global_cfg_path = GleanConfig::global_config_watch_path(engine.global_layout());
    let mut cfg_watch = ConfigMtimeWatch::new(&global_cfg_path);

    let mut indexing = engine.runtime_config().indexing.clone();
    let (min_bytes, max_bytes) = indexing.sync_byte_limits();
    let mut poll_duration = indexing.watch_poll_interval();
    let mut sync_tick = poll_duration.map(|poll| {
        let mut tick = tokio::time::interval(poll);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        tick
    });

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

    let mut config_tick =
        tokio::time::interval(std::time::Duration::from_secs(CONFIG_RELOAD_POLL_SECS));
    config_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    if poll_duration.is_none() {
        tracing::info!(
            workspace = %workspace.display(),
            index_root = %engine.index_layout().root.display(),
            "watch_interval=0: periodic sync disabled; config hot-reload active",
        );
    } else if let Some(poll) = poll_duration {
        tracing::info!(
            workspace = %workspace.display(),
            index_root = %engine.index_layout().root.display(),
            poll_interval_secs = poll.as_secs(),
            "glean daemon running; periodic sync enabled",
        );
    }

    loop {
        if let Some(tick) = sync_tick.as_mut() {
            tokio::select! {
                _ = shutdown_signal() => break,
                _ = config_tick.tick() => {
                    if cfg_watch.changed(&global_cfg_path) {
                        let old_interval = indexing.watch_interval;
                        let new_cfg = reload_daemon_config(engine.as_ref())?;
                        indexing = new_cfg.indexing.clone();
                        if new_cfg.indexing.watch_interval != old_interval {
                            poll_duration = new_cfg.indexing.watch_poll_interval();
                            sync_tick = poll_duration.map(|poll| {
                                let mut t = tokio::time::interval(poll);
                                t.set_missed_tick_behavior(MissedTickBehavior::Skip);
                                t
                            });
                            tracing::info!(
                                watch_interval_secs = new_cfg.indexing.watch_interval,
                                periodic_sync = poll_duration.is_some(),
                                "reloaded indexing.watch_interval from config.toml",
                            );
                        } else {
                            tracing::info!("reloaded config.toml");
                        }
                    }
                }
                _ = tick.tick() => {
                    if dirt.swap(false, Ordering::Relaxed) {
                        if let Err(e) = run_incremental_sync(engine.as_ref(), &workspace).await {
                            tracing::error!(error = %e, "incremental sync failed");
                        }
                    }
                }
            }
        } else {
            tokio::select! {
                _ = shutdown_signal() => break,
                _ = config_tick.tick() => {
                    if cfg_watch.changed(&global_cfg_path) {
                        let old_interval = indexing.watch_interval;
                        let new_cfg = reload_daemon_config(engine.as_ref())?;
                        indexing = new_cfg.indexing.clone();
                        if new_cfg.indexing.watch_interval != old_interval {
                            poll_duration = new_cfg.indexing.watch_poll_interval();
                            sync_tick = poll_duration.map(|poll| {
                                let mut t = tokio::time::interval(poll);
                                t.set_missed_tick_behavior(MissedTickBehavior::Skip);
                                t
                            });
                            tracing::info!(
                                watch_interval_secs = new_cfg.indexing.watch_interval,
                                periodic_sync = poll_duration.is_some(),
                                "reloaded indexing.watch_interval from config.toml",
                            );
                        } else {
                            tracing::info!("reloaded config.toml");
                        }
                    }
                }
            }
        }
    }

    tracing::info!("glean daemon shutting down");
    Ok(())
}
