use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use clawforge_core::{LlmProvider, LlmRequest, LlmResponse};

/// A generic provider for any service that exposes an OpenAI-compatible
/// `/chat/completions` endpoint.
///
/// The overwhelming majority of hosted model providers (OpenAI, Google Gemini's
/// compatibility endpoint, Mistral, xAI, Groq, Together, Fireworks, and the major
/// Chinese providers such as DeepSeek, Qwen/DashScope, Zhipu GLM, Moonshot/Kimi,
/// Baidu ERNIE, MiniMax, Tencent Hunyuan, StepFun, Baichuan, and iFlytek Spark)
/// speak this protocol, so a single client covers them all. Pass the provider
/// id, its base URL, and the API key.
pub struct OpenAiCompatibleProvider {
    client: Client,
    name: String,
    api_key: String,
    base_url: String,
}

impl OpenAiCompatibleProvider {
    /// Build a provider for the given id, base URL, and API key.
    pub fn new(name: impl Into<String>, base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            name: name.into(),
            api_key: api_key.into(),
            base_url: base_url.into(),
        }
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
impl LlmProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        &self.name
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

        debug!(provider = %self.name, model = %request.model, "Sending OpenAI-compatible request");

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .with_context(|| format!("{} HTTP request failed", self.name))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("{} returned {}: {}", self.name, status, error_body);
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .with_context(|| format!("Failed to parse {} response", self.name))?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let tokens_used = chat_response.usage.and_then(|u| u.total_tokens).unwrap_or(0);
        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(LlmResponse {
            content,
            provider: self.name.clone(),
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
    fn name_is_reported() {
        let p = OpenAiCompatibleProvider::new("deepseek", "https://api.deepseek.com/v1", "sk-test");
        assert_eq!(p.name(), "deepseek");
    }
}
