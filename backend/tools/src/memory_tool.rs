//! Memory tool — search, update, and clear agent memory collections.
//!
//! Mirrors `src/agents/tools/memory-tool.ts`.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A memory search result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryHit {
    /// Content of the memory chunk.
    pub content: String,
    /// Source file path.
    pub source: Option<String>,
    /// Similarity score [0.0, 1.0].
    pub score: f32,
    /// Memory collection name.
    pub collection: String,
}

/// Input for memory-search.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySearchInput {
    /// The search query (will be embedded).
    pub query: String,
    /// Collections to search. If empty, searches all.
    #[serde(default)]
    pub collections: Vec<String>,
    /// Max results to return per collection.
    pub limit: Option<usize>,
    /// Minimum similarity threshold [0.0, 1.0].
    pub min_score: Option<f32>,
}

/// Output from memory-search.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySearchOutput {
    pub query: String,
    pub hits: Vec<MemoryHit>,
    pub total: usize,
}

/// Input for memory-add (add a fact to memory).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAddInput {
    /// Text to store in memory.
    pub content: String,
    /// Target collection (default: "default").
    pub collection: Option<String>,
    /// Optional metadata tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Output from memory-add.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAddOutput {
    pub ok: bool,
    pub chunk_id: String,
    pub collection: String,
}

/// Input for memory-delete.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDeleteInput {
    /// Chunk ID to delete (from MemoryAddOutput or MemoryHit).
    pub chunk_id: Option<String>,
    /// Delete all chunks matching a query.
    pub query: Option<String>,
    /// Collection to delete from.
    pub collection: Option<String>,
}

/// Output from memory-delete.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDeleteOutput {
    pub deleted_count: usize,
}

/// Input for memory-list-collections.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryListCollectionsInput {}

/// Output from memory-list-collections.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCollection {
    pub name: String,
    pub chunk_count: usize,
    pub size_bytes: Option<u64>,
}

/// Trait for memory backends (used by the memory tool).
#[async_trait::async_trait]
pub trait MemoryToolBackend: Send + Sync {
    async fn search(&self, input: MemorySearchInput) -> Result<MemorySearchOutput>;
    async fn add(&self, input: MemoryAddInput) -> Result<MemoryAddOutput>;
    async fn delete(&self, input: MemoryDeleteInput) -> Result<MemoryDeleteOutput>;
    async fn list_collections(&self) -> Result<Vec<MemoryCollection>>;
}
