/// Command dispatch — route detected commands to handler functions.
///
/// Mirrors `src/auto-reply/dispatch.ts` from OpenClaw.
use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use crate::types::CommandInvocation;

// ---------------------------------------------------------------------------
// Handler trait
// ---------------------------------------------------------------------------

/// Context passed to every command handler.
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub session_id: String,
    pub channel: String,
    pub sender_id: String,
}

/// The result returned by a command handler — text reply to send back.
#[derive(Debug, Clone)]
pub struct CommandResponse {
    pub text: String,
    pub ephemeral: bool, // only visible to the invoker
}

impl CommandResponse {
    pub fn ok(text: impl Into<String>) -> Self {
        Self { text: text.into(), ephemeral: false }
    }
    pub fn ephemeral(text: impl Into<String>) -> Self {
        Self { text: text.into(), ephemeral: true }
    }
}

#[async_trait]
pub trait CommandHandler: Send + Sync {
    async fn handle(&self, ctx: &CommandContext, inv: &CommandInvocation) -> Result<CommandResponse>;
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::Arc;

pub struct CommandDispatcher {
    handlers: HashMap<String, Arc<dyn CommandHandler>>,
}

impl CommandDispatcher {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    pub fn register(&mut self, key: impl Into<String>, handler: Arc<dyn CommandHandler>) {
        self.handlers.insert(key.into(), handler);
    }

    pub async fn dispatch(
        &self,
        ctx: &CommandContext,
        inv: &CommandInvocation,
    ) -> Result<CommandResponse> {
        if let Some(handler) = self.handlers.get(&inv.key) {
            info!("[Commands] Dispatching /{} in session {}", inv.key, ctx.session_id);
            handler.handle(ctx, inv).await
        } else {
            Ok(CommandResponse::ephemeral(format!(
                "❓ No handler registered for command /{}", inv.key
            )))
        }
    }
}

impl Default for CommandDispatcher {
    fn default() -> Self { Self::new() }
}
