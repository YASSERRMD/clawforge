//! Active WebSocket Session Registry.
//!
//! Tracks connected clients and routes messages to them.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::ws_protocol::WsMessage;

pub type SessionId = String;
pub type ClientSender = mpsc::UnboundedSender<WsMessage>;

/// Manages active WebSocket connections.
#[derive(Clone)]
pub struct SessionRegistry {
    sessions: Arc<RwLock<HashMap<SessionId, ClientSender>>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new active session.
    pub async fn register(&self, session_id: SessionId, sender: ClientSender) {
        let mut w = self.sessions.write().await;
        w.insert(session_id, sender);
    }

    /// Unregister a disconnected session.
    pub async fn unregister(&self, session_id: &SessionId) {
        let mut w = self.sessions.write().await;
        w.remove(session_id);
    }

    /// Send a message to a specific session.
    /// Returns false and removes the session if the channel is closed (client disconnected).
    pub async fn send_to(&self, session_id: &SessionId, msg: WsMessage) -> bool {
        let r = self.sessions.read().await;
        if let Some(sender) = r.get(session_id) {
            if sender.send(msg).is_ok() {
                return true;
            }
        } else {
            return false;
        }
        // Channel was closed — upgrade to write lock and remove the dead session.
        drop(r);
        let mut w = self.sessions.write().await;
        w.remove(session_id);
        false
    }

    /// Remove all sessions whose send channels have been closed.
    pub async fn prune_dead_sessions(&self) {
        let mut w = self.sessions.write().await;
        w.retain(|_, sender| !sender.is_closed());
    }

    /// Current number of active sessions.
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}
