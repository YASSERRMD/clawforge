use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A stored memory entry with embedding vector and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: Uuid,
    /// The text content of the memory
    pub content: String,
    /// The embedding vector (e.g., 1536 dim for OpenAI ada-002)
    pub vector: Vec<f32>,
    /// Metadata key-value pairs
    pub metadata: serde_json::Value,
}

/// A query for retrieving relevant memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// The embedding vector of the query
    pub vector: Vec<f32>,
    /// Minimum similarity score (0.0 to 1.0)
    pub min_score: f32,
    /// Max number of results to return
    pub limit: usize,
}

/// Result of a memory search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entry: VectorEntry,
    pub score: f32,
}
