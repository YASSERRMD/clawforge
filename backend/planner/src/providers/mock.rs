use anyhow::Result;
use async_trait::async_trait;
use clawforge_core::{LlmProvider, LlmRequest, LlmResponse};

/// A mock LLM provider that returns canned responses.
pub struct MockProvider {
    name: String,
}

impl MockProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn complete(&self, _req: &LlmRequest) -> Result<LlmResponse> {
        Ok(LlmResponse {
            content: "Mock plan: Execute 'echo hello' step.".to_string(),
            provider: self.name.clone(),
            model: "mock-model".to_string(),
            tokens_used: 10,
            latency_ms: 50,
        })
    }
}
