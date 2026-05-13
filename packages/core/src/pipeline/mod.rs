//! Workspace scanning + incremental application of [`crate::sync::SyncTask`] values.
//!
//! Gate order matches `file-system-rules.md` §5: blacklist → ignore → basename hidden →
//! executable suffix denylist → size → Dispatcher (`resolve_parser`) → parse probe.

pub mod dispatcher;
pub mod gates;
pub mod workspace_ignore;

use std::path::Path;

use walkdir::WalkDir;

use crate::engine::GleanEngine;
use crate::error::CoreError;
use crate::parsers::ParserRegistry;
use crate::store::sqlite;
use crate::sync::{reconcile, DiskSnapshot, SyncTask};

pub use gates::DEFAULT_MIN_FILE_BYTES;
pub use workspace_ignore::WorkspaceIgnore;

/// Default per-file cap for MVP indexing (512 KiB).
pub const DEFAULT_MAX_FILE_BYTES: u64 = 512 * 1024;

/// SHA-256 hex digest of raw file bytes (stable for UTF-8 text and future binary parsers).
pub fn sha256_bytes_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn extension_key(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
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

/// Build disk snapshots for files that pass global gates and Dispatcher (registry).
pub fn scan_workspace(
    workspace_root: &Path,
    workspace_ignore: &WorkspaceIgnore,
    registry: &ParserRegistry,
    min_file_bytes: u64,
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
        // 2. Path-component blacklist
        if gates::should_skip_path_components(rel) {
            continue;
        }
        // 3. .gitignore / .gleanignore
        if workspace_ignore.is_ignored(rel, false) {
            continue;
        }
        // 4. Basename hidden (`file-system-rules.md` §3.3)
        if gates::should_skip_hidden_file(rel) {
            continue;
        }
        // 5. Global executable / security suffix blocklist (before Dispatcher)
        let Some(ext) = extension_key(rel) else {
            continue;
        };
        if gates::is_blocked_executable_extension(&ext) {
            continue;
        }
        let meta = entry.metadata()?;
        let len = meta.len();
        // 6. Size bounds
        if gates::should_skip_by_size(len, min_file_bytes, max_file_bytes) {
            continue;
        }
        // 7–8. Dispatcher + parse probe
        let Some(parser) = dispatcher::resolve_parser(registry, &ext) else {
            continue;
        };
        let bytes = std::fs::read(path)?;
        if parser.parse_bytes(rel, &bytes).is_err() {
            continue;
        }
        let path_key = normalize_path_key(rel);
        let mtime_ns = file_mtime_ns(&meta);
        let hash = sha256_bytes_hex(&bytes);
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
    min_file_bytes: u64,
    max_file_bytes: u64,
) -> Result<Vec<SyncTask>, CoreError> {
    let workspace_ignore = WorkspaceIgnore::load(workspace_root)?;
    let disk = scan_workspace(
        workspace_root,
        &workspace_ignore,
        engine.parser_registry(),
        min_file_bytes,
        max_file_bytes,
    )?;
    tracing::debug!(
        snapshot_entries = disk.len(),
        "workspace snapshot collected"
    );
    let db_rows = engine.with_sqlite(sqlite::load_all_meta)?;
    let tasks = reconcile(&disk, &db_rows);
    for t in &tasks {
        engine
            .apply_sync_task(
                workspace_root,
                t,
                min_file_bytes,
                max_file_bytes,
                &workspace_ignore,
            )
            .await?;
    }
    tracing::info!(
        applied_tasks = tasks.len(),
        "incremental sync batch applied"
    );
    Ok(tasks)
}

#[cfg(test)]
mod scan_tests {
    use super::*;
    use crate::parsers::ParserRegistry;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[derive(Debug)]
    struct BadExeParser;

    impl crate::parsers::DocumentParser for BadExeParser {
        fn extensions(&self) -> &'static [&'static str] {
            &["exe"]
        }

        fn parse_bytes(
            &self,
            _rel_path: &std::path::Path,
            bytes: &[u8],
        ) -> Result<String, crate::parsers::ParseError> {
            Ok(String::from_utf8_lossy(bytes).into_owned())
        }
    }

    #[test]
    fn scan_indexes_only_registered_extensions() {
        let dir = tempdir().expect("tmpdir");
        let root = dir.path();
        std::fs::write(root.join("a.md"), b"hello world markdown content").unwrap();
        std::fs::write(root.join("b.bin"), b"hello world bin contentxxxxxxxx").unwrap();
        let ig = WorkspaceIgnore::load(root).unwrap();
        let reg = ParserRegistry::with_builtins();
        let out = scan_workspace(
            root,
            &ig,
            &reg,
            DEFAULT_MIN_FILE_BYTES,
            DEFAULT_MAX_FILE_BYTES,
        )
        .unwrap();
        let keys: Vec<&str> = out.iter().map(|d| d.path_key.as_str()).collect();
        assert!(keys.iter().any(|k| k.ends_with("a.md")), "keys={keys:?}");
        assert!(
            !keys.iter().any(|k| k.contains("b.bin")),
            "binary ext should be skipped without parser: keys={keys:?}"
        );
    }

    #[test]
    fn scan_respects_gleanignore_directory() {
        let dir = tempdir().expect("tmpdir");
        let root = dir.path();
        std::fs::create_dir(root.join("skipme")).unwrap();
        std::fs::write(root.join(".gleanignore"), "skipme/\n").unwrap();
        std::fs::write(
            root.join("skipme").join("bad.md"),
            b"xxxxxxxxxxxxxxxxxxxxxxxx",
        )
        .unwrap();
        std::fs::write(root.join("good.md"), b"yyyyyyyyyyyyyyyyyyyyyyyyyy").unwrap();
        let ig = WorkspaceIgnore::load(root).unwrap();
        let reg = ParserRegistry::with_builtins();
        let out = scan_workspace(
            root,
            &ig,
            &reg,
            DEFAULT_MIN_FILE_BYTES,
            DEFAULT_MAX_FILE_BYTES,
        )
        .unwrap();
        let keys: Vec<&str> = out.iter().map(|d| d.path_key.as_str()).collect();
        assert!(
            keys.iter().any(|k| k.ends_with("good.md")),
            "good.md must index: {keys:?}"
        );
        assert!(
            !keys.iter().any(|k| k.contains("skipme")),
            "skipme must be excluded: {keys:?}"
        );
    }

    #[test]
    fn blocked_exe_skips_even_when_parser_registered() {
        let dir = tempdir().expect("tmpdir");
        let root = dir.path();
        std::fs::write(root.join("evil.exe"), b"zzzzzzzzzzzzzzzzzzzzzzz").unwrap();
        let ig = WorkspaceIgnore::load(root).unwrap();
        let reg = ParserRegistry::with_builtins().with_parser(Arc::new(BadExeParser));
        let out = scan_workspace(
            root,
            &ig,
            &reg,
            DEFAULT_MIN_FILE_BYTES,
            DEFAULT_MAX_FILE_BYTES,
        )
        .unwrap();
        assert!(
            out.is_empty(),
            "executable suffix blocked before Dispatcher: {:?}",
            out
        );
    }
}
