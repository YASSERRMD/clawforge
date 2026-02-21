//! Deepgram TTS provider for clawforge-tts.
//!
//! Implements speech synthesis via the Deepgram Aura API.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Deepgram Aura TTS voices.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeepgramVoice {
    #[default]
    Asteria,   // Warm female
    Orion,     // Deep male
    Luna,      // Soft female
    Stella,    // Upbeat female
    Atlas,     // Authoritative male
    Hera,      // Professional female
    Orca,      // Mature male
    Perseus,   // Warm male
    Angus,     // Irish male
    Arcas,     // Versatile male
    Helios,    // British male
    Hermes,    // Smooth male
    Saturn,    // Deliberate male
    Thalia,    // Lively female
    Io,        // Melodic female
}

impl DeepgramVoice {
    pub fn as_model_name(&self) -> &'static str {
        match self {
            Self::Asteria => "aura-asteria-en",
            Self::Orion   => "aura-orion-en",
            Self::Luna    => "aura-luna-en",
            Self::Stella  => "aura-stella-en",
            Self::Atlas   => "aura-atlas-en",
            Self::Hera    => "aura-hera-en",
            Self::Orca    => "aura-orca-en",
            Self::Perseus => "aura-perseus-en",
            Self::Angus   => "aura-angus-en",
            Self::Arcas   => "aura-arcas-en",
            Self::Helios  => "aura-helios-en",
            Self::Hermes  => "aura-hermes-en",
            Self::Saturn  => "aura-saturn-en",
            Self::Thalia  => "aura-thalia-en",
            Self::Io      => "aura-io-en",
        }
    }
}

/// Deepgram TTS request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepgramTtsRequest {
    pub text: String,
    pub voice: DeepgramVoice,
    /// Output encoding: "linear16" (WAV), "mulaw", "alaw", "mp3", "opus", "flac".
    pub encoding: Option<String>,
    /// Sample rate in Hz. Default 24000.
    pub sample_rate: Option<u32>,
    /// Bit rate for lossy codecs (kbps). Default 128.
    pub bit_rate: Option<u32>,
}

/// Deepgram TTS response.
#[derive(Debug, Clone)]
pub struct DeepgramTtsResponse {
    /// Raw audio bytes.
    pub audio_bytes: Vec<u8>,
    /// Content-Type header from Deepgram.
    pub content_type: String,
    /// Number of characters in the input text.
    pub character_count: usize,
}

/// Deepgram Aura TTS client.
pub struct DeepgramTts {
    client: Client,
    api_key: String,
    base_url: String,
}

impl DeepgramTts {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            base_url: "https://api.deepgram.com/v1/speak".to_string(),
        }
    }

    /// Synthesize text to audio bytes.
    pub async fn synthesize(&self, req: DeepgramTtsRequest) -> Result<DeepgramTtsResponse> {
        let encoding = req.encoding.as_deref().unwrap_or("mp3");
        let sample_rate = req.sample_rate.unwrap_or(24_000);

        let url = format!(
            "{}?model={}&encoding={}&sample_rate={}",
            self.base_url,
            req.voice.as_model_name(),
            encoding,
            sample_rate,
        );

        let char_count = req.text.len();

        #[derive(Serialize)]
        struct Body { text: String }

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&Body { text: req.text })
            .send()
            .await
            .context("Deepgram TTS request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Deepgram TTS error {status}: {body}");
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("audio/mp3")
            .to_string();

        let audio_bytes = resp.bytes().await?.to_vec();

        Ok(DeepgramTtsResponse {
            audio_bytes,
            content_type,
            character_count: char_count,
        })
    }
}
