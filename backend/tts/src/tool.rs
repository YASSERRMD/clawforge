/// TTS Tool — allows agents to reply with audio output.
///
/// Wraps the TTS engine and returns a base64-encoded audio payload
/// that can be sent to channel adapters that support audio.
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};

use crate::engine::{TtsProvider, TtsRequest};

/// Input parameters for the TTS tool.
#[derive(Debug, Deserialize)]
pub struct TtsToolInput {
    pub text: String,
    pub voice: Option<String>,
    pub format: Option<String>,
    pub speed: Option<f32>,
}

/// Output from the TTS tool.
#[derive(Debug, Serialize)]
pub struct TtsToolOutput {
    /// Base64-encoded audio bytes
    pub audio_base64: String,
    /// MIME type (e.g. audio/mpeg)
    pub mime_type: String,
    /// Number of characters synthesized
    pub char_count: usize,
}

/// Run the TTS tool with the given provider and input.
pub async fn run_tts_tool(
    provider: &dyn TtsProvider,
    input: TtsToolInput,
) -> Result<TtsToolOutput> {
    let char_count = input.text.len();
    let format = match input.format.as_deref().unwrap_or("mp3") {
        "opus" => crate::engine::AudioFormat::Opus,
        "aac" => crate::engine::AudioFormat::Aac,
        "flac" => crate::engine::AudioFormat::Flac,
        "pcm" => crate::engine::AudioFormat::Pcm,
        _ => crate::engine::AudioFormat::Mp3,
    };
    let mime = format.mime_type().to_string();

    let req = TtsRequest {
        text: input.text,
        voice: input.voice,
        format,
        speed: input.speed.unwrap_or(1.0),
    };

    let bytes = provider.synthesize(req).await?;
    let audio_base64 = general_purpose::STANDARD.encode(&bytes);

    Ok(TtsToolOutput {
        audio_base64,
        mime_type: mime,
        char_count,
    })
}
