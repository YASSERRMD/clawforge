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
    /// Unix timestamp (seconds) when this entry was created
    pub created_at: i64,
    /// Session or agent scope identifier (for filtering)
    pub session_id: Option<String>,
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
    /// Optional session scope filter
    pub session_id: Option<String>,
    /// Use MMR (Maximal Marginal Relevance) for diversity
    pub use_mmr: bool,
    /// MMR lambda — 1.0 = pure similarity, 0.0 = pure diversity
    pub mmr_lambda: f32,
    /// Apply temporal decay weighting
    pub use_decay: bool,
    /// Decay half-life in seconds (default 7 days)
    pub decay_half_life_secs: f64,
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self {
            vector: vec![],
            min_score: 0.0,
            limit: 10,
            session_id: None,
            use_mmr: false,
            mmr_lambda: 0.7,
            use_decay: false,
            decay_half_life_secs: 7.0 * 24.0 * 3600.0,
        }
    }
}

/// Result of a memory search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entry: VectorEntry,
    pub score: f32,
}
