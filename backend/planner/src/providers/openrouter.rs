use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use clawforge_core::{LlmProvider, LlmRequest, LlmResponse};

/// OpenRouter.ai LLM provider.
pub struct OpenRouterProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenRouterProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct Usage {
    total_tokens: Option<u64>,
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let start = Instant::now();

        let mut messages = Vec::new();
        if !request.system_prompt.is_empty() {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: request.system_prompt.clone(),
            });
        }
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: request.user_prompt.clone(),
        });

        let body = ChatRequest {
            model: request.model.clone(),
            messages,
            max_tokens: Some(request.max_tokens),
            temperature: Some(request.temperature),
        };

        debug!(
            model = %request.model,
            "Sending request to OpenRouter"
        );

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("OpenRouter HTTP request failed")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenRouter returned {}: {}", status, error_body);
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse OpenRouter response")?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let tokens_used = chat_response
            .usage
            .and_then(|u| u.total_tokens)
            .unwrap_or(0);

        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(LlmResponse {
            content,
            provider: "openrouter".to_string(),
            model: request.model.clone(),
            tokens_used,
            latency_ms,
        })
    }
}
