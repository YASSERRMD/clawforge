//! LLM-powered query expansion: enriches short queries before embedding.
//!
//! Short queries like "auth error" often miss relevant docs that use different
//! phrasing. This module uses the LLM to generate alternative phrasings,
//! embeds all of them, and averages the vectors for a richer query embedding.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// How many alternative phrasings to generate.
const DEFAULT_EXPANSIONS: usize = 3;

/// Input for query expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExpansionRequest {
    pub query: String,
    pub num_expansions: Option<usize>,
}

/// Output from query expansion.
#[derive(Debug, Clone)]
pub struct QueryExpansionResult {
    /// Original query.
    pub original: String,
    /// LLM-generated alternative phrasings.
    pub expansions: Vec<String>,
    /// All terms to embed (original + expansions).
    pub all_terms: Vec<String>,
}

/// Expand a query into alternative phrasings using a simple heuristic
/// (placeholder: a real implementation would call an LLM).
///
/// The heuristic approach:
/// 1. Return variations of the query (singular/plural, question form, keyword extraction).
/// 2. In production this calls the configured LLM to generate N rephrasings.
pub fn expand_query(req: &QueryExpansionRequest) -> QueryExpansionResult {
    let n = req.num_expansions.unwrap_or(DEFAULT_EXPANSIONS);
    let query = req.query.trim().to_string();
    let mut expansions = Vec::new();

    // Heuristic 1: Reframe as a question.
    if !query.ends_with('?') {
        expansions.push(format!("What is {}?", query.to_lowercase()));
    }

    // Heuristic 2: Keyword extraction (just take individual words as terms).
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.len() > 1 {
        expansions.push(words.join(" OR "));
    }

    // Heuristic 3: Append "how to" or "explain".
    if !query.starts_with("how") && !query.starts_with("explain") {
        expansions.push(format!("how to {}", query.to_lowercase()));
    }

    expansions.truncate(n);
    debug!(
        query = %query,
        expansions = ?expansions,
        "Query expanded"
    );

    let mut all_terms = vec![query.clone()];
    all_terms.extend(expansions.clone());

    QueryExpansionResult {
        original: query,
        expansions,
        all_terms,
    }
}

/// Average multiple embedding vectors into one query vector.
///
/// Used when a query has been expanded to multiple phrasings, each
/// with its own embedding. The averaged vector better captures the
/// "centroid" of the query intent.
pub fn average_embeddings(vectors: &[Vec<f32>]) -> Option<Vec<f32>> {
    if vectors.is_empty() {
        return None;
    }
    let len = vectors[0].len();
    if vectors.iter().any(|v| v.len() != len) {
        return None; // Dimension mismatch.
    }
    let n = vectors.len() as f32;
    let mut avg = vec![0.0f32; len];
    for vec in vectors {
        for (a, b) in avg.iter_mut().zip(vec.iter()) {
            *a += b;
        }
    }
    for x in &mut avg {
        *x /= n;
    }
    Some(avg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_short_query() {
        let result = expand_query(&QueryExpansionRequest {
            query: "authentication error".to_string(),
            num_expansions: Some(3),
        });
        assert_eq!(result.original, "authentication error");
        assert!(!result.expansions.is_empty());
        assert!(result.all_terms.len() >= 2);
    }

    #[test]
    fn averages_embeddings() {
        let v1 = vec![1.0f32, 0.0];
        let v2 = vec![0.0f32, 1.0];
        let avg = average_embeddings(&[v1, v2]).unwrap();
        assert!((avg[0] - 0.5).abs() < 1e-6);
        assert!((avg[1] - 0.5).abs() < 1e-6);
    }
}
