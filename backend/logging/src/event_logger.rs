//! Agent Event Logger
//!
//! Structured events (tool_call, message, error) written to rolling NDJSON logs.

use chrono::{DateTime, Utc};
use serde::Serialize;
use tracing::info;

use crate::redact::redact_sensitive_data;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    ToolCall {
        tool_name: String,
        arguments_json: String,
    },
    Message {
        role: String,
        content: String,
    },
    Error {
        error_msg: String,
    }
}

#[derive(Debug, Serialize)]
pub struct EventLogEntry {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub event: AgentEvent,
}

pub struct EventLogger;

impl EventLogger {
    /// Logs an agent's runtime event securely, immediately serializing it to the tracing system.
    pub fn log_event(session_id: &str, mut event: AgentEvent) {
        
        // Redact any string contents before logging
        match &mut event {
            AgentEvent::ToolCall { arguments_json, .. } => {
                *arguments_json = redact_sensitive_data(arguments_json);
            }
            AgentEvent::Message { content, .. } => {
                *content = redact_sensitive_data(content);
            }
            AgentEvent::Error { error_msg } => {
                *error_msg = redact_sensitive_data(error_msg);
            }
        }

        let entry = EventLogEntry {
            session_id: session_id.into(),
            timestamp: Utc::now(),
            event,
        };

        // Leverage tracing to output NDJSON correctly wrapped
        info!(target: "agent_events", event = ?entry, "Agent trace event");
    }
}
