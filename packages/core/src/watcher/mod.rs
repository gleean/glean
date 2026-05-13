//! Filesystem watcher: architecture **Watcher** — OS notifications → coarse dirty flag for daemon reconcile.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::CoreError;

/// Recursive watch on `workspace_root`; any OK notify event sets `dirty` so the daemon can debounce-sync.
pub fn install_recursive_workspace_watch(
    workspace_root: &Path,
    dirty: Arc<AtomicBool>,
) -> Result<RecommendedWatcher, CoreError> {
    let dirty_cb = Arc::clone(&dirty);
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if res.is_ok() {
                dirty_cb.store(true, Ordering::Relaxed);
            }
        },
        notify::Config::default(),
    )
    .map_err(|e| CoreError::Msg(format!("notify watcher: {e}")))?;

    watcher
        .watch(workspace_root, RecursiveMode::Recursive)
        .map_err(|e| CoreError::Msg(format!("notify watch: {e}")))?;
    Ok(watcher)
}
