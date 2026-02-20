use crate::ChannelAdapter;
use async_trait::async_trait;
use serenity::prelude::*;
use serenity::model::channel::Message as DiscordMessage;
use serenity::model::gateway::Ready;
use tokio::sync::mpsc;
use tracing::{error, info};
use clawforge_core::{Message, EventKind, Event};
use uuid::Uuid;

struct Handler {
    supervisor_tx: mpsc::Sender<Message>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: DiscordMessage) {
        if msg.author.bot {
            return;
        }

        let channel_id = msg.channel_id.to_string();
        info!("Received message from Discord channel {}: {}", channel_id, msg.content);

        let event = Event::new(
            Uuid::new_v4(), // Dummy Run ID for now
            Uuid::new_v4(), // Dummy Agent ID
            EventKind::RunStarted,
            serde_json::json!({
                "source": "discord",
                "channel_id": channel_id,
                "author": msg.author.name,
                "text": msg.content
            })
        );

        let _ = self.supervisor_tx.send(Message::AuditEvent(clawforge_core::AuditEventPayload { event })).await;

        if let Err(e) = msg.channel_id.say(&ctx.http, format!("Received: {}", msg.content)).await {
            error!("Error sending message: {:?}", e);
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

pub struct DiscordAdapter {
    token: String,
}

impl DiscordAdapter {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

#[async_trait]
impl ChannelAdapter for DiscordAdapter {
    fn name(&self) -> &str { "discord" }

    async fn start(&self, supervisor_tx: mpsc::Sender<Message>) -> anyhow::Result<()> {
        info!("Starting Discord adapter");
        
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        let mut client = Client::builder(&self.token, intents)
            .event_handler(Handler { supervisor_tx })
            .await?;

        if let Err(why) = client.start().await {
            error!("Client error: {:?}", why);
            anyhow::bail!("Discord client error: {:?}", why);
        }

        Ok(())
    }
}

impl DiscordAdapter {
    pub async fn send_message(&self, _chat_id: &str, _text: &str) -> anyhow::Result<()> {
        info!("Discord send_message not fully implemented in adapter yet.");
        Ok(())
    }
}
