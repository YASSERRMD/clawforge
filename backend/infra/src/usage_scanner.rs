//! Usage Scanner Module
//!
//! Scans session history for cost/token usage: aggregates by agent, by channel, by date range.

use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::info;

#[derive(Debug, Serialize)]
pub struct UsageReport {
    pub total_cost_usd: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

pub struct UsageScanner {
    // Db connection pool
}

impl UsageScanner {
    pub fn new() -> Self {
        Self {}
    }

    /// Scan usage within a specific date range representing global or specific channel cost.
    pub async fn scan_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        _agent_id: Option<&str>,
        _channel_id: Option<&str>,
    ) -> anyhow::Result<UsageReport> {
        info!("Scanning usage between {} and {}", start, end);
        
        // MOCK: In reality, run a SQL sum group by date
        Ok(UsageReport {
            total_cost_usd: 12.45,
            total_input_tokens: 45000,
            total_output_tokens: 12000,
            start_date: start,
            end_date: end,
        })
    }
}
