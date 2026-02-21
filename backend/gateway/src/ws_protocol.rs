//! WebSocket Protocol for ClawForge Gateway.
//!
//! Mirrors `src/gateway/events.ts` and protocol definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The types of messages exchanged over the Gateway WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Client -> Server: Ping to keep connection alive
    Ping,
    /// Server -> Client: Pong response
    Pong,
    /// Client -> Server: Invoke a specific agent session
    Invoke {
        session_id: String,
        agent_id: String,
        content: String,
    },
    /// Server -> Client: A result/response from the agent
    Result {
        session_id: String,
        content: String,
    },
    /// Server -> Client: An error occurred
    Error {
        session_id: Option<String>,
        error_code: String,
        message: String,
    },
    /// Server -> Client: State change (e.g. typing, stopped)
    StateChange {
        session_id: String,
        state: String,
    },
}
