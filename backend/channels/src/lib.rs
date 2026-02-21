use async_trait::async_trait;
use clawforge_core::Message;
use tokio::sync::mpsc;

// --------------- Original adapters ---------------
pub mod telegram;
pub mod telegram_commands;
pub mod telegram_groups;
pub mod telegram_inline;
pub mod telegram_media;
pub mod discord;
pub mod discord_embeds;
pub mod discord_slash;
pub mod discord_threads;
pub mod discord_voice;
pub mod whatsapp;
pub mod wa_media;
pub mod wa_groups;

// --------------- Phase 14 web-hook adapters ---------------
pub mod bluebubbles;
pub mod slack;
pub mod slack_events;
pub mod slack_blocks;
pub mod slack_modals;
pub mod matrix;

// --------------- Phase 25 long-tail adapters ---------------
pub mod googlechat;
pub mod irc;
pub mod line;
pub mod line_receive;
pub mod line_send;
pub mod line_rich_menu;
pub mod imessage;
pub mod applescript;
pub mod bluebubbles_client;
pub mod mattermost;
pub mod msteams;
pub mod signal;

// --------------- Phase 75 rate limiting ---------------
pub mod rate_limiter;
pub use rate_limiter::{ChannelRateLimiter, RateLimitPolicy, RateLimitResult};

/// All channel adapters implement this trait.
#[async_trait]
pub trait ChannelAdapter: Send + Sync {
    /// Human-readable adapter name for logging.
    fn name(&self) -> &str;

    /// Build an optional Axum sub-router for inbound webhook endpoints.
    /// Adapters that use polling/long-connections return an empty router.
    fn build_router(&self) -> axum::Router {
        axum::Router::new()
    }

    /// Start the adapter's background work (polling loop, WS connection, etc.).
    async fn start(&self, supervisor_tx: mpsc::Sender<Message>) -> anyhow::Result<()>;
}
