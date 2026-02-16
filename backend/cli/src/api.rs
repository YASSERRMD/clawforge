use std::sync::Arc;
use tokio::sync::broadcast;
use axum::{
    extract::{State, ws::{WebSocketUpgrade, WebSocket, Message}},
    http::StatusCode,
    response::{Json, IntoResponse},
    routing::get,
    Router,
};
use serde_json::{json, Value};
use tokio_stream::wrappers::BroadcastStream;
use futures::{sink::SinkExt, stream::StreamExt};

use clawforge_core::Event;
use clawforge_supervisor::Supervisor;

/// Shared application state for API handlers.
pub struct AppState {
    pub supervisor: Arc<Supervisor>,
    pub broadcast_tx: broadcast::Sender<Event>,
}

/// Build the Axum router with all API routes.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/runs", get(get_runs))
        .route("/api/runs/{id}", get(get_run_details))
        .route("/api/agents", get(get_agents))
        .route("/api/status", get(get_status))
        .route("/api/ws", get(ws_handler))
        .with_state(state)
}

/// WebSocket handler for real-time events.
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.broadcast_tx.subscribe();
    
    // Create a stream from the broadcast receiver
    let mut stream = BroadcastStream::new(rx);

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(event) => {
                if let Ok(json) = serde_json::to_string(&event) {
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }
            Err(_) => {
                // Lagged or closed
                break;
            }
        }
    }
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

/// Get details for a specific run.
async fn get_run_details(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(run_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<Value>, StatusCode> {
    match state.supervisor.get_run_summary(&run_id) {
        Ok(summary) => Ok(Json(summary)),
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch run details");
            Err(StatusCode::NOT_FOUND)
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
