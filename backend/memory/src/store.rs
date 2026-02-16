use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::types::{MemoryQuery, SearchResult, VectorEntry};

/// Abstract interface for vector storage.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Insert or update an entry.
    async fn upsert(&self, entry: VectorEntry) -> Result<()>;

    /// Search for similar entries.
    async fn search(&self, query: MemoryQuery) -> Result<Vec<SearchResult>>;

    /// Delete an entry by ID.
    async fn delete(&self, id: Uuid) -> Result<()>;
}

/// Simple in-memory vector store for MVP/testing.
/// Uses brute-force cosine similarity.
pub struct InMemoryVectorStore {
    entries: Arc<RwLock<HashMap<Uuid, VectorEntry>>>,
}

impl InMemoryVectorStore {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Calculate cosine similarity between two vectors.
    fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
        if v1.len() != v2.len() {
            return 0.0;
        }

        let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
        let norm_a: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
        let norm_b: f32 = v2.iter().map(|b| b * b).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }
}

#[async_trait]
impl MemoryStore for InMemoryVectorStore {
    async fn upsert(&self, entry: VectorEntry) -> Result<()> {
        let mut entries = self.entries.write().unwrap();
        entries.insert(entry.id, entry);
        Ok(())
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<SearchResult>> {
        let entries = self.entries.read().unwrap();
        let mut results: Vec<SearchResult> = entries
            .values()
            .map(|entry| {
                let score = Self::cosine_similarity(&query.vector, &entry.vector);
                SearchResult {
                    entry: entry.clone(),
                    score,
                }
            })
            .filter(|r| r.score >= query.min_score)
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        
        // Take top N
        results.truncate(query.limit);

        Ok(results)
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let mut entries = self.entries.write().unwrap();
        entries.remove(&id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_and_search() {
        let store = InMemoryVectorStore::new();

        let entry1 = VectorEntry {
            id: Uuid::new_v4(),
            content: "The cat sits on the mat".to_string(),
            vector: vec![1.0, 0.0, 0.0],
            metadata: serde_json::json!({}),
        };

        let entry2 = VectorEntry {
            id: Uuid::new_v4(),
            content: "The dog barks at the mailman".to_string(),
            vector: vec![0.0, 1.0, 0.0],
            metadata: serde_json::json!({}),
        };

        store.upsert(entry1.clone()).await.unwrap();
        store.upsert(entry2.clone()).await.unwrap();

        // Query close to entry1
        let query = MemoryQuery {
            vector: vec![0.9, 0.1, 0.0],
            min_score: 0.5,
            limit: 1,
        };

        let results = store.search(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entry.content, entry1.content);
        assert!(results[0].score > 0.8);
    }
}
