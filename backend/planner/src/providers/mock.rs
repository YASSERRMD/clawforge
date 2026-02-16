use anyhow::Result;
use async_trait::async_trait;
use clawforge_core::{LlmProvider, LlmRequest, LlmResponse};

/// A mock LLM provider that returns canned responses.
pub struct MockProvider {
    name: String,
    fixed_response: Option<String>,
}

impl MockProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fixed_response: None,
        }
    }

    pub fn with_response(mut self, response: impl Into<String>) -> Self {
        self.fixed_response = Some(response.into());
        self
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn complete(&self, _req: &LlmRequest) -> Result<LlmResponse> {
        Ok(LlmResponse {
            content: self.fixed_response.clone().unwrap_or_else(|| "Mock response".to_string()),
            provider: self.name.clone(),
            model: "mock".to_string(),
            tokens_used: 0,
            latency_ms: 0,
        })
    }
}
