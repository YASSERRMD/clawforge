use async_trait::async_trait;
use bytes::Bytes;
use clawforge_core::{Message, EventKind, Event};
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

pub mod audio;
pub mod image;
pub mod media_server;
pub mod mime_detect;

pub use media_server::media_router;
pub use mime_detect::{detect_mime_type, is_audio, is_image, is_inline_safe, is_video};

#[derive(Debug, Clone)]
pub struct MediaPayload {
    pub source: String,
    pub mime_type: String,
    pub data: Bytes,
}

#[async_trait]
pub trait MediaHandler: Send + Sync {
    /// Process incoming media (e.g., transcribe audio, OCR/describe images)
    /// and return textual representation or metadata to be injected into the session.
    async fn process(&self, payload: &MediaPayload) -> anyhow::Result<String>;
}

pub struct MediaPipeline {
    audio_handler: Box<dyn MediaHandler>,
    image_handler: Box<dyn MediaHandler>,
    supervisor_tx: mpsc::Sender<Message>,
}

impl MediaPipeline {
    pub fn new(
        audio_handler: Box<dyn MediaHandler>,
        image_handler: Box<dyn MediaHandler>,
        supervisor_tx: mpsc::Sender<Message>,
    ) -> Self {
        Self {
            audio_handler,
            image_handler,
            supervisor_tx,
        }
    }

    pub async fn handle_media(&self, run_id: Uuid, agent_id: Uuid, payload: MediaPayload) -> anyhow::Result<()> {
        info!("Received media payload: {} from {}", payload.mime_type, payload.source);

        let result = if payload.mime_type.starts_with("audio/") {
            self.audio_handler.process(&payload).await
        } else if payload.mime_type.starts_with("image/") {
            self.image_handler.process(&payload).await
        } else {
            warn!("Unsupported media type: {}", payload.mime_type);
            anyhow::bail!("Unsupported media type");
        };

        match result {
            Ok(text) => {
                let event = Event::new(
                    run_id,
                    agent_id,
                    EventKind::RunStarted, // Map to appropriate run event
                    serde_json::json!({
                        "source": payload.source,
                        "type": "media_processed",
                        "mime": payload.mime_type,
                        "extracted_text": text
                    })
                );
                let _ = self.supervisor_tx.send(Message::AuditEvent(clawforge_core::AuditEventPayload { event })).await;
            }
            Err(e) => warn!("Failed to process media: {}", e),
        }

        Ok(())
    }
}
