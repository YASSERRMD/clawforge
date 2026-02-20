/// SQLite-backed durable vector store.
///
/// Uses `rusqlite` to persist `VectorEntry` rows in a `memories` table.
/// Cosine similarity is computed in Rust application logic (not in SQLite)
/// since `sqlite-vec` is an optional native extension. This provides full
/// persistence and crash safety while keeping the dependency footprint small.
///
/// For production high-scale deployments the `mmr`, `hybrid`, and `temporal`
/// modules can be layered on top of results from this store.
use std::path::Path;

use anyhow::{Context, Result};
use async_trait::async_trait;
use rusqlite::{params, Connection};
use serde_json;
use tokio::sync::Mutex;
use tracing::{debug, info};
use uuid::Uuid;

use crate::mmr::cosine_similarity;
use crate::store::MemoryStore;
use crate::types::{MemoryQuery, SearchResult, VectorEntry};

pub struct SqliteVecStore {
    conn: Mutex<Connection>,
}

impl SqliteVecStore {
    /// Create or open a database at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path.as_ref())
            .context("Failed to open SQLite memory database")?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS memories (
                 id          TEXT PRIMARY KEY,
                 session_id  TEXT,
                 content     TEXT NOT NULL,
                 vector_json TEXT NOT NULL,
                 metadata    TEXT NOT NULL,
                 created_at  INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id);
             CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at);",
        )
        .context("Failed to initialize memories schema")?;

        info!("SqliteVecStore opened at {:?}", path.as_ref());
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open an in-memory database (for tests).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (
                 id          TEXT PRIMARY KEY,
                 session_id  TEXT,
                 content     TEXT NOT NULL,
                 vector_json TEXT NOT NULL,
                 metadata    TEXT NOT NULL,
                 created_at  INTEGER NOT NULL
             );",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }
}

#[async_trait]
impl MemoryStore for SqliteVecStore {
    async fn upsert(&self, entry: VectorEntry) -> Result<()> {
        let conn = self.conn.lock().await;
        let vector_json = serde_json::to_string(&entry.vector)?;
        let metadata_json = serde_json::to_string(&entry.metadata)?;
        conn.execute(
            "INSERT OR REPLACE INTO memories (id, session_id, content, vector_json, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.id.to_string(),
                entry.session_id,
                entry.content,
                vector_json,
                metadata_json,
                entry.created_at,
            ],
        )?;
        debug!("Upserted memory {}", entry.id);
        Ok(())
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<SearchResult>> {
        let conn = self.conn.lock().await;

        // Base query with optional session filter
        let sql = if query.session_id.is_some() {
            "SELECT id, session_id, content, vector_json, metadata, created_at
             FROM memories WHERE session_id = ?1"
        } else {
            "SELECT id, session_id, content, vector_json, metadata, created_at
             FROM memories WHERE 1=1"
        };

        let mut stmt = conn.prepare(sql)?;

        let rows: Vec<VectorEntry> = if let Some(sid) = &query.session_id {
            stmt.query_map(params![sid], |row| {
                row_to_entry(row)
            })?
            .filter_map(|r| r.ok())
            .collect()
        } else {
            stmt.query_map([], |row| row_to_entry(row))?
                .filter_map(|r| r.ok())
                .collect()
        };

        let mut results: Vec<SearchResult> = rows
            .into_iter()
            .map(|entry| {
                let score = cosine_similarity(&query.vector, &entry.vector);
                SearchResult { entry, score }
            })
            .filter(|r| r.score >= query.min_score)
            .collect();

        // Sort descending by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Temporal decay
        if query.use_decay {
            let now = chrono::Utc::now().timestamp();
            crate::temporal::apply_decay(&mut results, now, query.decay_half_life_secs);
            // Re-sort after decay
            results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        }

        // MMR re-ranking
        if query.use_mmr {
            results = crate::mmr::mmr_rerank(&query.vector, results, query.limit, query.mmr_lambda);
        } else {
            results.truncate(query.limit);
        }

        Ok(results)
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM memories WHERE id = ?1", params![id.to_string()])?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Row deserialization helper
// ---------------------------------------------------------------------------

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<VectorEntry> {
    let id_str: String = row.get(0)?;
    let session_id: Option<String> = row.get(1)?;
    let content: String = row.get(2)?;
    let vector_json: String = row.get(3)?;
    let metadata_json: String = row.get(4)?;
    let created_at: i64 = row.get(5)?;

    let id = Uuid::parse_str(&id_str)
        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
    let vector: Vec<f32> = serde_json::from_str(&vector_json)
        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
    let metadata: serde_json::Value = serde_json::from_str(&metadata_json)
        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

    Ok(VectorEntry { id, session_id, content, vector, metadata, created_at })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MemoryQuery;

    #[tokio::test]
    async fn test_sqlite_store_roundtrip() {
        let store = SqliteVecStore::in_memory().expect("in-memory db");
        let entry = VectorEntry {
            id: Uuid::new_v4(),
            content: "hello world".to_string(),
            vector: vec![1.0, 0.0, 0.0],
            metadata: serde_json::json!({}),
            created_at: 0,
            session_id: Some("sess1".to_string()),
        };
        store.upsert(entry.clone()).await.unwrap();

        let q = MemoryQuery {
            vector: vec![1.0, 0.0, 0.0],
            min_score: 0.9,
            limit: 5,
            session_id: Some("sess1".to_string()),
            ..Default::default()
        };
        let results = store.search(q).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entry.content, "hello world");
    }
}
