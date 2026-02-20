/// Audio understanding — transcribe audio using STT providers.
///
/// Mirrors `src/media-understanding/providers/deepgram.ts` from OpenClaw.
use anyhow::{bail, Result};
use tracing::info;

pub enum AudioProvider {
    Whisper { api_key: String },
    Deepgram { api_key: String },
}

impl AudioProvider {
    pub fn whisper(api_key: impl Into<String>) -> Self {
        Self::Whisper { api_key: api_key.into() }
    }
    pub fn deepgram(api_key: impl Into<String>) -> Self {
        Self::Deepgram { api_key: api_key.into() }
    }
}

/// Transcribe audio bytes to text.
pub async fn transcribe_audio(
    provider: &AudioProvider,
    audio_bytes: Vec<u8>,
    mime_type: &str,
) -> Result<String> {
    match provider {
        AudioProvider::Whisper { api_key } => {
            transcribe_whisper(api_key, audio_bytes, mime_type).await
        }
        AudioProvider::Deepgram { api_key } => {
            transcribe_deepgram(api_key, audio_bytes, mime_type).await
        }
    }
}

async fn transcribe_whisper(api_key: &str, audio: Vec<u8>, mime: &str) -> Result<String> {
    info!("[Audio] Transcribing via OpenAI Whisper");
    let ext = if mime.contains("mp3") { "mp3" } else { "wav" };
    let part = reqwest::multipart::Part::bytes(audio)
        .file_name(format!("audio.{}", ext))
        .mime_str(mime)?;
    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .part("file", part);
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("Whisper error: {}", resp.text().await.unwrap_or_default());
    }
    let json: serde_json::Value = resp.json().await?;
    Ok(json["text"].as_str().unwrap_or("").to_string())
}

async fn transcribe_deepgram(api_key: &str, audio: Vec<u8>, mime: &str) -> Result<String> {
    info!("[Audio] Transcribing via Deepgram");
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.deepgram.com/v1/listen?model=nova-2")
        .header("Authorization", format!("Token {}", api_key))
        .header("Content-Type", mime)
        .body(audio)
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("Deepgram error: {}", resp.text().await.unwrap_or_default());
    }
    let json: serde_json::Value = resp.json().await?;
    Ok(json["results"]["channels"][0]["alternatives"][0]["transcript"]
        .as_str()
        .unwrap_or("")
        .to_string())
}
