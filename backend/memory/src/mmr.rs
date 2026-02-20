/// Maximal Marginal Relevance (MMR) re-ranking.
///
/// Selects the next result that maximises relevance to the *query* and
/// minimises redundancy with *already-selected* results.
///
/// score(d) = λ · sim(d, q) − (1 – λ) · max_{d' ∈ S} sim(d, d')
///
/// Reference: Carbonell & Goldstein, 1998.
use crate::types::SearchResult;

/// Calculate cosine similarity between two equal-length slices.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}

/// Rerank `candidates` using MMR.
///
/// # Arguments
/// * `query_vector` – the query embedding
/// * `candidates`   – pre-filtered and scored search results (by cosine sim)
/// * `k`            – how many to return
/// * `lambda`       – 0.0 = pure diversity, 1.0 = pure relevance (default 0.7)
pub fn mmr_rerank(
    query_vector: &[f32],
    candidates: Vec<SearchResult>,
    k: usize,
    lambda: f32,
) -> Vec<SearchResult> {
    if candidates.is_empty() || k == 0 {
        return vec![];
    }

    let lambda = lambda.clamp(0.0, 1.0);
    let mut remaining: Vec<SearchResult> = candidates;
    let mut selected: Vec<SearchResult> = Vec::with_capacity(k);

    while selected.len() < k && !remaining.is_empty() {
        let best_idx = remaining
            .iter()
            .enumerate()
            .map(|(i, cand)| {
                let relevance = cosine_similarity(query_vector, &cand.entry.vector);

                let redundancy = selected
                    .iter()
                    .map(|s| cosine_similarity(&cand.entry.vector, &s.entry.vector))
                    .fold(f32::NEG_INFINITY, f32::max);

                let redundancy = if redundancy == f32::NEG_INFINITY { 0.0 } else { redundancy };
                let mmr_score = lambda * relevance - (1.0 - lambda) * redundancy;
                (i, mmr_score)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i);

        if let Some(idx) = best_idx {
            selected.push(remaining.remove(idx));
        } else {
            break;
        }
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SearchResult, VectorEntry};
    use uuid::Uuid;

    fn make_result(vec: Vec<f32>, score: f32) -> SearchResult {
        SearchResult {
            entry: VectorEntry {
                id: Uuid::new_v4(),
                content: "test".into(),
                vector: vec,
                metadata: serde_json::json!({}),
                created_at: 0,
                session_id: None,
            },
            score,
        }
    }

    #[test]
    fn test_mmr_selects_diverse_results() {
        let query = vec![1.0f32, 0.0, 0.0];

        // r1 and r2 are near-identical; r3 is diverse
        let r1 = make_result(vec![1.0, 0.0, 0.0], 1.0);
        let r2 = make_result(vec![0.99, 0.01, 0.0], 0.99);
        let r3 = make_result(vec![0.0, 1.0, 0.0], 0.5);

        let results = mmr_rerank(&query, vec![r1, r2, r3], 2, 0.5);
        assert_eq!(results.len(), 2);
        // The best candidate (r1) is always first
        // The second should be r3 (diverse), not r2 (near-duplicate of r1)
        let second_content_vec = &results[1].entry.vector;
        assert!(second_content_vec[1] > 0.5); // r3's y component
    }
}
