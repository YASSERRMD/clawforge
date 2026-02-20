use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
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

// Removed duplicate import
use clawforge_core::{Event, AgentSpec, Message as CoreMessage};
use clawforge_core::message::JobTrigger;
use clawforge_supervisor::Supervisor;

/// Shared application state for API handlers.
pub struct AppState {
    pub supervisor: Arc<Supervisor>,
    pub broadcast_tx: broadcast::Sender<Event>,
    pub scheduler_tx: mpsc::Sender<CoreMessage>,
    pub supervisor_tx: mpsc::Sender<CoreMessage>,
}

/// Build the Axum router with all API routes.
pub fn build_router(state: Arc<AppState>, bluebubbles_router: Option<Router>) -> Router {
    let mut app = Router::new()
        .route("/api/health", get(health))
        .route("/api/runs", get(get_runs))
        .route("/api/runs/{id}", get(get_run_details))
        .route("/api/agents", get(list_agents).post(create_agent))
        .route("/api/agents/{id}/run", get(run_agent).post(run_agent)) // Allow GET for easy testing, POST for correctness
        .route("/api/runs/{id}/cancel", get(cancel_run).post(cancel_run))
        .route("/api/runs/{id}/input", get(provide_input).post(provide_input))
        .route("/api/status", get(get_status))
        .route("/api/ws", get(ws_handler))
        .with_state(state);
        
    if let Some(bb_router) = bluebubbles_router {
        app = app.merge(bb_router);
    }
    
    app
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

/// List all registered agents.
async fn list_agents(State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    match state.supervisor.list_agents() {
        Ok(agents) => Ok(Json(json!({ "agents": agents }))),
        Err(e) => {
            tracing::error!(error = %e, "Failed to list agents");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create a new agent.
async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(mut agent): Json<AgentSpec>,
) -> Result<Json<Value>, StatusCode> {
    // Ensure ID is generated if empty (though AgentSpec::new does it, JSON might override)
    if agent.id.is_nil() {
        agent.id = uuid::Uuid::new_v4();
    }
    
    match state.supervisor.save_agent(&agent) {
        Ok(_) => Ok(Json(json!({ "status": "created", "id": agent.id }))),
        Err(e) => {
            tracing::error!(error = %e, "Failed to create agent");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Trigger a run for an agent.
async fn run_agent(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(agent_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // 1. Fetch agent spec
    let agent = state.supervisor.get_agent(&agent_id)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get agent");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // 2. Create ScheduleJob
    let run_id = uuid::Uuid::new_v4();
    let msg = CoreMessage::ScheduleJob(JobTrigger {
        run_id,
        agent_id: agent.id,
        trigger_reason: "Manually triggered via API".to_string(),
    });

    // 3. Send to scheduler
    state.scheduler_tx.send(msg).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to send job to scheduler");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({
        "status": "triggered",
        "run_id": run_id,
        "agent_id": agent_id
    })))
}

/// Cancel a specific run.
async fn cancel_run(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(run_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let msg = CoreMessage::CancelRun(run_id);
    
    state.supervisor_tx.send(msg).await.map_err(|e| {
         tracing::error!(error = %e, "Failed to send cancel request");
         StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({ "status": "cancellation_requested", "run_id": run_id })))
}

/// Provide input for a run.
async fn provide_input(
     State(state): State<Arc<AppState>>,
    axum::extract::Path(run_id): axum::extract::Path<uuid::Uuid>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let input = payload.get("input").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let msg = CoreMessage::ProvideInput { run_id, input };

    state.supervisor_tx.send(msg).await.map_err(|e| {
         tracing::error!(error = %e, "Failed to send input");
         StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(json!({ "status": "input_provided", "run_id": run_id })))
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
