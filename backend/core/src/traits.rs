use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::message::Message;

/// Trait for all ClawForge runtime components (Scheduler, Planner, Executor, Supervisor).
///
/// Each component receives messages from its channel and runs in its own Tokio task.
#[async_trait]
pub trait Component: Send + Sync + 'static {
    /// Human-readable name of this component.
    fn name(&self) -> &str;

    /// Start the component's event loop, consuming from the given receiver.
    async fn start(&self, rx: mpsc::Receiver<Message>) -> Result<()>;
}

/// Trait for LLM providers used by the planner.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider name (e.g., "openrouter", "ollama").
    fn name(&self) -> &str;

    /// Send a completion request and return the response text.
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse>;
}

/// Request to an LLM provider.
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

/// Response from an LLM provider.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub provider: String,
    pub model: String,
    pub tokens_used: u64,
    pub latency_ms: u64,
}
