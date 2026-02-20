use crate::{MediaHandler, MediaPayload};
use async_trait::async_trait;
use tracing::info;

pub struct VisionImageHandler {
    api_key: String,
    model: String,
}

impl VisionImageHandler {
    pub fn new(api_key: String, model: impl Into<String>) -> Self {
        Self {
            api_key,
            model: model.into(),
        }
    }
}

#[async_trait]
impl MediaHandler for VisionImageHandler {
    async fn process(&self, payload: &MediaPayload) -> anyhow::Result<String> {
        info!("Describing image payload of {} bytes using model {}", payload.data.len(), self.model);
        
        // In reality, this would make an API call to GPT-4o-vision or similar
        // For the stub implementation:
        
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        Ok(format!("[Analyzed using {}] ...a descriptive summary of the image...", self.model))
    }
}
