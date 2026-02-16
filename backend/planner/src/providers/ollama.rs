use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use clawforge_core::{LlmProvider, LlmRequest, LlmResponse};

/// Ollama local LLM provider.
pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "http://localhost:11434".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Serialize, Deserialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaChatMessage,
    eval_count: Option<u64>,
    prompt_eval_count: Option<u64>,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let start = Instant::now();

        let mut messages = Vec::new();
        if !request.system_prompt.is_empty() {
            messages.push(OllamaChatMessage {
                role: "system".to_string(),
                content: request.system_prompt.clone(),
            });
        }
        messages.push(OllamaChatMessage {
            role: "user".to_string(),
            content: request.user_prompt.clone(),
        });

        // Extract model name (strip any provider prefix like "openai/")
        let model = request
            .model
            .split('/')
            .last()
            .unwrap_or(&request.model)
            .to_string();

        let body = OllamaChatRequest {
            model: model.clone(),
            messages,
            stream: false,
            options: OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            },
        };

        debug!(model = %model, "Sending request to Ollama");

        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .context("Ollama HTTP request failed")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama returned {}: {}", status, error_body);
        }

        let chat_response: OllamaChatResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        let tokens_used = chat_response.eval_count.unwrap_or(0)
            + chat_response.prompt_eval_count.unwrap_or(0);

        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(LlmResponse {
            content: chat_response.message.content,
            provider: "ollama".to_string(),
            model,
            tokens_used,
            latency_ms,
        })
    }
}
