//! WebSocket entrypoint and connection handler.
//!
//! Upgrades HTTP to WS and handles the connection loop.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use tracing::{debug, error, info, warn};

use clawforge_core::{Message as CoreMessage, message::JobTrigger};
use uuid::Uuid;

use crate::server::GatewayState;
use crate::ws_protocol::WsMessage;
use tokio::sync::mpsc;
use futures::{sink::SinkExt, stream::StreamExt};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<GatewayState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_connection(socket, state))
}

async fn handle_connection(socket: WebSocket, state: GatewayState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    // Forward from bounded app sender to actual websocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    error!(error = %e, "Failed to serialize WebSocket message; closing connection");
                    break;
                }
            };
            if sender.send(Message::Text(json.into())).await.is_err() {
                debug!("WebSocket send failed — client disconnected");
                break;
            }
        }
    });

    // Receive from websocket and route to app
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        handle_incoming_message(ws_msg, &tx, &state_clone).await;
                    } else {
                        warn!("Received invalid JSON message: {}", text);
                    }
                }
                Message::Close(_) => {
                    break;
                }
                _ => {} // Ignore binary, ping, pong for now
            }
        }
    });

    // If either task exits, abort the other.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
    
    info!("WebSocket connection closed");
}

async fn handle_incoming_message(
    msg: WsMessage,
    reply_tx: &mpsc::UnboundedSender<WsMessage>,
    state: &GatewayState,
) {
    match msg {
        WsMessage::Ping => {
            if reply_tx.send(WsMessage::Pong).is_err() {
                warn!("Failed to send Pong — receiver dropped");
            }
        }
        WsMessage::Invoke { session_id, agent_id, content } => {
            info!(session_id = %session_id, agent_id = %agent_id, "Received Invoke — dispatching to scheduler");
            let parsed_agent_id = match Uuid::parse_str(&agent_id) {
                Ok(id) => id,
                Err(_) => {
                    if reply_tx.send(WsMessage::Error {
                        session_id: Some(session_id),
                        error_code: "invalid_agent_id".to_string(),
                        message: format!("agent_id '{}' is not a valid UUID", agent_id),
                    }).is_err() {
                        warn!("Failed to send Error — receiver dropped");
                    }
                    return;
                }
            };
            match &state.scheduler_tx {
                Some(tx) => {
                    let run_id = Uuid::new_v4();
                    let trigger = JobTrigger {
                        run_id,
                        agent_id: parsed_agent_id,
                        trigger_reason: format!("WebSocket Invoke from session {}: {}", session_id, content),
                    };
                    if let Err(e) = tx.send(CoreMessage::ScheduleJob(trigger)).await {
                        error!(error = %e, "Failed to dispatch Invoke to scheduler");
                        if reply_tx.send(WsMessage::Error {
                            session_id: Some(session_id),
                            error_code: "scheduler_unavailable".to_string(),
                            message: "Scheduler is not reachable".to_string(),
                        }).is_err() {
                            warn!("Failed to send Error — receiver dropped");
                        }
                    } else if reply_tx.send(WsMessage::StateChange {
                        session_id,
                        state: format!("scheduled:{}", run_id),
                    }).is_err() {
                        warn!("Failed to send StateChange — receiver dropped");
                    }
                }
                None => {
                    warn!(agent_id = %agent_id, "No scheduler connected — Invoke ignored");
                    if reply_tx.send(WsMessage::Error {
                        session_id: Some(session_id),
                        error_code: "scheduler_unavailable".to_string(),
                        message: "No scheduler is connected to this gateway".to_string(),
                    }).is_err() {
                        warn!("Failed to send Error — receiver dropped");
                    }
                }
            }
        }
        _ => warn!("Received unexpected message type from client"),
    }
}
