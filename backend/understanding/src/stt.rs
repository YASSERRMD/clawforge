//! Speech-to-Text (STT) Parsing
//!
//! Provides async wrappers for transcribing voice notes and audio attachments
//! utilizing external models like Deepgram, OpenAI Whisper, or local ones.

use anyhow::Result;
use tracing::info;

pub enum SttEngine {
    Deepgram,
    Whisper,
    LocalSilero,
}

pub struct SttService {
    engine: SttEngine,
}

impl SttService {
    pub fn new(engine: SttEngine) -> Self {
        Self { engine }
    }

    /// Uploads an audio blob to the configured STT engine and returns the text transcript.
    pub async fn transcribe_audio(&self, audio_data: &[u8], mime_type: &str) -> Result<String> {
        info!("Transcribing {} bytes of {} audio...", audio_data.len(), mime_type);
        // MOCK: POST to API
        Ok("mock_transcription_text".into())
    }
}
