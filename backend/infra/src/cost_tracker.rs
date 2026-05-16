//! Cost Tracker Module
//!
//! Accumulates per-session LLM token cost in memory with a capped ring buffer.
//! Records are kept up to MAX_RECORDS; oldest entries are dropped when full.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_RECORDS: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    pub session_id: String,
    pub agent_id: String,
    pub model_name: String,
    pub usage: TokenUsage,
    pub cost_usd: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone)]
pub struct CostTracker {
    records: Arc<RwLock<VecDeque<CostRecord>>>,
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_RECORDS))),
        }
    }

    /// Calculate the cost for a given usage and model.
    pub fn calculate_cost(model_name: &str, usage: &TokenUsage) -> f64 {
        let (in_price_per_1k, out_price_per_1k) = match model_name {
            "gpt-4"           => (0.03,   0.06),
            "gpt-3.5-turbo"   => (0.0015, 0.002),
            "claude-3-opus"   => (0.015,  0.075),
            _                 => (0.001,  0.001),
        };
        let in_cost  = (usage.prompt_tokens as f64     / 1000.0) * in_price_per_1k;
        let out_cost = (usage.completion_tokens as f64 / 1000.0) * out_price_per_1k;
        in_cost + out_cost
    }

    /// Record a token usage event; oldest entry is evicted when the buffer is full.
    pub async fn record_usage(
        &self,
        session_id: &str,
        agent_id: &str,
        model_name: &str,
        usage: TokenUsage,
    ) -> anyhow::Result<CostRecord> {
        let cost_usd = Self::calculate_cost(model_name, &usage);
        let record = CostRecord {
            session_id: session_id.into(),
            agent_id: agent_id.into(),
            model_name: model_name.into(),
            usage,
            cost_usd,
            timestamp: Utc::now(),
        };

        let mut records = self.records.write().await;
        if records.len() >= MAX_RECORDS {
            records.pop_front();
        }
        records.push_back(record.clone());
        tracing::debug!(cost_usd = record.cost_usd, session_id = %record.session_id, "Usage recorded");

        Ok(record)
    }

    /// Return a snapshot of all stored records (most recent last).
    pub async fn get_records(&self) -> Vec<CostRecord> {
        self.records.read().await.iter().cloned().collect()
    }

    /// Return the sum of all recorded costs in USD.
    pub async fn total_cost_usd(&self) -> f64 {
        self.records.read().await.iter().map(|r| r.cost_usd).sum()
    }
}

impl Default for CostTracker {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_and_total() {
        let tracker = CostTracker::new();
        let usage = TokenUsage { prompt_tokens: 1000, completion_tokens: 500, total_tokens: 1500 };
        tracker.record_usage("s1", "a1", "gpt-4", usage).await.unwrap();
        assert!(tracker.total_cost_usd().await > 0.0);
        assert_eq!(tracker.get_records().await.len(), 1);
    }

    #[tokio::test]
    async fn test_ring_buffer_cap() {
        let tracker = CostTracker::new();
        for i in 0..MAX_RECORDS + 5 {
            let u = TokenUsage { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 };
            tracker.record_usage(&i.to_string(), "a", "gpt-4", u).await.unwrap();
        }
        assert_eq!(tracker.get_records().await.len(), MAX_RECORDS);
    }
}
