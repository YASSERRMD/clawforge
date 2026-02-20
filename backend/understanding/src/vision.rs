/// Vision understanding — describe images using a vision LLM.
///
/// Mirrors `src/media-understanding/providers/` from OpenClaw.
use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use tracing::info;

/// Supported vision providers.
pub enum VisionProvider {
    OpenAI { api_key: String, model: String },
    Gemini { api_key: String },
}

impl VisionProvider {
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self::OpenAI { api_key: api_key.into(), model: "gpt-4o".to_string() }
    }
    pub fn gemini(api_key: impl Into<String>) -> Self {
        Self::Gemini { api_key: api_key.into() }
    }
}

/// Describe an image from raw bytes using a vision LLM.
pub async fn describe_image(
    provider: &VisionProvider,
    image_bytes: &[u8],
    mime_type: &str,
    prompt: &str,
) -> Result<String> {
    let b64 = STANDARD.encode(image_bytes);
    match provider {
        VisionProvider::OpenAI { api_key, model } => {
            describe_via_openai(api_key, model, &b64, mime_type, prompt).await
        }
        VisionProvider::Gemini { api_key } => {
            describe_via_gemini(api_key, &b64, mime_type, prompt).await
        }
    }
}

async fn describe_via_openai(
    api_key: &str, model: &str, b64: &str, mime_type: &str, prompt: &str,
) -> Result<String> {
    info!("[Vision] Describing image via OpenAI {}", model);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": [
                { "type": "text", "text": prompt },
                { "type": "image_url",
                  "image_url": { "url": format!("data:{};base64,{}", mime_type, b64) } }
            ]
        }],
        "max_tokens": 512
    });
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("OpenAI vision error: {}", resp.text().await.unwrap_or_default());
    }
    let json: serde_json::Value = resp.json().await?;
    Ok(json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

async fn describe_via_gemini(
    api_key: &str, b64: &str, mime_type: &str, prompt: &str,
) -> Result<String> {
    info!("[Vision] Describing image via Gemini");
    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );
    let body = serde_json::json!({
        "contents": [{ "parts": [
            { "text": prompt },
            { "inlineData": { "mimeType": mime_type, "data": b64 } }
        ]}]
    });
    let resp = client.post(&url).json(&body).send().await?;
    if !resp.status().is_success() {
        bail!("Gemini vision error: {}", resp.text().await.unwrap_or_default());
    }
    let json: serde_json::Value = resp.json().await?;
    Ok(json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string())
}
