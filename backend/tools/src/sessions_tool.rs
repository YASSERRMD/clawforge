//! Sessions tool — list, send to, read history from agent sessions.
//!
//! Mirrors `src/agents/tools/sessions-*.ts`.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// A session listing entry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEntry {
    pub session_id: String,
    pub agent: String,
    pub channel: Option<String>,
    pub status: SessionStatus,
    pub last_activity: Option<DateTime<Utc>>,
    pub message_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Idle,
    Paused,
    Stopped,
}

/// Input to list sessions.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSessionsInput {
    /// Filter by agent name.
    pub agent: Option<String>,
    /// Filter by channel.
    pub channel: Option<String>,
    /// Max sessions to return.
    pub limit: Option<usize>,
}

/// Input to send a message to a session.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendToSessionInput {
    pub session_id: String,
    pub message: String,
    /// If true, wait for the agent's response before returning.
    pub wait_for_response: Option<bool>,
    pub timeout_secs: Option<u64>,
}

/// Output from send-to-session.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendToSessionOutput {
    pub ok: bool,
    pub response: Option<String>,
    pub timed_out: bool,
}

/// A transcript entry (history item).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptEntry {
    pub role: String, // "user" | "assistant" | "tool"
    pub content: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub tool_name: Option<String>,
}

/// Input to read session history.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistoryInput {
    pub session_id: String,
    pub limit: Option<usize>,
    pub before: Option<DateTime<Utc>>,
}

/// Output from session history.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistoryOutput {
    pub session_id: String,
    pub entries: Vec<TranscriptEntry>,
    pub total: usize,
}

/// Input to spawn a new session.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnSessionInput {
    pub agent: String,
    pub message: Option<String>,
    pub channel: Option<String>,
}

/// Output from spawn-session.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnSessionOutput {
    pub session_id: String,
    pub status: SessionStatus,
}

/// Input to get session status.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusInput {
    pub session_id: String,
}

/// Trait for session management backends.
#[async_trait::async_trait]
pub trait SessionBackend: Send + Sync {
    async fn list(&self, input: ListSessionsInput) -> Result<Vec<SessionEntry>>;
    async fn send(&self, input: SendToSessionInput) -> Result<SendToSessionOutput>;
    async fn history(&self, input: SessionHistoryInput) -> Result<SessionHistoryOutput>;
    async fn spawn(&self, input: SpawnSessionInput) -> Result<SpawnSessionOutput>;
    async fn status(&self, input: SessionStatusInput) -> Result<Option<SessionEntry>>;
}
