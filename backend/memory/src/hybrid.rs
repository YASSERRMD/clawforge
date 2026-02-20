/// Hybrid search: combines dense vector similarity with sparse BM25 keyword scoring.
///
/// The combined score is:
///   hybrid_score = α × vector_score + (1 − α) × bm25_score
///
/// Where α ∈ [0, 1] (default 0.7 = mostly vector, some keyword).
use crate::types::{MemoryQuery, SearchResult, VectorEntry};

/// BM25 parameters
const K1: f32 = 1.5;
const B: f32 = 0.75;

/// Compute a simple BM25 score for a single document against a query.
fn bm25_score(query_terms: &[&str], document: &str, avg_doc_len: f32) -> f32 {
    let terms_in_doc: Vec<&str> = document.split_whitespace().collect();
    let doc_len = terms_in_doc.len() as f32;
    let mut score = 0.0f32;

    for term in query_terms {
        let term_lc = term.to_lowercase();
        let tf = terms_in_doc
            .iter()
            .filter(|t| t.to_lowercase() == term_lc)
            .count() as f32;

        if tf == 0.0 {
            continue;
        }

        // IDF is simplified here (a real BM25 needs corpus-level df).
        // We use a constant because we don't have df statistics without indexing.
        let idf = 1.0f32; // fixed for simplicity

        let numerator = tf * (K1 + 1.0);
        let denominator = tf + K1 * (1.0 - B + B * doc_len / avg_doc_len.max(1.0));
        score += idf * numerator / denominator;
    }

    score
}

/// Re-score `results` using hybrid BM25 + vector scoring.
///
/// The `results` must already have `.score` set from vector search.
pub fn hybrid_rerank(
    query_text: &str,
    results: &mut Vec<SearchResult>,
    alpha: f32,
) {
    if results.is_empty() || query_text.is_empty() {
        return;
    }

    let alpha = alpha.clamp(0.0, 1.0);
    let query_terms: Vec<&str> = query_text.split_whitespace().collect();

    let avg_doc_len = results
        .iter()
        .map(|r| r.entry.content.split_whitespace().count() as f32)
        .sum::<f32>()
        / results.len() as f32;

    // Compute raw BM25 scores
    let bm25_scores: Vec<f32> = results
        .iter()
        .map(|r| bm25_score(&query_terms, &r.entry.content, avg_doc_len))
        .collect();

    // Normalize BM25 scores to [0, 1]
    let max_bm25 = bm25_scores.iter().cloned().fold(0.0f32, f32::max);
    let bm25_scores_norm: Vec<f32> = if max_bm25 > 0.0 {
        bm25_scores.iter().map(|s| s / max_bm25).collect()
    } else {
        vec![0.0; bm25_scores.len()]
    };

    // Combine
    for (r, bm25) in results.iter_mut().zip(bm25_scores_norm.iter()) {
        let vector_score = r.score;
        r.score = alpha * vector_score + (1.0 - alpha) * bm25;
    }

    // Re-sort descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SearchResult, VectorEntry};
    use uuid::Uuid;

    fn make_result(content: &str, vec_score: f32) -> SearchResult {
        SearchResult {
            entry: VectorEntry {
                id: Uuid::new_v4(),
                content: content.to_string(),
                vector: vec![vec_score],
                metadata: serde_json::json!({}),
                created_at: 0,
                session_id: None,
            },
            score: vec_score,
        }
    }

    #[test]
    fn test_hybrid_boosts_keyword_match() {
        let mut results = vec![
            make_result("the quick brown fox", 0.9),    // high vector score
            make_result("rust programming language", 0.5), // lower vector score but keyword match
        ];

        // Query contains "rust" which matches the second result
        hybrid_rerank("rust language", &mut results, 0.5);

        // After hybrid scoring, the keyword-matching result should rank higher
        assert!(results[0].entry.content.contains("rust"),
            "Expected keyword match to rank first");
    }
}
