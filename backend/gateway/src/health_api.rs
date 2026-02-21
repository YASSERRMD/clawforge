//! Gateway Health API
//!
//! Exposes a public endpoint for global gateway health and specific channel statuses.

use axum::{extract::State, Json};
use serde::Serialize;
use chrono::{DateTime, Utc};

use crate::server::GatewayState;
use crate::health_monitor::ChannelHealth;

#[derive(Serialize)]
pub struct GlobalHealthReport {
    pub status: String,
    pub uptime_seconds: u64,
    pub channels: Vec<ChannelHealth>,
    pub timestamp: DateTime<Utc>,
}

/// Handler for `GET /api/health`
pub async fn get_health(State(state): State<GatewayState>) -> Json<GlobalHealthReport> {
    
    // Combine gateway status with channel status
    let channels = state.health_monitor.get_report().await;
    
    // If any channel is offline, we might report global as degraded, but let's just return OK for the gateway process itself.
    Json(GlobalHealthReport {
        status: "ok".into(),
        uptime_seconds: 1337, // MOCK: implement real uptime tracker
        channels,
        timestamp: Utc::now(),
    })
}
