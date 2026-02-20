/// Built-in command handlers.
///
/// Each handler is a concrete struct implementing `CommandHandler`.
/// These are stub implementations — real behavior will call into
/// the appropriate ClawForge subsystems (executor, session manager, etc.).
use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use crate::dispatch::{CommandContext, CommandHandler, CommandResponse};
use crate::registry::CommandRegistry;
use crate::types::CommandInvocation;

// ---------------------------------------------------------------------------
// /help
// ---------------------------------------------------------------------------

pub struct HelpHandler {
    pub registry: CommandRegistry,
}

#[async_trait]
impl CommandHandler for HelpHandler {
    async fn handle(&self, _ctx: &CommandContext, _inv: &CommandInvocation) -> Result<CommandResponse> {
        let mut lines = vec!["*Available commands:*".to_string()];
        for cmd in self.registry.all() {
            let alias = cmd.primary_alias();
            lines.push(format!("• `{}` — {}", alias, cmd.description));
        }
        Ok(CommandResponse::ephemeral(lines.join("\n")))
    }
}

// ---------------------------------------------------------------------------
// /status
// ---------------------------------------------------------------------------

pub struct StatusHandler;

#[async_trait]
impl CommandHandler for StatusHandler {
    async fn handle(&self, ctx: &CommandContext, _inv: &CommandInvocation) -> Result<CommandResponse> {
        Ok(CommandResponse::ephemeral(format!(
            "✅ Session `{}` on channel `{}` — agent is running",
            ctx.session_id, ctx.channel
        )))
    }
}

// ---------------------------------------------------------------------------
// /whoami
// ---------------------------------------------------------------------------

pub struct WhoAmIHandler;

#[async_trait]
impl CommandHandler for WhoAmIHandler {
    async fn handle(&self, ctx: &CommandContext, _inv: &CommandInvocation) -> Result<CommandResponse> {
        Ok(CommandResponse::ephemeral(format!("👤 Your sender id: `{}`", ctx.sender_id)))
    }
}

// ---------------------------------------------------------------------------
// /think
// ---------------------------------------------------------------------------

pub struct ThinkHandler;

#[async_trait]
impl CommandHandler for ThinkHandler {
    async fn handle(&self, _ctx: &CommandContext, inv: &CommandInvocation) -> Result<CommandResponse> {
        let level = inv.args.first().map(|s| s.as_str()).unwrap_or("medium");
        let levels = ["off", "minimal", "low", "medium", "high", "xhigh"];
        if !levels.contains(&level) {
            return Ok(CommandResponse::ephemeral(format!(
                "❌ Unknown thinking level `{}`. Valid: {}", level, levels.join(", ")
            )));
        }
        info!("[Commands] Setting thinking level: {}", level);
        Ok(CommandResponse::ephemeral(format!("🧠 Thinking level set to `{}`", level)))
    }
}

// ---------------------------------------------------------------------------
// /stop
// ---------------------------------------------------------------------------

pub struct StopHandler;

#[async_trait]
impl CommandHandler for StopHandler {
    async fn handle(&self, ctx: &CommandContext, _inv: &CommandInvocation) -> Result<CommandResponse> {
        info!("[Commands] Stop requested for session {}", ctx.session_id);
        Ok(CommandResponse::ok("🛑 Stopping current run..."))
    }
}

// ---------------------------------------------------------------------------
// /reset
// ---------------------------------------------------------------------------

pub struct ResetHandler;

#[async_trait]
impl CommandHandler for ResetHandler {
    async fn handle(&self, ctx: &CommandContext, inv: &CommandInvocation) -> Result<CommandResponse> {
        info!("[Commands] Reset session {}", ctx.session_id);
        let suffix = if inv.raw_args.is_empty() {
            String::new()
        } else {
            format!(" with instructions: _{}_", inv.raw_args)
        };
        Ok(CommandResponse::ok(format!("🔄 Session reset{}", suffix)))
    }
}

// ---------------------------------------------------------------------------
// /compact
// ---------------------------------------------------------------------------

pub struct CompactHandler;

#[async_trait]
impl CommandHandler for CompactHandler {
    async fn handle(&self, ctx: &CommandContext, inv: &CommandInvocation) -> Result<CommandResponse> {
        info!("[Commands] Compact session {}", ctx.session_id);
        Ok(CommandResponse::ok("📦 Compacting context..."))
    }
}

// ---------------------------------------------------------------------------
// /model
// ---------------------------------------------------------------------------

pub struct ModelHandler;

#[async_trait]
impl CommandHandler for ModelHandler {
    async fn handle(&self, _ctx: &CommandContext, inv: &CommandInvocation) -> Result<CommandResponse> {
        if let Some(model) = inv.args.first() {
            Ok(CommandResponse::ok(format!("🤖 Model set to `{}`", model)))
        } else {
            Ok(CommandResponse::ephemeral("🤖 Current model: _(use /model <id> to change)_"))
        }
    }
}

// ---------------------------------------------------------------------------
// /verbose, /reasoning
// ---------------------------------------------------------------------------

pub struct ToggleHandler { pub label: String }

#[async_trait]
impl CommandHandler for ToggleHandler {
    async fn handle(&self, _ctx: &CommandContext, inv: &CommandInvocation) -> Result<CommandResponse> {
        let mode = inv.args.first().map(|s| s.as_str()).unwrap_or("on");
        Ok(CommandResponse::ephemeral(format!("🔧 {} set to `{}`", self.label, mode)))
    }
}

// ---------------------------------------------------------------------------
// /kill, /steer, /subagents
// ---------------------------------------------------------------------------

pub struct SubagentHandler;

#[async_trait]
impl CommandHandler for SubagentHandler {
    async fn handle(&self, ctx: &CommandContext, inv: &CommandInvocation) -> Result<CommandResponse> {
        let action = inv.args.first().map(|s| s.as_str()).unwrap_or("list");
        info!("[Commands] Subagent '{}' in session {}", action, ctx.session_id);
        Ok(CommandResponse::ephemeral(format!("🤖 Subagent action `{}` queued", action)))
    }
}

// ---------------------------------------------------------------------------
// /skill
// ---------------------------------------------------------------------------

pub struct SkillHandler;

#[async_trait]
impl CommandHandler for SkillHandler {
    async fn handle(&self, ctx: &CommandContext, inv: &CommandInvocation) -> Result<CommandResponse> {
        let name = inv.args.first().map(|s| s.as_str()).unwrap_or("");
        if name.is_empty() {
            return Ok(CommandResponse::ephemeral("❌ Usage: /skill <name> [input]"));
        }
        info!("[Commands] Running skill '{}' in session {}", name, ctx.session_id);
        Ok(CommandResponse::ok(format!("⚡ Running skill `{}`...", name)))
    }
}

// ---------------------------------------------------------------------------
// /tts
// ---------------------------------------------------------------------------

pub struct TtsHandler;

#[async_trait]
impl CommandHandler for TtsHandler {
    async fn handle(&self, _ctx: &CommandContext, inv: &CommandInvocation) -> Result<CommandResponse> {
        let action = inv.args.first().map(|s| s.as_str()).unwrap_or("status");
        match action {
            "on" => Ok(CommandResponse::ok("🔊 TTS enabled")),
            "off" => Ok(CommandResponse::ok("🔇 TTS disabled")),
            "status" => Ok(CommandResponse::ephemeral("🔊 TTS status: _(not yet configured)_")),
            _ => Ok(CommandResponse::ephemeral(format!("TTS action `{}` applied", action))),
        }
    }
}
