//! Shared runtime wiring SQLite + LanceDB for CLI / MCP.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::chunk::chunk_plain_text_markdown_mvp;
use crate::config::GleanConfig;
use crate::embed::Embedder;
use crate::error::CoreError;
use crate::parsers::ParserRegistry;
use crate::storage::StorageLayout;
use crate::store::{lance_chunks, sqlite};

/// Primary engine handle (opened once per process).
pub struct GleanEngine {
    layout: StorageLayout,
    sqlite: std::sync::Mutex<Connection>,
    lance: Mutex<lancedb::Connection>,
    /// `None`: lazily initialize the default backend on first embed (avoids ONNX fetch during MCP handshake-only runs).
    embedder_slot: std::sync::Mutex<Option<Arc<dyn Embedder>>>,
    /// Registered file parsers (extension → implementation); builtins cover UTF-8 text types.
    parsers: Arc<ParserRegistry>,
    /// Merged TOML + defaults; controls optional features such as stub rerank hooks.
    runtime_config: GleanConfig,
}

impl GleanEngine {
    /// Prepare metadata DB, Lance dataset, and tables (default embedder loads lazily).
    ///
    /// Uses the built-in community [`ParserRegistry`] only. To load `packages/enterprise`, build
    /// the CLI with `--features enterprise` and use [`Self::open_with_registry`].
    pub async fn open(layout: StorageLayout) -> Result<Arc<Self>, CoreError> {
        Self::open_inner(
            layout,
            None,
            Arc::new(ParserRegistry::with_builtins()),
            None,
        )
        .await
    }

    /// Inject an embedder (integration tests should prefer [`crate::DeterministicEmbedder`]).
    pub async fn open_with_embedder(
        layout: StorageLayout,
        embedder: Arc<dyn Embedder>,
    ) -> Result<Arc<Self>, CoreError> {
        Self::open_inner(
            layout,
            Some(embedder),
            Arc::new(ParserRegistry::with_builtins()),
            None,
        )
        .await
    }

    /// Open engine with a caller-built parser registry (community + optional enterprise).
    pub async fn open_with_registry(
        layout: StorageLayout,
        parsers: Arc<ParserRegistry>,
    ) -> Result<Arc<Self>, CoreError> {
        Self::open_inner(layout, None, parsers, None).await
    }

    /// Same as [`Self::open_with_registry`] with an explicit merged [`GleanConfig`].
    pub async fn open_with_registry_and_config(
        layout: StorageLayout,
        parsers: Arc<ParserRegistry>,
        runtime_config: GleanConfig,
    ) -> Result<Arc<Self>, CoreError> {
        Self::open_inner(layout, None, parsers, Some(runtime_config)).await
    }

    /// Open with custom embedder and custom parser registry.
    pub async fn open_with_embedder_and_registry(
        layout: StorageLayout,
        embedder: Arc<dyn Embedder>,
        parsers: Arc<ParserRegistry>,
    ) -> Result<Arc<Self>, CoreError> {
        Self::open_inner(layout, Some(embedder), parsers, None).await
    }

    async fn open_inner(
        layout: StorageLayout,
        embedder: Option<Arc<dyn Embedder>>,
        parsers: Arc<ParserRegistry>,
        runtime_config: Option<GleanConfig>,
    ) -> Result<Arc<Self>, CoreError> {
        layout.ensure_directories()?;
        let sqlite_path = layout.metadata_db_path();
        let conn = sqlite::open_conn(&sqlite_path)?;

        let uri = layout.lancedb_uri();
        let uri_str = uri.to_string_lossy().to_string();
        let lance = lancedb::connect(&uri_str)
            .execute()
            .await
            .map_err(|e| CoreError::Lance(e.to_string()))?;
        lance_chunks::ensure_document_chunks_table(&lance).await?;

        Ok(Arc::new(Self {
            layout,
            sqlite: std::sync::Mutex::new(conn),
            lance: Mutex::new(lance),
            embedder_slot: std::sync::Mutex::new(embedder),
            parsers,
            runtime_config: runtime_config.unwrap_or_default(),
        }))
    }

    fn embedder_or_init(&self) -> Result<Arc<dyn Embedder>, CoreError> {
        let mut slot = self
            .embedder_slot
            .lock()
            .map_err(|_| CoreError::Msg("embedder mutex poisoned".into()))?;
        if slot.is_none() {
            *slot = Some(crate::embed::default_embedder(
                &self.runtime_config.embedding,
            )?);
        }
        Ok(slot
            .as_ref()
            .ok_or_else(|| CoreError::Msg("embedder missing after lazy init".into()))?
            .clone())
    }

    pub fn layout(&self) -> &StorageLayout {
        &self.layout
    }

    /// Parser registry used by workspace scans and incremental upserts.
    pub fn parser_registry(&self) -> &ParserRegistry {
        &self.parsers
    }

    /// Effective merged configuration for this engine instance.
    pub fn runtime_config(&self) -> &GleanConfig {
        &self.runtime_config
    }

    /// Hybrid retrieval (BM25 on `text` + vector kNN, RRF) when FTS index exists; falls back to vector-only if hybrid fails.
    pub async fn semantic_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(String, String)>, CoreError> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let embedder = self.embedder_or_init()?;
        let query_vec = embedder.embed_query(q)?;
        let db = self.lance.lock().await;
        let hits = lance_chunks::semantic_search_chunks(&db, &query_vec, q, limit).await?;
        drop(db);
        crate::pipeline::reranker::apply_cross_encoder_rerank(&self.runtime_config.rerank, q, hits)
    }

    /// Recent indexed paths from SQLite shadow metadata.
    pub fn recent_changes(&self, limit: usize) -> Result<Vec<(String, i64)>, CoreError> {
        let conn = self
            .sqlite
            .lock()
            .map_err(|_| CoreError::Msg("sqlite lock poisoned".into()))?;
        Ok(sqlite::recent_changes(&conn, limit)?)
    }

    /// Read UTF-8 text when `abs_path` stays inside `workspace_root` (both canonicalized).
    pub fn read_file_context(
        &self,
        workspace_root: &Path,
        abs_path: &Path,
        max_bytes: u64,
    ) -> Result<String, CoreError> {
        let ws = workspace_root.canonicalize()?;
        let target = abs_path.canonicalize().map_err(CoreError::Io)?;
        if !target.starts_with(&ws) {
            return Err(CoreError::PathForbidden(target));
        }
        let meta = std::fs::metadata(&target)?;
        let len = meta.len();
        if len > max_bytes {
            return Err(CoreError::FileTooLarge {
                path: target,
                limit: max_bytes,
            });
        }
        let bytes = std::fs::read(&target)?;
        String::from_utf8(bytes).map_err(|_| CoreError::Msg("file is not valid UTF-8".into()))
    }

    pub(crate) fn with_sqlite<R>(
        &self,
        f: impl FnOnce(&Connection) -> Result<R, rusqlite::Error>,
    ) -> Result<R, CoreError> {
        let conn = self
            .sqlite
            .lock()
            .map_err(|_| CoreError::Msg("sqlite lock poisoned".into()))?;
        Ok(f(&conn)?)
    }

    pub(crate) async fn apply_sync_task(
        &self,
        workspace_root: &Path,
        task: &crate::sync::SyncTask,
        min_file_bytes: u64,
        max_file_bytes: u64,
        workspace_ignore: &crate::pipeline::WorkspaceIgnore,
    ) -> Result<(), CoreError> {
        match task {
            crate::sync::SyncTask::SkipLocked { .. } => Ok(()),
            crate::sync::SyncTask::Purge { path_key } => {
                self.with_sqlite(|c| sqlite::delete_file_meta(c, path_key))?;
                let lance_path = lance_file_path_for_index(workspace_root, path_key)?;
                let db = self.lance.lock().await;
                lance_chunks::delete_chunks_for_file(&db, &lance_path).await?;
                Ok(())
            }
            crate::sync::SyncTask::Upsert { path_key } => {
                let rel = PathBuf::from(path_key);
                let full_path = workspace_root.join(&rel);
                let lance_path = lance_file_path_for_index(workspace_root, path_key)?;
                let meta = match std::fs::metadata(&full_path) {
                    Ok(m) => m,
                    Err(_) => {
                        self.with_sqlite(|c| sqlite::delete_file_meta(c, path_key))?;
                        let db = self.lance.lock().await;
                        lance_chunks::delete_chunks_for_file(&db, &lance_path).await?;
                        return Ok(());
                    }
                };
                let len = meta.len();
                let rel_ref = rel.as_path();
                if crate::pipeline::gates::should_skip_path_components(rel_ref) {
                    self.with_sqlite(|c| sqlite::delete_file_meta(c, path_key))?;
                    let db = self.lance.lock().await;
                    lance_chunks::delete_chunks_for_file(&db, &lance_path).await?;
                    return Ok(());
                }
                if workspace_ignore.is_ignored(rel_ref, false) {
                    self.with_sqlite(|c| sqlite::delete_file_meta(c, path_key))?;
                    let db = self.lance.lock().await;
                    lance_chunks::delete_chunks_for_file(&db, &lance_path).await?;
                    return Ok(());
                }
                if crate::pipeline::gates::should_skip_hidden_file(rel_ref) {
                    self.with_sqlite(|c| sqlite::delete_file_meta(c, path_key))?;
                    let db = self.lance.lock().await;
                    lance_chunks::delete_chunks_for_file(&db, &lance_path).await?;
                    return Ok(());
                }
                let Some(ext) = rel
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_ascii_lowercase())
                else {
                    self.with_sqlite(|c| sqlite::delete_file_meta(c, path_key))?;
                    let db = self.lance.lock().await;
                    lance_chunks::delete_chunks_for_file(&db, &lance_path).await?;
                    return Ok(());
                };
                if crate::pipeline::gates::is_blocked_executable_extension(&ext) {
                    self.with_sqlite(|c| sqlite::delete_file_meta(c, path_key))?;
                    let db = self.lance.lock().await;
                    lance_chunks::delete_chunks_for_file(&db, &lance_path).await?;
                    return Ok(());
                }
                if crate::pipeline::gates::should_skip_by_size(len, min_file_bytes, max_file_bytes)
                {
                    self.with_sqlite(|c| sqlite::delete_file_meta(c, path_key))?;
                    let db = self.lance.lock().await;
                    lance_chunks::delete_chunks_for_file(&db, &lance_path).await?;
                    return Ok(());
                }
                let Some(parser) = crate::pipeline::dispatcher::resolve_parser(&self.parsers, &ext)
                else {
                    self.with_sqlite(|c| sqlite::delete_file_meta(c, path_key))?;
                    let db = self.lance.lock().await;
                    lance_chunks::delete_chunks_for_file(&db, &lance_path).await?;
                    return Ok(());
                };
                let bytes = std::fs::read(&full_path)?;
                let content = match parser.parse_bytes(&rel, &bytes) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!(
                            path = %path_key,
                            err = %e,
                            "parser failed; clearing index rows for path"
                        );
                        self.with_sqlite(|c| sqlite::delete_file_meta(c, path_key))?;
                        let db = self.lance.lock().await;
                        lance_chunks::delete_chunks_for_file(&db, &lance_path).await?;
                        return Ok(());
                    }
                };
                let mtime_ns = file_mtime_ns(&meta);
                let hash = crate::pipeline::sha256_bytes_hex(&bytes);

                let chunk_texts = chunk_plain_text_markdown_mvp(&content);
                if chunk_texts.is_empty() {
                    let db = self.lance.lock().await;
                    lance_chunks::delete_chunks_for_file(&db, &lance_path).await?;
                } else {
                    let embedder = self.embedder_or_init()?;
                    let refs: Vec<&str> = chunk_texts.iter().map(|s| s.as_str()).collect();
                    let t0 = std::time::Instant::now();
                    let embeddings = embedder.embed_batch(&refs)?;
                    tracing::debug!(
                        path = %path_key,
                        lance_file_path = %lance_path,
                        chunk_count = chunk_texts.len(),
                        embed_ms = t0.elapsed().as_millis() as u64,
                        "embedded workspace file chunks"
                    );
                    let indexed: Vec<(u32, String)> = chunk_texts
                        .into_iter()
                        .enumerate()
                        .map(|(i, s)| (i as u32, s))
                        .collect();
                    let db = self.lance.lock().await;
                    lance_chunks::replace_file_chunks(&db, &lance_path, &indexed, &embeddings)
                        .await?;
                }

                self.with_sqlite(|c| sqlite::upsert_file_meta(c, path_key, mtime_ns, &hash))?;
                Ok(())
            }
        }
    }
}

/// Absolute path string for Lance `file_path`: canonical workspace root + shadow `path_key`.
///
/// Uses `canonicalize(workspace_root)` (not the file itself) so the same value is reproduced on
/// purge after the file is deleted.
fn lance_file_path_for_index(workspace_root: &Path, path_key: &str) -> Result<String, CoreError> {
    if path_key.is_empty() || path_key.contains("..") {
        return Err(CoreError::Msg(
            "path_key must be non-empty and must not contain '..'".into(),
        ));
    }
    let root = workspace_root.canonicalize().map_err(CoreError::Io)?;
    let abs = root.join(path_key);
    if !abs.starts_with(&root) {
        return Err(CoreError::PathForbidden(abs));
    }
    Ok(abs.to_string_lossy().into_owned())
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
