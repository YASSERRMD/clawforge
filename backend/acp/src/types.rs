/// ACP (Agent Communication Protocol) types shared between client and server.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a sub-agent session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubAgentStatus {
    Starting,
    Running,
    AwaitingPermission,
    Completed,
    Failed,
    Cancelled,
}

/// A spawned sub-agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentSession {
    pub session_id: Uuid,
    pub parent_session_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub depth: usize,
    pub status: SubAgentStatus,
    pub prompt: String,
    pub result: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// ACP permission request from an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub session_id: Uuid,
    pub tool_name: String,
    pub tool_kind: String,
    pub description: String,
    pub auto_approvable: bool,
}

/// ACP permission response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResponse {
    pub approved: bool,
}

/// Request to spawn a new sub-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    pub parent_session_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub prompt: String,
    pub workspace: Option<String>,
}

/// Announcement broadcasted by a sub-agent about its status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentAnnouncement {
    pub session_id: Uuid,
    pub status: SubAgentStatus,
    pub message: Option<String>,
    pub timestamp: i64,
}
