/// TTS provider trait and implementations (ElevenLabs + OpenAI TTS).
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use reqwest::Client;
use serde::Serialize;
use tracing::info;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Audio format for TTS output.
#[derive(Debug, Clone, Default)]
pub enum AudioFormat {
    #[default]
    Mp3,
    Opus,
    Aac,
    Flac,
    Pcm,
}

impl AudioFormat {
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Opus => "audio/opus",
            Self::Aac => "audio/aac",
            Self::Flac => "audio/flac",
            Self::Pcm => "audio/pcm",
        }
    }

    pub fn openai_str(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Opus => "opus",
            Self::Aac => "aac",
            Self::Flac => "flac",
            Self::Pcm => "pcm",
        }
    }
}

/// A TTS request.
#[derive(Debug, Clone)]
pub struct TtsRequest {
    pub text: String,
    pub voice: Option<String>,
    pub format: AudioFormat,
    pub speed: f32,
}

impl Default for TtsRequest {
    fn default() -> Self {
        Self {
            text: String::new(),
            voice: None,
            format: AudioFormat::Mp3,
            speed: 1.0,
        }
    }
}

/// Returns raw audio bytes.
#[async_trait]
pub trait TtsProvider: Send + Sync {
    async fn synthesize(&self, req: TtsRequest) -> Result<Bytes>;
}

// ---------------------------------------------------------------------------
// OpenAI TTS
// ---------------------------------------------------------------------------

pub struct OpenAiTts {
    api_key: String,
    model: String,
    default_voice: String,
    client: Client,
}

impl OpenAiTts {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "tts-1".to_string(),
            default_voice: "nova".to_string(),
            client: Client::new(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_voice(mut self, voice: impl Into<String>) -> Self {
        self.default_voice = voice.into();
        self
    }
}

#[derive(Serialize)]
struct OpenAiTtsBody {
    model: String,
    input: String,
    voice: String,
    response_format: String,
    speed: f32,
}

#[async_trait]
impl TtsProvider for OpenAiTts {
    async fn synthesize(&self, req: TtsRequest) -> Result<Bytes> {
        let body = OpenAiTtsBody {
            model: self.model.clone(),
            input: req.text,
            voice: req.voice.unwrap_or_else(|| self.default_voice.clone()),
            response_format: req.format.openai_str().to_string(),
            speed: req.speed,
        };
        info!("[TTS/OpenAI] Synthesizing with model={}", body.model);
        let bytes = self
            .client
            .post("https://api.openai.com/v1/audio/speech")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(bytes)
    }
}

// ---------------------------------------------------------------------------
// ElevenLabs TTS
// ---------------------------------------------------------------------------

pub struct ElevenLabsTts {
    api_key: String,
    default_voice_id: String,
    client: Client,
}

impl ElevenLabsTts {
    pub fn new(api_key: String, voice_id: Option<String>) -> Self {
        Self {
            api_key,
            default_voice_id: voice_id.unwrap_or_else(|| "21m00Tcm4TlvDq8ikWAM".to_string()), // Rachel
            client: Client::new(),
        }
    }
}

#[derive(Serialize)]
struct ElevenLabsBody {
    text: String,
    model_id: String,
    voice_settings: ElevenLabsVoiceSettings,
}

#[derive(Serialize)]
struct ElevenLabsVoiceSettings {
    stability: f32,
    similarity_boost: f32,
    speed: f32,
}

#[async_trait]
impl TtsProvider for ElevenLabsTts {
    async fn synthesize(&self, req: TtsRequest) -> Result<Bytes> {
        let voice_id = req.voice.as_deref().unwrap_or(&self.default_voice_id);
        let url = format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}/stream",
            voice_id
        );
        let body = ElevenLabsBody {
            text: req.text,
            model_id: "eleven_monolingual_v1".to_string(),
            voice_settings: ElevenLabsVoiceSettings {
                stability: 0.5,
                similarity_boost: 0.75,
                speed: req.speed,
            },
        };
        info!("[TTS/ElevenLabs] Synthesizing voice_id={}", voice_id);
        let bytes = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        Ok(bytes)
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

pub enum TtsProviderKind {
    OpenAi { api_key: String },
    ElevenLabs { api_key: String, voice_id: Option<String> },
}

pub fn create_tts(kind: TtsProviderKind) -> Box<dyn TtsProvider> {
    match kind {
        TtsProviderKind::OpenAi { api_key } => Box::new(OpenAiTts::new(api_key)),
        TtsProviderKind::ElevenLabs { api_key, voice_id } => {
            Box::new(ElevenLabsTts::new(api_key, voice_id))
        }
    }
}
