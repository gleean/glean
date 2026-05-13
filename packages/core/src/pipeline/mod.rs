//! Workspace scanning + incremental application of [`crate::sync::SyncTask`] values.

use std::path::Path;

use walkdir::WalkDir;

use crate::engine::GleanEngine;
use crate::error::CoreError;
use crate::store::sqlite;
use crate::sync::{reconcile, DiskSnapshot, SyncTask};

/// Default per-file cap for MVP indexing (512 KiB).
pub const DEFAULT_MAX_FILE_BYTES: u64 = 512 * 1024;

fn should_skip_rel(rel: &Path) -> bool {
    rel.components().any(|c| {
        if let std::path::Component::Normal(name) = c {
            name == std::ffi::OsStr::new(".git") || name == std::ffi::OsStr::new("target")
        } else {
            false
        }
    })
}

fn normalize_path_key(rel: &Path) -> String {
    rel.to_string_lossy().replace('\\', "/")
}

fn file_mtime_ns(meta: &std::fs::Metadata) -> i64 {
    let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    match modified.duration_since(std::time::SystemTime::UNIX_EPOCH) {
        Ok(d) => {
            let n = d.as_secs() as i128 * 1_000_000_000 + d.subsec_nanos() as i128;
            n.clamp(i64::MIN as i128, i64::MAX as i128) as i64
        }
        Err(_) => 0,
    }
}

fn sha256_hex(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Build disk snapshots for UTF-8 files under `workspace_root`.
pub fn scan_workspace(
    workspace_root: &Path,
    max_file_bytes: u64,
) -> Result<Vec<DiskSnapshot>, CoreError> {
    let mut out = Vec::new();
    for entry in WalkDir::new(workspace_root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(workspace_root)
            .map_err(|_| CoreError::Msg("walk outside workspace".into()))?;
        if should_skip_rel(rel) {
            continue;
        }
        let meta = entry.metadata()?;
        if meta.len() > max_file_bytes {
            continue;
        }
        let bytes = std::fs::read(path)?;
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        let path_key = normalize_path_key(rel);
        let mtime_ns = file_mtime_ns(&meta);
        let hash = sha256_hex(&text);
        out.push(DiskSnapshot {
            path_key,
            mtime_ns,
            content_hash: hash,
        });
    }
    out.sort_by(|a, b| a.path_key.cmp(&b.path_key));
    Ok(out)
}

/// Scan workspace, reconcile against SQLite shadow rows, then apply resulting tasks.
pub async fn run_incremental_sync(
    engine: &GleanEngine,
    workspace_root: &Path,
    max_file_bytes: u64,
) -> Result<Vec<SyncTask>, CoreError> {
    let disk = scan_workspace(workspace_root, max_file_bytes)?;
    tracing::debug!(
        snapshot_entries = disk.len(),
        "workspace snapshot collected"
    );
    let db_rows = engine.with_sqlite(sqlite::load_all_meta)?;
    let tasks = reconcile(&disk, &db_rows);
    for t in &tasks {
        engine
            .apply_sync_task(workspace_root, t, max_file_bytes)
            .await?;
    }
    tracing::info!(
        applied_tasks = tasks.len(),
        "incremental sync batch applied"
    );
    Ok(tasks)
}
