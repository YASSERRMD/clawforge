//! ACP (Agent Communication Protocol) telemetry: tracks request/response metrics.
//!
//! Mirrors `src/acp/telemetry.ts`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

/// A single ACP request telemetry record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpRequestRecord {
    pub request_id: String,
    pub method: String,
    pub target_agent: String,
    pub duration_ms: u64,
    pub success: bool,
    pub error_code: Option<String>,
    pub timestamp_secs: u64,
}

/// Aggregated telemetry for a method.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodStats {
    pub total_calls: u64,
    pub success_calls: u64,
    pub failure_calls: u64,
    pub total_duration_ms: u64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
}

impl MethodStats {
    pub fn avg_duration_ms(&self) -> f64 {
        if self.total_calls == 0 { 0.0 } else { self.total_duration_ms as f64 / self.total_calls as f64 }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_calls == 0 { 1.0 } else { self.success_calls as f64 / self.total_calls as f64 }
    }
}

/// ACP telemetry store.
pub struct AcpTelemetry {
    /// Recent records (ring-buffer style, capped at max_records).
    records: Arc<RwLock<Vec<AcpRequestRecord>>>,
    max_records: usize,
    stats: Arc<RwLock<HashMap<String, MethodStats>>>,
}

impl AcpTelemetry {
    pub fn new(max_records: usize) -> Self {
        Self {
            records: Arc::new(RwLock::new(Vec::new())),
            max_records,
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a completed ACP request.
    pub async fn record(&self, rec: AcpRequestRecord) {
        debug!(method = %rec.method, duration_ms = rec.duration_ms, success = rec.success, "ACP telemetry");

        // Update method stats.
        {
            let mut stats = self.stats.write().await;
            let entry = stats.entry(rec.method.clone()).or_default();
            entry.total_calls += 1;
            entry.total_duration_ms += rec.duration_ms;
            if rec.success {
                entry.success_calls += 1;
            } else {
                entry.failure_calls += 1;
            }
            if entry.total_calls == 1 {
                entry.min_duration_ms = rec.duration_ms;
                entry.max_duration_ms = rec.duration_ms;
            } else {
                entry.min_duration_ms = entry.min_duration_ms.min(rec.duration_ms);
                entry.max_duration_ms = entry.max_duration_ms.max(rec.duration_ms);
            }
        }

        // Append to ring buffer.
        let mut records = self.records.write().await;
        records.push(rec);
        if records.len() > self.max_records {
            let excess = records.len() - self.max_records;
            records.drain(..excess);
        }
    }

    /// Get recent records.
    pub async fn recent(&self, limit: usize) -> Vec<AcpRequestRecord> {
        let records = self.records.read().await;
        records.iter().rev().take(limit).cloned().collect()
    }

    /// Get stats per method.
    pub async fn stats(&self) -> HashMap<String, MethodStats> {
        self.stats.read().await.clone()
    }

    /// Clear all telemetry.
    pub async fn clear(&self) {
        self.records.write().await.clear();
        self.stats.write().await.clear();
    }
}

/// Timer helper for measuring ACP call durations.
pub struct AcpTimer {
    pub start: Instant,
}

impl AcpTimer {
    pub fn start() -> Self {
        Self { start: Instant::now() }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}
