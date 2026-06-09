use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use clawforge_core::{LlmProvider, LlmRequest, LlmResponse};

/// Native Anthropic provider using the Messages API.
///
/// Anthropic does not use the OpenAI chat-completions wire format, so it gets a
/// dedicated client: `POST {base}/v1/messages` with the `x-api-key` and
/// `anthropic-version` headers, a top-level `system` field, and a `messages`
/// array.
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
    version: String,
}

impl AnthropicProvider {
    /// Build a provider from an API key (uses the public API base URL).
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com".to_string(),
            version: "2023-06-01".to_string(),
        }
    }

    /// Override the base URL (e.g. a gateway or Bedrock-style proxy).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "String::is_empty")]
    system: String,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let start = Instant::now();

        let body = MessagesRequest {
            model: request.model.clone(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            system: request.system_prompt.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: request.user_prompt.clone(),
            }],
        };

        debug!(model = %request.model, "Sending request to Anthropic");

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url.trim_end_matches('/')))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.version)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Anthropic HTTP request failed")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic returned {}: {}", status, error_body);
        }

        let parsed: MessagesResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        let content = parsed
            .content
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("");

        let tokens_used = parsed
            .usage
            .map(|u| u.input_tokens + u.output_tokens)
            .unwrap_or(0);
        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(LlmResponse {
            content,
            provider: "anthropic".to_string(),
            model: request.model.clone(),
            tokens_used,
            latency_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_anthropic() {
        assert_eq!(AnthropicProvider::new("sk-test").name(), "anthropic");
    }
}
