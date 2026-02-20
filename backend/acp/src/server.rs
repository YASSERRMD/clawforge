/// ACP HTTP server — axum routes exposed by the gateway for sub-agent coordination.
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

use crate::registry::SubAgentRegistry;
use crate::types::{
    PermissionResponse, SpawnRequest, SubAgentAnnouncement, SubAgentSession, SubAgentStatus,
};

#[derive(Clone)]
pub struct AcpServerState {
    pub registry: Arc<SubAgentRegistry>,
    /// Announce broadcast so gateway can relay status to connected WS clients.
    pub announce_tx: mpsc::Sender<SubAgentAnnouncement>,
}

pub fn build_acp_router(state: AcpServerState) -> Router {
    Router::new()
        .route("/api/acp/spawn", post(spawn_handler))
        .route("/api/acp/sessions/:id", get(session_handler))
        .route("/api/acp/sessions/:id/status", post(status_handler))
        .route("/api/acp/sessions/:id/permission", post(permission_handler))
        .route("/api/acp/sessions", get(list_sessions_handler))
        .with_state(state)
}

/// POST /api/acp/spawn — register and start a sub-agent session.
async fn spawn_handler(
    State(state): State<AcpServerState>,
    Json(req): Json<SpawnRequest>,
) -> impl IntoResponse {
    match state
        .registry
        .register(req.parent_session_id, req.agent_id, req.prompt)
        .await
    {
        Ok(session) => {
            let ann = SubAgentAnnouncement {
                session_id: session.session_id,
                status: SubAgentStatus::Starting,
                message: None,
                timestamp: Utc::now().timestamp(),
            };
            let _ = state.announce_tx.send(ann).await;
            (StatusCode::CREATED, Json(session)).into_response()
        }
        Err(e) => {
            error!("[ACP] Spawn failed: {}", e);
            (StatusCode::BAD_REQUEST, e.to_string()).into_response()
        }
    }
}

/// GET /api/acp/sessions/:id — get session status.
async fn session_handler(
    State(state): State<AcpServerState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.registry.get(id).await {
        Some(session) => (StatusCode::OK, Json(session)).into_response(),
        None => (StatusCode::NOT_FOUND, "Session not found").into_response(),
    }
}

/// POST /api/acp/sessions/:id/status — sub-agent posts its own status update.
#[derive(serde::Deserialize)]
struct StatusUpdate {
    status: SubAgentStatus,
    message: Option<String>,
}

async fn status_handler(
    State(state): State<AcpServerState>,
    Path(id): Path<Uuid>,
    Json(body): Json<StatusUpdate>,
) -> impl IntoResponse {
    state
        .registry
        .update_status(id, body.status.clone(), body.message.clone())
        .await;

    let ann = SubAgentAnnouncement {
        session_id: id,
        status: body.status,
        message: body.message,
        timestamp: Utc::now().timestamp(),
    };
    let _ = state.announce_tx.send(ann).await;
    StatusCode::OK
}

/// POST /api/acp/sessions/:id/permission — respond to a permission request.
async fn permission_handler(
    State(state): State<AcpServerState>,
    Path(id): Path<Uuid>,
    Json(body): Json<PermissionResponse>,
) -> impl IntoResponse {
    // Permission decisions are logged; in full impl they'd wake a waiting executor.
    info!(
        "[ACP] Permission for session {}: {}",
        id,
        if body.approved { "approved" } else { "denied" }
    );
    StatusCode::OK
}

/// GET /api/acp/sessions — list all active sessions.
async fn list_sessions_handler(
    State(state): State<AcpServerState>,
) -> impl IntoResponse {
    let sessions: Vec<SubAgentSession> = state.registry.active().await;
    (StatusCode::OK, Json(sessions)).into_response()
}
