use crate::{MediaHandler, MediaPayload};
use async_trait::async_trait;
use tracing::info;

pub struct WhisperSttHandler {
    api_key: String,
    model: String,
}

impl WhisperSttHandler {
    pub fn new(api_key: String, model: impl Into<String>) -> Self {
        Self {
            api_key,
            model: model.into(),
        }
    }
}

#[async_trait]
impl MediaHandler for WhisperSttHandler {
    async fn process(&self, payload: &MediaPayload) -> anyhow::Result<String> {
        info!("Transcribing audio payload of {} bytes using model {}", payload.data.len(), self.model);
        
        // In reality, this would make an API call to OpenAI Whisper API 
        // using reqwest and the provided api_key.
        // For the stub implementation:
        
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        Ok(format!("[Transcribed using {}] ...audio content...", self.model))
    }
}
