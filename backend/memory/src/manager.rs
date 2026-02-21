//! Top-level Memory Manager: orchestrates search, sync, and collection management.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    embeddings::EmbeddingProvider,
    query_expansion::{average_embeddings, expand_query, QueryExpansionRequest},
    sqlite_store::SqliteVecStore,
    store::MemoryStore,
    types::{MemoryQuery, SearchResult, VectorEntry},
};
use uuid::Uuid;

/// A named memory collection backed by a SQLite-vec store.
pub struct MemoryCollection {
    pub name: String,
    pub store: SqliteVecStore,
}

/// Options for a memory search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchOptions {
    pub query: String,
    /// Collections to search. If empty, searches all.
    #[serde(default)]
    pub collections: Vec<String>,
    /// Max results per collection.
    pub limit: Option<usize>,
    /// Minimum score threshold.
    pub min_score: Option<f32>,
    /// Apply MMR reranking for diversity.
    #[serde(default)]
    pub use_mmr: bool,
    /// Apply time-decay scoring.
    #[serde(default)]
    pub use_decay: bool,
    /// Expand query before embedding.
    #[serde(default)]
    pub use_expansion: bool,
    /// MMR lambda (0=diversity, 1=relevance). Default 0.7.
    pub mmr_lambda: Option<f32>,
    /// Filter by session ID.
    pub session_id: Option<String>,
}

/// Search result with collection metadata.
#[derive(Debug, Clone, Serialize)]
pub struct ManagedSearchResult {
    #[serde(flatten)]
    pub result: SearchResult,
    pub collection: String,
}

/// The central memory manager.
pub struct MemoryManager {
    collections: Arc<RwLock<HashMap<String, MemoryCollection>>>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
}

impl MemoryManager {
    pub fn new(embedding_provider: Arc<dyn EmbeddingProvider>) -> Self {
        Self {
            collections: Arc::new(RwLock::new(HashMap::new())),
            embedding_provider,
        }
    }

    /// Open or create a collection with the given name.
    pub fn open_collection(&self, name: &str, db_path: &Path) -> Result<()> {
        let store = SqliteVecStore::open(db_path)?;
        let name = name.to_string();
        let collections = Arc::clone(&self.collections);
        // Block-in-place registration (store open is sync).
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                collections.write().await.insert(
                    name.clone(),
                    MemoryCollection { name: name.clone(), store },
                );
            });
        });
        info!(collection = %name, "Memory collection opened");
        Ok(())
    }

    /// Register an already-opened collection.
    pub async fn register_collection(&self, name: &str, store: SqliteVecStore) {
        self.collections.write().await.insert(
            name.to_string(),
            MemoryCollection { name: name.to_string(), store },
        );
        info!(collection = %name, "Memory collection registered");
    }

    /// Search across one or more collections.
    pub async fn search(&self, opts: MemorySearchOptions) -> Result<Vec<ManagedSearchResult>> {
        let limit = opts.limit.unwrap_or(10);
        let min_score = opts.min_score.unwrap_or(0.0);
        let lambda = opts.mmr_lambda.unwrap_or(0.7);

        // Build query embedding (with optional expansion).
        let query_vec = if opts.use_expansion {
            let expansion = expand_query(&QueryExpansionRequest {
                query: opts.query.clone(),
                num_expansions: Some(3),
            });
            let mut vecs = Vec::new();
            for term in &expansion.all_terms {
                vecs.push(self.embedding_provider.embed(term).await?);
            }
            average_embeddings(&vecs).ok_or_else(|| anyhow::anyhow!("Embedding failed"))?
        } else {
            self.embedding_provider.embed(&opts.query).await?
        };

        let collections = self.collections.read().await;
        let search_names: Vec<String> = if opts.collections.is_empty() {
            collections.keys().cloned().collect()
        } else {
            opts.collections.clone()
        };

        let mut all_results: Vec<ManagedSearchResult> = Vec::new();

        for name in &search_names {
            let Some(coll) = collections.get(name) else {
                continue;
            };

            let query = MemoryQuery {
                vector: query_vec.clone(),
                min_score,
                limit: limit * 2,
                session_id: opts.session_id.clone(),
                use_mmr: opts.use_mmr,
                mmr_lambda: lambda,
                use_decay: opts.use_decay,
                ..Default::default()
            };

            let results = coll.store.search(query).await?;

            for r in results.into_iter().take(limit) {
                all_results.push(ManagedSearchResult {
                    collection: name.clone(),
                    result: r,
                });
            }
        }

        // Sort by score across all collections.
        all_results.sort_by(|a, b| {
            b.result.score.partial_cmp(&a.result.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.truncate(limit);

        Ok(all_results)
    }

    /// Insert a text entry into a collection.
    pub async fn insert(
        &self,
        collection: &str,
        content: &str,
        metadata: serde_json::Value,
        session_id: Option<String>,
    ) -> Result<VectorEntry> {
        let vector = self.embedding_provider.embed(content).await?;
        let now = chrono::Utc::now().timestamp();
        let entry = VectorEntry {
            id: Uuid::new_v4(),
            content: content.to_string(),
            vector,
            metadata,
            created_at: now,
            session_id,
        };

        let collections = self.collections.read().await;
        let coll = collections
            .get(collection)
            .ok_or_else(|| anyhow::anyhow!("Collection '{collection}' not found"))?;
        coll.store.upsert(entry.clone()).await?;
        info!(collection = %collection, id = %entry.id, "Memory entry inserted");
        Ok(entry)
    }

    /// Delete an entry by ID from a collection.
    pub async fn delete(&self, collection: &str, id: Uuid) -> Result<()> {
        let collections = self.collections.read().await;
        let coll = collections
            .get(collection)
            .ok_or_else(|| anyhow::anyhow!("Collection '{collection}' not found"))?;
        coll.store.delete(id).await
    }

    /// List all open collection names.
    pub async fn list_collections(&self) -> Vec<String> {
        self.collections.read().await.keys().cloned().collect()
    }
}
