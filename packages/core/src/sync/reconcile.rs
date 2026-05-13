//! Pure reconciliation between disk snapshots and SQLite shadow rows.

/// Single observed file on disk (normalized relative path key within workspace).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskSnapshot {
    pub path_key: String,
    pub mtime_ns: i64,
    pub content_hash: String,
}

/// Row mirrored from `file_meta`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbSnapshot {
    pub path_key: String,
    pub mtime_ns: i64,
    pub content_hash: String,
    pub indexed_version: i64,
    pub safety_lock: bool,
}

/// Actions derived from reconciliation (applied later by the pipeline).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncTask {
    /// Index or re-index file content into Lance + SQLite row update.
    Upsert { path_key: String },
    /// File disappeared from disk — drop Lance chunks + DB row.
    Purge { path_key: String },
    /// Row is locked — skip mutation until lock clears.
    SkipLocked { path_key: String },
}

/// Compute sync tasks with deterministic ordering (purge → upsert → skip).
pub fn reconcile(disk: &[DiskSnapshot], db: &[DbSnapshot]) -> Vec<SyncTask> {
    use std::collections::BTreeMap;

    let disk_map: BTreeMap<&str, &DiskSnapshot> =
        disk.iter().map(|d| (d.path_key.as_str(), d)).collect();
    let db_map: BTreeMap<&str, &DbSnapshot> = db.iter().map(|r| (r.path_key.as_str(), r)).collect();

    let mut tasks = Vec::new();

    // Paths only on disk or changed → Upsert (unless locked).
    for (&path_key, d) in &disk_map {
        match db_map.get(path_key) {
            None => tasks.push(SyncTask::Upsert {
                path_key: path_key.to_string(),
            }),
            Some(row) => {
                if row.safety_lock {
                    if row.mtime_ns != d.mtime_ns || row.content_hash != d.content_hash {
                        tasks.push(SyncTask::SkipLocked {
                            path_key: path_key.to_string(),
                        });
                    }
                    continue;
                }
                if row.mtime_ns != d.mtime_ns || row.content_hash != d.content_hash {
                    tasks.push(SyncTask::Upsert {
                        path_key: path_key.to_string(),
                    });
                }
            }
        }
    }

    // Paths only in DB → Purge (disk deletion beats stale locks).
    for &path_key in db_map.keys() {
        if !disk_map.contains_key(path_key) {
            tasks.push(SyncTask::Purge {
                path_key: path_key.to_string(),
            });
        }
    }

    let mut purges = Vec::new();
    let mut upserts = Vec::new();
    let mut skips = Vec::new();
    for t in tasks {
        match t {
            SyncTask::Purge { path_key } => purges.push(path_key),
            SyncTask::Upsert { path_key } => upserts.push(path_key),
            SyncTask::SkipLocked { path_key } => skips.push(path_key),
        }
    }
    purges.sort();
    upserts.sort();
    skips.sort();

    purges
        .into_iter()
        .map(|path_key| SyncTask::Purge { path_key })
        .chain(
            upserts
                .into_iter()
                .map(|path_key| SyncTask::Upsert { path_key }),
        )
        .chain(
            skips
                .into_iter()
                .map(|path_key| SyncTask::SkipLocked { path_key }),
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(path: &str, mtime: i64, hash: &str) -> DiskSnapshot {
        DiskSnapshot {
            path_key: path.to_string(),
            mtime_ns: mtime,
            content_hash: hash.to_string(),
        }
    }

    fn r(path: &str, mtime: i64, hash: &str, lock: bool) -> DbSnapshot {
        DbSnapshot {
            path_key: path.to_string(),
            mtime_ns: mtime,
            content_hash: hash.to_string(),
            indexed_version: 1,
            safety_lock: lock,
        }
    }

    #[test]
    fn new_file_on_disk_triggers_upsert() {
        let disk = vec![d("a.txt", 1, "h1")];
        let db = vec![];
        assert_eq!(
            reconcile(&disk, &db),
            vec![SyncTask::Upsert {
                path_key: "a.txt".into()
            }]
        );
    }

    #[test]
    fn deleted_file_triggers_purge() {
        let disk = vec![];
        let db = vec![r("gone.txt", 1, "h", false)];
        assert_eq!(
            reconcile(&disk, &db),
            vec![SyncTask::Purge {
                path_key: "gone.txt".into()
            }]
        );
    }

    #[test]
    fn modified_file_triggers_upsert() {
        let disk = vec![d("a.txt", 2, "h2")];
        let db = vec![r("a.txt", 1, "h1", false)];
        assert_eq!(
            reconcile(&disk, &db),
            vec![SyncTask::Upsert {
                path_key: "a.txt".into()
            }]
        );
    }

    #[test]
    fn unchanged_no_tasks() {
        let disk = vec![d("a.txt", 1, "h")];
        let db = vec![r("a.txt", 1, "h", false)];
        assert!(reconcile(&disk, &db).is_empty());
    }

    #[test]
    fn safety_lock_skips_drift_but_purge_on_delete() {
        let disk = vec![];
        let db = vec![r("locked.txt", 1, "old", true)];
        assert_eq!(
            reconcile(&disk, &db),
            vec![SyncTask::Purge {
                path_key: "locked.txt".into()
            }]
        );

        let disk = vec![d("locked.txt", 9, "new")];
        let db = vec![r("locked.txt", 1, "old", true)];
        assert_eq!(
            reconcile(&disk, &db),
            vec![SyncTask::SkipLocked {
                path_key: "locked.txt".into()
            }]
        );
    }
}
