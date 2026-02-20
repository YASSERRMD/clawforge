use crate::ChannelAdapter;
use async_trait::async_trait;
use teloxide::prelude::*;
use tokio::sync::mpsc;
use tracing::{error, info};
use clawforge_core::{Message, EventKind, Event};
use uuid::Uuid;

pub struct TelegramAdapter {
    bot: Bot,
}

impl TelegramAdapter {
    pub fn new(token: String) -> Self {
        Self {
            bot: Bot::new(token),
        }
    }
}

#[async_trait]
impl ChannelAdapter for TelegramAdapter {
    fn name(&self) -> &str { "telegram" }

    async fn start(&self, supervisor_tx: mpsc::Sender<Message>) -> anyhow::Result<()> {
        info!("Starting Telegram adapter");
        
        // This is a simplified listener. A full version would handle commands, media, etc.
        let bot = self.bot.clone();
        let tx = supervisor_tx.clone();
        
        let handler = Update::filter_message().endpoint(
            |bot: Bot, msg: teloxide::types::Message, tx: mpsc::Sender<Message>| async move {
                if let Some(text) = msg.text() {
                    let chat_id = msg.chat.id.to_string();
                    info!("Received message from Telegram chat {}: {}", chat_id, text);
                    
                    // We need to map this to a Run. For simplicity in this iteration,
                    // we'll emit an audit event that the Supervisor could use to auto-create a Run.
                    // In a production app, we'd lookup active sessions/runs for this channel+chat_id.
                    
                    let event = Event::new(
                        Uuid::new_v4(), // Dummy Run ID for now
                        Uuid::new_v4(), // Dummy Agent ID
                        EventKind::RunStarted,
                        serde_json::json!({
                            "source": "telegram",
                            "chat_id": chat_id,
                            "text": text
                        })
                    );
                    
                    let _ = tx.send(Message::AuditEvent(clawforge_core::AuditEventPayload { event })).await;
                    
                    // Echo back for testing
                    let _ = bot.send_message(msg.chat.id, format!("Received: {}", text)).await;
                }
                respond(())
            }
        );

        Dispatcher::builder(bot.clone(), handler)
            .dependencies(dptree::deps![tx])
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
            
        Ok(())
    }

}

impl TelegramAdapter {
    pub async fn send_message(&self, chat_id: &str, text: &str) -> anyhow::Result<()> {
        let chat_id: i64 = chat_id.parse()?;
        self.bot.send_message(ChatId(chat_id), text).await?;
        Ok(())
    }
}
