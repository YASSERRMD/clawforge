/// Image generation tool.
///
/// Mirrors `src/agents/tools/image-tool.ts` from OpenClaw.
/// Supports DALL·E 3 (OpenAI) and Stable Diffusion via Replicate.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ImageProvider {
    DallE3 { api_key: String },
    Replicate { api_key: String, model_version: String },
}

impl ImageProvider {
    pub fn dalle3(api_key: impl Into<String>) -> Self {
        Self::DallE3 { api_key: api_key.into() }
    }
    pub fn replicate(api_key: impl Into<String>, model_version: impl Into<String>) -> Self {
        Self::Replicate { api_key: api_key.into(), model_version: model_version.into() }
    }
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenInput {
    pub prompt: String,
    pub size: Option<String>, // "1024x1024", "1024x1792", "1792x1024"
    pub quality: Option<String>, // "standard" | "hd"
    pub style: Option<String>,  // "vivid" | "natural"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenOutput {
    pub url: Option<String>,
    pub b64_json: Option<String>,
    pub revised_prompt: Option<String>,
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

pub async fn generate_image(
    provider: &ImageProvider,
    input: &ImageGenInput,
) -> Result<ImageGenOutput> {
    match provider {
        ImageProvider::DallE3 { api_key } => generate_dalle3(api_key, input).await,
        ImageProvider::Replicate { api_key, model_version } => {
            generate_replicate(api_key, model_version, input).await
        }
    }
}

async fn generate_dalle3(api_key: &str, input: &ImageGenInput) -> Result<ImageGenOutput> {
    info!("[ImageGen] DALL·E 3 — prompt: {:.80}", input.prompt);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "dall-e-3",
        "prompt": input.prompt,
        "n": 1,
        "size": input.size.as_deref().unwrap_or("1024x1024"),
        "quality": input.quality.as_deref().unwrap_or("standard"),
        "style": input.style.as_deref().unwrap_or("vivid"),
    });
    let resp = client
        .post("https://api.openai.com/v1/images/generations")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("DALL·E 3 error: {}", resp.text().await.unwrap_or_default());
    }
    let json: serde_json::Value = resp.json().await?;
    let data = &json["data"][0];
    Ok(ImageGenOutput {
        url: data["url"].as_str().map(str::to_string),
        b64_json: data["b64_json"].as_str().map(str::to_string),
        revised_prompt: data["revised_prompt"].as_str().map(str::to_string),
    })
}

async fn generate_replicate(
    api_key: &str, model_version: &str, input: &ImageGenInput,
) -> Result<ImageGenOutput> {
    info!("[ImageGen] Replicate model {} — prompt: {:.80}", model_version, input.prompt);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "version": model_version,
        "input": {
            "prompt": input.prompt,
            "width": 1024,
            "height": 1024,
        }
    });
    let resp = client
        .post("https://api.replicate.com/v1/predictions")
        .header("Authorization", format!("Token {}", api_key))
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("Replicate error: {}", resp.text().await.unwrap_or_default());
    }
    let json: serde_json::Value = resp.json().await?;
    let url = json["output"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Ok(ImageGenOutput { url, b64_json: None, revised_prompt: None })
}
