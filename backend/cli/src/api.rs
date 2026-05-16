use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use axum::{
    extract::{State, Query, ws::{WebSocketUpgrade, WebSocket, Message}},
    http::StatusCode,
    response::{Json, IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct PaginationParams {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize { 20 }
use serde_json::{json, Value};
use tokio_stream::wrappers::BroadcastStream;
use futures::{sink::SinkExt, stream::StreamExt};

/// Standardized JSON error response returned by all API handlers.
fn api_error(status: StatusCode, code: &str, message: &str) -> Response {
    let body = Json(json!({ "error": code, "message": message }));
    (status, body).into_response()
}

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

/// Get recent runs with optional pagination (?limit=20&offset=0).
async fn get_runs(
    State(state): State<Arc<AppState>>,
    Query(page): Query<PaginationParams>,
) -> Response {
    let limit = page.limit.min(200);
    match state.supervisor.get_recent_runs(limit, page.offset) {
        Ok(runs) => {
            Json(json!({ "runs": runs, "limit": limit, "offset": page.offset })).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch runs");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "fetch_runs_failed", "Could not retrieve runs")
        }
    }
}

/// Get details for a specific run.
async fn get_run_details(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(run_id): axum::extract::Path<uuid::Uuid>,
) -> Response {
    match state.supervisor.get_run_summary(&run_id) {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch run details");
            api_error(StatusCode::NOT_FOUND, "run_not_found", &format!("Run {} not found", run_id))
        }
    }
}

/// List registered agents with optional pagination (?limit=20&offset=0).
async fn list_agents(
    State(state): State<Arc<AppState>>,
    Query(page): Query<PaginationParams>,
) -> Response {
    let limit = page.limit.min(200);
    match state.supervisor.list_agents_page(limit, page.offset) {
        Ok(agents) => {
            Json(json!({ "agents": agents, "limit": limit, "offset": page.offset })).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to list agents");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "list_agents_failed", "Could not retrieve agents")
        }
    }
}

/// Create a new agent.
async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(mut agent): Json<AgentSpec>,
) -> Response {
    let name = agent.name.trim().to_string();
    if name.is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "invalid_name", "Agent name must not be empty");
    }
    if name.len() > 256 {
        return api_error(StatusCode::BAD_REQUEST, "invalid_name", "Agent name must not exceed 256 characters");
    }
    agent.name = name;

    let max_tokens = agent.llm_policy.max_tokens;
    if max_tokens == 0 || max_tokens > 32_000 {
        return api_error(StatusCode::BAD_REQUEST, "invalid_max_tokens", "max_tokens must be between 1 and 32000");
    }

    if agent.id.is_nil() {
        agent.id = uuid::Uuid::new_v4();
    }
    match state.supervisor.save_agent(&agent) {
        Ok(_) => Json(json!({ "status": "created", "id": agent.id })).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to create agent");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "create_agent_failed", "Could not save agent")
        }
    }
}

/// Trigger a run for an agent.
async fn run_agent(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(agent_id): axum::extract::Path<uuid::Uuid>,
) -> Response {
    let agent = match state.supervisor.get_agent(&agent_id) {
        Ok(Some(a)) => a,
        Ok(None) => return api_error(StatusCode::NOT_FOUND, "agent_not_found", &format!("Agent {} not found", agent_id)),
        Err(e) => {
            tracing::error!(error = %e, "Failed to get agent");
            return api_error(StatusCode::INTERNAL_SERVER_ERROR, "get_agent_failed", "Could not retrieve agent");
        }
    };

    let run_id = uuid::Uuid::new_v4();
    let msg = CoreMessage::ScheduleJob(JobTrigger {
        run_id,
        agent_id: agent.id,
        trigger_reason: "Manually triggered via API".to_string(),
    });

    match state.scheduler_tx.send(msg).await {
        Ok(_) => Json(json!({ "status": "triggered", "run_id": run_id, "agent_id": agent_id })).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to send job to scheduler");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "schedule_failed", "Could not schedule agent run")
        }
    }
}

/// Cancel a specific run.
async fn cancel_run(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(run_id): axum::extract::Path<uuid::Uuid>,
) -> Response {
    match state.supervisor_tx.send(CoreMessage::CancelRun(run_id)).await {
        Ok(_) => Json(json!({ "status": "cancellation_requested", "run_id": run_id })).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to send cancel request");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "cancel_failed", "Could not cancel run")
        }
    }
}

/// Provide input for a run.
async fn provide_input(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(run_id): axum::extract::Path<uuid::Uuid>,
    Json(payload): Json<Value>,
) -> Response {
    let input = match payload.get("input").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
        Some(s) => s.to_string(),
        None => return api_error(StatusCode::BAD_REQUEST, "empty_input", "input field must be a non-empty string"),
    };
    match state.supervisor_tx.send(CoreMessage::ProvideInput { run_id, input }).await {
        Ok(_) => Json(json!({ "status": "input_provided", "run_id": run_id })).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to send input");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "input_failed", "Could not deliver input to run")
        }
    }
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
