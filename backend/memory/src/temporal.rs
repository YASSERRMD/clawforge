/// Temporal decay weighting for memory search results.
///
/// Older memories receive lower scores according to an exponential
/// decay function:
///
///   score' = score × exp( −λ × age_secs )
///
/// Where λ = ln(2) / half_life_secs (exponential half-life).
use crate::types::SearchResult;

/// Apply exponential temporal decay to a set of search results.
///
/// # Arguments
/// * `results`         – scored search results (mutated in-place)
/// * `now_secs`        – current Unix timestamp in seconds
/// * `half_life_secs`  – age at which a memory's score is halved (default: 7 days = 604800s)
pub fn apply_decay(results: &mut Vec<SearchResult>, now_secs: i64, half_life_secs: f64) {
    let lambda = std::f64::consts::LN_2 / half_life_secs;
    for r in results.iter_mut() {
        let age = (now_secs - r.entry.created_at).max(0) as f64;
        let decay = (-lambda * age).exp() as f32;
        r.score *= decay;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SearchResult, VectorEntry};
    use uuid::Uuid;

    fn make_result(created_at: i64) -> SearchResult {
        SearchResult {
            entry: VectorEntry {
                id: Uuid::new_v4(),
                content: "test".into(),
                vector: vec![1.0],
                metadata: serde_json::json!({}),
                created_at,
                session_id: None,
            },
            score: 1.0,
        }
    }

    #[test]
    fn test_decay_halves_at_half_life() {
        let now = 1_000_000i64;
        let half_life = 3600.0; // 1 hour
        let mut results = vec![
            make_result(now - 3600), // exactly 1 half-life old → score should be ~0.5
        ];
        apply_decay(&mut results, now, half_life);
        let score = results[0].score;
        // Score should be approximately 0.5 (within 1% tolerance)
        assert!((score - 0.5).abs() < 0.01, "Expected ~0.5, got {}", score);
    }

    #[test]
    fn test_fresh_entry_unchanged() {
        let now = 1_000_000i64;
        let mut results = vec![make_result(now)]; // age = 0
        apply_decay(&mut results, now, 3600.0);
        assert!((results[0].score - 1.0).abs() < 0.001);
    }
}
