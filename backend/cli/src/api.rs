use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};

use clawforge_supervisor::Supervisor;

/// Shared application state for API handlers.
pub struct AppState {
    pub supervisor: Arc<Supervisor>,
}

/// Build the Axum router with all API routes.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/runs", get(get_runs))
        .route("/api/agents", get(get_agents))
        .route("/api/status", get(get_status))
        .with_state(state)
}

/// Health check endpoint.
async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "clawforge",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Get recent runs.
async fn get_runs(State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    match state.supervisor.get_recent_runs(50) {
        Ok(runs) => Ok(Json(json!({ "runs": runs }))),
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch runs");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get registered agents (placeholder for Phase 2 agent registry).
async fn get_agents() -> Json<Value> {
    Json(json!({
        "agents": [],
        "note": "Agent registry coming in Phase 2"
    }))
}

/// Get runtime status.
async fn get_status(State(_state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "status": "running",
        "components": {
            "scheduler": "active",
            "planner": "active",
            "executor": "active",
            "supervisor": "active",
        },
        "uptime_seconds": 0,
    }))
}
