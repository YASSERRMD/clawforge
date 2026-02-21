/// Hook lifecycle phases.
///
/// Mirrors `src/hooks/types.ts` from OpenClaw.
/// Hooks fire at specific points in the agent run pipeline.
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Hook phases
// ---------------------------------------------------------------------------

/// The lifecycle phase at which a hook fires.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookPhase {
    /// Before an inbound message is processed.
    PreMessage,
    /// After the agent produces a reply, before delivery.
    PostMessage,
    /// Before a tool call is executed.
    PreToolCall,
    /// After a tool call completes (success or error).
    AfterToolCall,
    /// Before context compaction starts.
    PreCompaction,
    /// After context compaction completes.
    PostCompaction,
    /// When a new agent session starts.
    SessionStart,
    /// When an agent session ends (normally or by cancellation).
    SessionEnd,
    /// Model override — dynamically change the model for a run.
    ModelOverride,
}

// ---------------------------------------------------------------------------
// Payload carried into each hook
// ---------------------------------------------------------------------------

/// Payload passed to Pre/Post message hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    pub session_id: String,
    pub channel: String,
    pub role: String,
    pub content: String,
    pub metadata: serde_json::Value,
}

/// Payload passed to Pre/After tool-call hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallPayload {
    pub session_id: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    /// `None` for PreToolCall; populated for AfterToolCall.
    pub tool_output: Option<serde_json::Value>,
    /// True if the tool call resulted in an error.
    pub is_error: bool,
}

/// Payload for compaction hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionPayload {
    pub session_id: String,
    pub original_turn_count: usize,
    pub retained_turn_count: usize,
    pub summary: Option<String>,
}

/// Payload for session start/end hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPayload {
    pub session_id: String,
    pub channel: String,
    pub agent_id: String,
    /// True for SessionEnd; false for SessionStart.
    pub is_end: bool,
    /// Exit reason for SessionEnd (e.g. "completed", "cancelled", "error").
    pub exit_reason: Option<String>,
}

/// Payload for the model override hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelOverridePayload {
    pub session_id: String,
    pub requested_model: String,
}

/// Union payload type passed to all hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum HookPayload {
    PreMessage(MessagePayload),
    PostMessage(MessagePayload),
    PreToolCall(ToolCallPayload),
    AfterToolCall(ToolCallPayload),
    PreCompaction(CompactionPayload),
    PostCompaction(CompactionPayload),
    SessionStart(SessionPayload),
    SessionEnd(SessionPayload),
    ModelOverride(ModelOverridePayload),
}

impl HookPayload {
    pub fn phase(&self) -> HookPhase {
        match self {
            Self::PreMessage(_) => HookPhase::PreMessage,
            Self::PostMessage(_) => HookPhase::PostMessage,
            Self::PreToolCall(_) => HookPhase::PreToolCall,
            Self::AfterToolCall(_) => HookPhase::AfterToolCall,
            Self::PreCompaction(_) => HookPhase::PreCompaction,
            Self::PostCompaction(_) => HookPhase::PostCompaction,
            Self::SessionStart(_) => HookPhase::SessionStart,
            Self::SessionEnd(_) => HookPhase::SessionEnd,
            Self::ModelOverride(_) => HookPhase::ModelOverride,
        }
    }
}

// ---------------------------------------------------------------------------
// Hook result
// ---------------------------------------------------------------------------

/// Result returned by a hook.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookResult {
    /// If set, replace the payload content (e.g. transformed message text).
    pub modified_content: Option<String>,
    /// If set, override the model for this run.
    pub model_override: Option<String>,
    /// If true, abort the pipeline (e.g. block a tool call).
    pub abort: bool,
    /// Optional human-readable reason for abortion or modification.
    pub reason: Option<String>,
}

impl HookResult {
    pub fn pass() -> Self {
        Self::default()
    }

    pub fn abort(reason: impl Into<String>) -> Self {
        Self { abort: true, reason: Some(reason.into()), ..Default::default() }
    }

    pub fn transform(content: impl Into<String>) -> Self {
        Self { modified_content: Some(content.into()), ..Default::default() }
    }
}

// ---------------------------------------------------------------------------
// Hook trigger (used by evaluator)
// ---------------------------------------------------------------------------

/// When a hook should fire.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum HookTrigger {
    Always,
    OnEvent { event_name: String },
    OnCondition { condition: HookCondition },
    OnPattern { pattern: String },
    OnSchedule { cron: String },
}

/// A structured condition for hook triggering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum HookCondition {
    FieldEquals { field: String, value: serde_json::Value },
    FieldContains { field: String, substring: String },
    FieldMatches { field: String, regex: String },
    And { conditions: Vec<HookCondition> },
    Or { conditions: Vec<HookCondition> },
    Not { condition: Box<HookCondition> },
}

/// Runtime context passed to the hook evaluator.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookContext {
    pub event_name: Option<String>,
    pub message_text: Option<String>,
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}
