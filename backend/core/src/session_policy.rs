//! Session-level policies and model overrides.
//!
//! Mirrors `src/sessions/send-policy.ts` and `model-override.ts`.

use serde::{Deserialize, Serialize};

/// Who can send messages to a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SendPolicy {
    /// Any authenticated user.
    Anyone,
    /// Only users explicitly on the allowlist.
    AllowlistOnly,
    /// Only the session owner.
    OwnerOnly,
    /// No external senders — agent-only session.
    AgentOnly,
}

impl Default for SendPolicy {
    fn default() -> Self {
        SendPolicy::Anyone
    }
}

/// Model override for a session — overrides the agent's default model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOverride {
    /// Provider ID (e.g., "anthropic", "openai").
    pub provider: String,
    /// Model ID (e.g., "claude-3-5-sonnet-20241022").
    pub model: String,
    /// Optional temperature override.
    pub temperature: Option<f32>,
    /// Optional max tokens override.
    pub max_tokens: Option<u32>,
}

/// Input provenance: where did this input come from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InputProvenance {
    /// Direct user message.
    User,
    /// Message forwarded from another agent.
    Agent,
    /// Message injected by a channel adapter.
    Channel,
    /// Synthetic message created by the system.
    System,
    /// Subagent result passed back.
    Subagent,
}

/// Session policy: controls who can interact and with what model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPolicy {
    pub send_policy: SendPolicy,
    /// Explicit allow-list of user IDs / phone numbers / channel handles.
    #[serde(default)]
    pub allowed_senders: Vec<String>,
    /// Model override for this session.
    pub model_override: Option<ModelOverride>,
    /// Maximum number of messages in this session.
    pub max_messages: Option<u32>,
    /// Maximum input tokens per message.
    pub max_input_tokens: Option<u32>,
    /// Allow sub-agent spawning from within this session.
    #[serde(default = "default_true")]
    pub allow_subagents: bool,
}

fn default_true() -> bool { true }

impl Default for SessionPolicy {
    fn default() -> Self {
        Self {
            send_policy: SendPolicy::default(),
            allowed_senders: Vec::new(),
            model_override: None,
            max_messages: None,
            max_input_tokens: None,
            allow_subagents: true,
        }
    }
}

impl SessionPolicy {
    /// Check if a sender is permitted to send to this session.
    pub fn is_permitted(&self, sender_id: &str) -> bool {
        match self.send_policy {
            SendPolicy::Anyone => true,
            SendPolicy::AgentOnly => false,
            SendPolicy::OwnerOnly => {
                self.allowed_senders.first().map(|s| s == sender_id).unwrap_or(false)
            }
            SendPolicy::AllowlistOnly => {
                self.allowed_senders.iter().any(|s| s == sender_id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anyone_policy_allows_all() {
        let policy = SessionPolicy::default();
        assert!(policy.is_permitted("any-user"));
    }

    #[test]
    fn allowlist_policy_blocks_unknown() {
        let policy = SessionPolicy {
            send_policy: SendPolicy::AllowlistOnly,
            allowed_senders: vec!["user-1".to_string()],
            ..Default::default()
        };
        assert!(policy.is_permitted("user-1"));
        assert!(!policy.is_permitted("user-2"));
    }

    #[test]
    fn agent_only_blocks_everyone() {
        let policy = SessionPolicy {
            send_policy: SendPolicy::AgentOnly,
            ..Default::default()
        };
        assert!(!policy.is_permitted("any-user"));
    }
}
