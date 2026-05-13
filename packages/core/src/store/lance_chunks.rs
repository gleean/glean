//! LanceDB table helpers for `document_chunks`.

use std::sync::Arc;

use arrow_array::{
    cast::AsArray, ArrayRef, FixedSizeListArray, Float32Array, RecordBatch, StringArray,
    UInt32Array,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lance_index::scalar::FullTextSearchQuery;
use lancedb::index::{Index, IndexConfig, IndexType};
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use sha2::{Digest, Sha256};

use crate::error::CoreError;
use crate::storage::StorageLayout;

/// Embedding width aligned with the internal Lance schema doc (`AllMiniLM-L6-v2`).
pub const EMBEDDING_DIM: i32 = 384;

/// Arrow schema for `document_chunks`.
pub fn chunks_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("file_path", DataType::Utf8, false),
        Field::new("chunk_index", DataType::UInt32, false),
        Field::new("text", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM,
            ),
            false,
        ),
    ]))
}

fn schema_mismatch_message() -> String {
    format!(
        "document_chunks table does not match this engine (expect {}-dim `vector`, columns id/file_path/chunk_index(UInt32)/text). \
         Stop all processes, delete `$GLEAN_STORAGE_ROOT/vectors` (or the entire storage root), then run `glean daemon` or resync to rebuild.",
        EMBEDDING_DIM
    )
}

/// Validate an existing table schema (column names and types, in order).
pub fn validate_chunks_schema(actual: &Schema) -> Result<(), CoreError> {
    let expected = chunks_schema();
    if actual.fields().len() != expected.fields().len() {
        return Err(CoreError::LanceSchemaMismatch {
            detail: schema_mismatch_message(),
        });
    }
    for i in 0..expected.fields().len() {
        let ef = expected.field(i);
        let af = actual.field(i);
        if af.name() != ef.name() || af.data_type() != ef.data_type() {
            return Err(CoreError::LanceSchemaMismatch {
                detail: schema_mismatch_message(),
            });
        }
    }
    Ok(())
}

/// Stable row id: hex-encoded `SHA256(file_path || '\\0' || BE(chunk_index))`.
pub fn chunk_row_id(file_path: &str, chunk_index: u32) -> String {
    let mut hasher = Sha256::new();
    hasher.update(file_path.as_bytes());
    hasher.update([0_u8]);
    hasher.update(chunk_index.to_be_bytes());
    format!("{:x}", hasher.finalize())
}

fn embedding_batch_from_vectors(rows: &[Vec<f32>]) -> Result<ArrayRef, CoreError> {
    let dim = EMBEDDING_DIM as usize;
    let mut flat = Vec::with_capacity(rows.len() * dim);
    for row in rows {
        if row.len() != dim {
            return Err(CoreError::Msg(format!(
                "embedding vectors must have dimension {dim}, got {}",
                row.len()
            )));
        }
        flat.extend_from_slice(row);
    }
    let values = Float32Array::from(flat);
    let field = Arc::new(Field::new("item", DataType::Float32, true));
    Ok(Arc::new(
        FixedSizeListArray::try_new(field, EMBEDDING_DIM, Arc::new(values), None)
            .map_err(|e| CoreError::Arrow(e.to_string()))?,
    ))
}

fn record_batch_for_chunks(
    schema: Arc<Schema>,
    file_path: &str,
    indexed_chunks: &[(u32, String)],
    embeddings: &[Vec<f32>],
) -> Result<RecordBatch, CoreError> {
    if indexed_chunks.len() != embeddings.len() {
        return Err(CoreError::Msg(format!(
            "chunk count {} does not match embedding count {}",
            indexed_chunks.len(),
            embeddings.len()
        )));
    }
    let n = indexed_chunks.len();
    if n == 0 {
        return Err(CoreError::Msg(
            "RecordBatch requires at least one row".into(),
        ));
    }

    let mut ids = Vec::with_capacity(n);
    let mut paths = Vec::with_capacity(n);
    let mut indices = Vec::with_capacity(n);
    let mut texts = Vec::with_capacity(n);
    for ((idx, text), _emb) in indexed_chunks.iter().zip(embeddings.iter()) {
        ids.push(chunk_row_id(file_path, *idx));
        paths.push(file_path.to_string());
        indices.push(*idx);
        texts.push(text.as_str());
    }

    let id_arr = StringArray::from(ids);
    let path_arr = StringArray::from(paths);
    let idx_arr = UInt32Array::from(indices);
    let text_arr = StringArray::from(texts);
    let vec_arr = embedding_batch_from_vectors(embeddings)?;

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id_arr),
            Arc::new(path_arr),
            Arc::new(idx_arr),
            Arc::new(text_arr),
            vec_arr,
        ],
    )
    .map_err(|e| CoreError::Arrow(e.to_string()))
}

/// Ensure `document_chunks` exists; validate schema when it already exists.
pub async fn ensure_document_chunks_table(db: &lancedb::Connection) -> Result<(), CoreError> {
    let names = db
        .table_names()
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    if names
        .iter()
        .any(|t| t == StorageLayout::DOCUMENT_CHUNKS_TABLE)
    {
        let tbl = db
            .open_table(StorageLayout::DOCUMENT_CHUNKS_TABLE)
            .execute()
            .await
            .map_err(|e| CoreError::Lance(e.to_string()))?;
        let sch = tbl
            .schema()
            .await
            .map_err(|e| CoreError::Lance(e.to_string()))?;
        validate_chunks_schema(&sch)?;
        return Ok(());
    }

    let schema = chunks_schema();
    db.create_empty_table(StorageLayout::DOCUMENT_CHUNKS_TABLE, schema)
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    Ok(())
}

/// Replace all rows for `file_path`: delete then insert (upsert semantics).
pub async fn replace_file_chunks(
    db: &lancedb::Connection,
    file_path: &str,
    chunks: &[(u32, String)],
    embeddings: &[Vec<f32>],
) -> Result<(), CoreError> {
    ensure_document_chunks_table(db).await?;

    let tbl = db
        .open_table(StorageLayout::DOCUMENT_CHUNKS_TABLE)
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    let escaped = file_path.replace('\'', "''");
    tbl.delete(&format!("file_path = '{escaped}'"))
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    if chunks.is_empty() {
        return Ok(());
    }

    let schema = chunks_schema();
    let batch = record_batch_for_chunks(schema, file_path, chunks, embeddings)?;

    tbl.add(vec![batch])
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    ensure_text_fts_index(db).await?;

    Ok(())
}

fn fts_index_covers_text(indices: &[IndexConfig]) -> bool {
    indices.iter().any(|cfg| {
        cfg.index_type == IndexType::FTS && cfg.columns.iter().any(|c| c == "text")
    })
}

/// Ensure a BM25-capable FTS index exists on `text` when the table has rows.
pub async fn ensure_text_fts_index(db: &lancedb::Connection) -> Result<(), CoreError> {
    ensure_document_chunks_table(db).await?;
    let tbl = db
        .open_table(StorageLayout::DOCUMENT_CHUNKS_TABLE)
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    let row_count = tbl
        .count_rows(None)
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;
    if row_count == 0 {
        return Ok(());
    }

    let indices = tbl
        .list_indices()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;
    if fts_index_covers_text(&indices) {
        return Ok(());
    }

    tbl.create_index(&["text"], Index::FTS(Default::default()))
        .replace(true)
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    Ok(())
}

/// Delete every chunk row for `file_path`.
pub async fn delete_chunks_for_file(
    db: &lancedb::Connection,
    file_path: &str,
) -> Result<(), CoreError> {
    ensure_document_chunks_table(db).await?;
    let tbl = db
        .open_table(StorageLayout::DOCUMENT_CHUNKS_TABLE)
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    let escaped = file_path.replace('\'', "''");
    tbl.delete(&format!("file_path = '{escaped}'"))
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    Ok(())
}

/// Vector kNN (flat scan when no vector index); returns `(file_path, text)`.
pub async fn vector_search_chunks(
    db: &lancedb::Connection,
    query_embedding: &[f32],
    limit: usize,
) -> Result<Vec<(String, String)>, CoreError> {
    ensure_document_chunks_table(db).await?;
    if query_embedding.len() != EMBEDDING_DIM as usize {
        return Err(CoreError::Msg(format!(
            "query embedding length must be {}, got {}",
            EMBEDDING_DIM,
            query_embedding.len()
        )));
    }
    if limit == 0 {
        return Ok(Vec::new());
    }

    let tbl = db
        .open_table(StorageLayout::DOCUMENT_CHUNKS_TABLE)
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    let stream = tbl
        .query()
        .nearest_to(query_embedding)
        .map_err(|e| CoreError::Lance(e.to_string()))?
        .column("vector")
        .limit(limit)
        .select(Select::Columns(vec!["file_path".into(), "text".into()]))
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    let batches: Vec<RecordBatch> = stream
        .try_collect()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    let mut out = Vec::new();
    for batch in batches {
        let fp = batch
            .column_by_name("file_path")
            .and_then(|c| c.as_string_opt::<i32>());
        let tx = batch
            .column_by_name("text")
            .and_then(|c| c.as_string_opt::<i32>());
        let (Some(fp), Some(tx)) = (fp, tx) else {
            continue;
        };
        for i in 0..batch.num_rows() {
            out.push((fp.value(i).to_string(), tx.value(i).to_string()));
            if out.len() >= limit {
                return Ok(out);
            }
        }
    }

    Ok(out)
}

/// Hybrid vector kNN + BM25 with default RRF reranking (requires `text` FTS index).
pub async fn hybrid_search_chunks(
    db: &lancedb::Connection,
    query_embedding: &[f32],
    fts_query: &str,
    limit: usize,
) -> Result<Vec<(String, String)>, CoreError> {
    ensure_document_chunks_table(db).await?;
    if query_embedding.len() != EMBEDDING_DIM as usize {
        return Err(CoreError::Msg(format!(
            "query embedding length must be {}, got {}",
            EMBEDDING_DIM,
            query_embedding.len()
        )));
    }
    if limit == 0 {
        return Ok(Vec::new());
    }

    let tbl = db
        .open_table(StorageLayout::DOCUMENT_CHUNKS_TABLE)
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    let fts_q = FullTextSearchQuery::new(fts_query.to_string());

    let stream = tbl
        .query()
        .full_text_search(fts_q)
        .limit(limit)
        .nearest_to(query_embedding)
        .map_err(|e| CoreError::Lance(e.to_string()))?
        .column("vector")
        .select(Select::Columns(vec!["file_path".into(), "text".into()]))
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    let batches: Vec<RecordBatch> = stream
        .try_collect()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    let mut out = Vec::new();
    for batch in batches {
        let fp = batch
            .column_by_name("file_path")
            .and_then(|c| c.as_string_opt::<i32>());
        let tx = batch
            .column_by_name("text")
            .and_then(|c| c.as_string_opt::<i32>());
        let (Some(fp), Some(tx)) = (fp, tx) else {
            continue;
        };
        for i in 0..batch.num_rows() {
            out.push((fp.value(i).to_string(), tx.value(i).to_string()));
            if out.len() >= limit {
                return Ok(out);
            }
        }
    }

    Ok(out)
}

/// Prefer hybrid BM25 + vector (RRF) when FTS is available; fall back to vector-only on failure.
pub async fn semantic_search_chunks(
    db: &lancedb::Connection,
    query_embedding: &[f32],
    query_text: &str,
    limit: usize,
) -> Result<Vec<(String, String)>, CoreError> {
    ensure_document_chunks_table(db).await?;
    if query_embedding.len() != EMBEDDING_DIM as usize {
        return Err(CoreError::Msg(format!(
            "query embedding length must be {}, got {}",
            EMBEDDING_DIM,
            query_embedding.len()
        )));
    }
    if limit == 0 {
        return Ok(Vec::new());
    }

    let tbl = db
        .open_table(StorageLayout::DOCUMENT_CHUNKS_TABLE)
        .execute()
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;

    let rows = tbl
        .count_rows(None)
        .await
        .map_err(|e| CoreError::Lance(e.to_string()))?;
    if rows == 0 {
        return Ok(Vec::new());
    }

    let trimmed = query_text.trim();
    if trimmed.is_empty() {
        return vector_search_chunks(db, query_embedding, limit).await;
    }

    ensure_text_fts_index(db).await?;

    match hybrid_search_chunks(db, query_embedding, trimmed, limit).await {
        Ok(v) => Ok(v),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "hybrid_search_chunks failed; falling back to vector-only search"
            );
            vector_search_chunks(db, query_embedding, limit).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn vector_search_returns_nearest_neighbor_first() {
        let dir = tempdir().unwrap();
        let uri = dir.path().to_string_lossy().to_string();
        let db = lancedb::connect(&uri).execute().await.unwrap();

        ensure_document_chunks_table(&db).await.unwrap();

        let mut z = vec![0.0_f32; EMBEDDING_DIM as usize];
        z[0] = 1.0;
        let mut far = vec![0.0_f32; EMBEDDING_DIM as usize];
        far[1] = 1.0;

        replace_file_chunks(
            &db,
            "a.txt",
            &[(0, "near".into()), (1, "far".into())],
            &[z.clone(), far],
        )
        .await
        .unwrap();

        let hits = vector_search_chunks(&db, &z, 5).await.unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].1, "near");
    }

    #[tokio::test]
    async fn hybrid_search_prioritizes_chunk_matching_fts_when_vector_prefers_other() {
        let dir = tempdir().unwrap();
        let uri = dir.path().to_string_lossy().to_string();
        let db = lancedb::connect(&uri).execute().await.unwrap();

        ensure_document_chunks_table(&db).await.unwrap();

        let mut near = vec![0.0_f32; EMBEDDING_DIM as usize];
        near[0] = 1.0;
        let mut far = vec![0.0_f32; EMBEDDING_DIM as usize];
        far[1] = 1.0;

        let filler = "lorem ipsum boring filler text ".repeat(24);
        let needle = "XYZZYHYBRIDMARKER99";
        let chunk_far_text = format!("{filler} token_value_{needle} end");
        let chunk_near_text = filler.clone();

        replace_file_chunks(
            &db,
            "doc.txt",
            &[(0, chunk_near_text), (1, chunk_far_text)],
            &[near.clone(), far],
        )
        .await
        .unwrap();

        let hits = semantic_search_chunks(&db, &near, needle, 5)
            .await
            .unwrap();
        assert!(
            hits.first().map(|(_, t)| t.contains(needle)).unwrap_or(false),
            "expected hybrid RRF to rank lexical match first, got {:?}",
            hits
        );
    }
}
