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
            info!(session_id = %session_id, agent_id = %agent_id, "Received Invoke");
            // TODO: dispatch to AgentRunner via bus
            if reply_tx.send(WsMessage::Result {
                session_id,
                content: format!("Echoing: {}", content),
            }).is_err() {
                warn!("Failed to send Result — receiver dropped");
            }
        }
        _ => warn!("Received unexpected message type from client"),
    }
}
