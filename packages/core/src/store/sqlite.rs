//! SQLite shadow metadata (`file_meta`).

use std::path::Path;

use rusqlite::{params, Connection};

use crate::error::CoreError;
use crate::sync::DbSnapshot;

/// Apply schema migrations (idempotent).
pub fn migrate(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS file_meta (
            path_key TEXT PRIMARY KEY NOT NULL,
            mtime_ns INTEGER NOT NULL,
            content_hash TEXT NOT NULL,
            indexed_version INTEGER NOT NULL DEFAULT 1,
            safety_lock INTEGER NOT NULL DEFAULT 0
        );
        "#,
    )?;
    Ok(())
}

/// Open SQLite at `path`, run migrations.
pub fn open_conn(path: &Path) -> Result<Connection, CoreError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    migrate(&conn)?;
    Ok(conn)
}

/// Load all rows for reconciliation / listing.
pub fn load_all_meta(conn: &Connection) -> Result<Vec<DbSnapshot>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT path_key, mtime_ns, content_hash, indexed_version, safety_lock FROM file_meta",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(DbSnapshot {
            path_key: row.get(0)?,
            mtime_ns: row.get(1)?,
            content_hash: row.get(2)?,
            indexed_version: row.get(3)?,
            safety_lock: row.get::<_, i64>(4)? != 0,
        })
    })?;
    rows.collect()
}

pub fn upsert_file_meta(
    conn: &Connection,
    path_key: &str,
    mtime_ns: i64,
    content_hash: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        r#"
        INSERT INTO file_meta (path_key, mtime_ns, content_hash, indexed_version, safety_lock)
        VALUES (?1, ?2, ?3, 1, 0)
        ON CONFLICT(path_key) DO UPDATE SET
          mtime_ns = excluded.mtime_ns,
          content_hash = excluded.content_hash,
          indexed_version = indexed_version + 1,
          safety_lock = 0
        "#,
        params![path_key, mtime_ns, content_hash],
    )?;
    Ok(())
}

pub fn delete_file_meta(conn: &Connection, path_key: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM file_meta WHERE path_key = ?1",
        params![path_key],
    )?;
    Ok(())
}

/// Recent changes ordered by `mtime_ns` descending.
pub fn recent_changes(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<(String, i64)>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT path_key, mtime_ns FROM file_meta ORDER BY mtime_ns DESC LIMIT ?1")?;
    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    rows.collect()
}
