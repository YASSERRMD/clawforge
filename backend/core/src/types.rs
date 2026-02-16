use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Specification of an agent's capabilities and behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub trigger: TriggerSpec,
    pub capabilities: Capabilities,
    pub llm_policy: LlmPolicy,
    pub workflow: Vec<WorkflowStep>,
}

impl AgentSpec {
    pub fn new(name: impl Into<String>, trigger: TriggerSpec) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: String::new(),
            trigger,
            capabilities: Capabilities::default(),
            llm_policy: LlmPolicy::default(),
            workflow: Vec::new(),
        }
    }
}

/// How an agent is triggered (cron, interval, webhook, or manual).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerSpec {
    /// Cron expression (e.g., "*/30 * * * *")
    Cron { expression: String },
    /// Fixed interval in seconds
    Interval { seconds: u64 },
    /// HTTP webhook trigger
    Webhook { path: String },
    /// Manually triggered
    Manual,
}

/// What the agent is allowed to do.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Capabilities {
    pub can_read_files: bool,
    pub can_write_files: bool,
    pub can_execute_commands: bool,
    pub can_make_http_requests: bool,
    pub allowed_domains: Vec<String>,
    pub max_tokens_per_run: Option<u64>,
    pub max_cost_per_run_usd: Option<f64>,
}

/// LLM provider selection policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPolicy {
    /// Provider names to use (raced in parallel)
    pub providers: Vec<String>,
    /// Model to request from each provider
    pub model: String,
    /// Maximum tokens for completion
    pub max_tokens: u32,
    /// Temperature for generation
    pub temperature: f32,
    /// System prompt
    pub system_prompt: String,
}

impl Default for LlmPolicy {
    fn default() -> Self {
        Self {
            providers: vec!["openrouter".to_string()],
            model: "openai/gpt-4o-mini".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            system_prompt: String::new(),
        }
    }
}

/// A single step in an agent's workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub action: ActionType,
    pub on_failure: FailurePolicy,
}

/// The type of action a workflow step performs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionType {
    /// Send a prompt to the LLM planner
    LlmPrompt { prompt_template: String },
    /// Execute a shell command
    ShellCommand { command: String, args: Vec<String> },
    /// Make an HTTP request
    HttpRequest {
        method: String,
        url: String,
        headers: std::collections::HashMap<String, String>,
        body: Option<String>,
    },
}

/// What to do when a workflow step fails.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FailurePolicy {
    #[default]
    Stop,
    Retry { max_attempts: u32 },
    Skip,
    Replan,
}

impl fmt::Display for TriggerSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriggerSpec::Cron { expression } => write!(f, "cron({})", expression),
            TriggerSpec::Interval { seconds } => write!(f, "every {}s", seconds),
            TriggerSpec::Webhook { path } => write!(f, "webhook({})", path),
            TriggerSpec::Manual => write!(f, "manual"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_spec_creation() {
        let agent = AgentSpec::new("test-agent", TriggerSpec::Manual);
        assert_eq!(agent.name, "test-agent");
        assert!(!agent.id.is_nil());
    }

    #[test]
    fn test_agent_spec_serialization() {
        let agent = AgentSpec::new(
            "pr-reviewer",
            TriggerSpec::Cron {
                expression: "*/30 * * * *".to_string(),
            },
        );
        let json = serde_json::to_string(&agent).unwrap();
        let deserialized: AgentSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "pr-reviewer");
    }

    #[test]
    fn test_trigger_display() {
        assert_eq!(
            TriggerSpec::Cron {
                expression: "*/5 * * * *".into()
            }
            .to_string(),
            "cron(*/5 * * * *)"
        );
        assert_eq!(TriggerSpec::Manual.to_string(), "manual");
    }

    #[test]
    fn test_capabilities_default() {
        let caps = Capabilities::default();
        assert!(!caps.can_read_files);
        assert!(!caps.can_execute_commands);
        assert!(caps.allowed_domains.is_empty());
    }
}
