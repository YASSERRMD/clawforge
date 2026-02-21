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
pub mod whatsapp;

// --------------- Phase 14 web-hook adapters ---------------
pub mod bluebubbles;
pub mod slack;
pub mod matrix;

// --------------- Phase 25 long-tail adapters ---------------
pub mod googlechat;
pub mod irc;
pub mod line;
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
