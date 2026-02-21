//! Cost Tracker Module
//!
//! Mirrors `src/infra/session-cost-usage.ts` in purpose.
//! Accumulates per-session LLM token cost: input/output tokens, calculating cost, etc.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

pub struct CostTracker {
    // In a real implementation, this would hold a db connection pool (sqlx::PgPool or SqlitePool)
}

impl CostTracker {
    pub fn new() -> Self {
        Self {}
    }

    /// Calculate the cost for a given usage and model.
    /// This is simplified; ideally this maps against a database of model prices.
    pub fn calculate_cost(model_name: &str, usage: &TokenUsage) -> f64 {
        // Mock prices
        let (in_price_per_1k, out_price_per_1k) = match model_name {
            "gpt-4" => (0.03, 0.06),
            "gpt-3.5-turbo" => (0.0015, 0.002),
            "claude-3-opus" => (0.015, 0.075),
            _ => (0.001, 0.001), // default fallback
        };

        let in_cost = (usage.prompt_tokens as f64 / 1000.0) * in_price_per_1k;
        let out_cost = (usage.completion_tokens as f64 / 1000.0) * out_price_per_1k;
        
        in_cost + out_cost
    }

    /// Record a token usage event.
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

        // TODO: Save to DB
        tracing::debug!("Recorded usage: {:?}", record);

        Ok(record)
    }
}
