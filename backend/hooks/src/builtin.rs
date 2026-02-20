/// Built-in hook implementations.
///
/// These are bundled hooks that ship with ClawForge and can be enabled
/// via configuration. Each hook is a concrete struct that implements the
/// `Hook` trait.
use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use crate::registry::Hook;
use crate::types::{HookPayload, HookResult};

// ---------------------------------------------------------------------------
// Logging hook — logs every lifecycle event
// ---------------------------------------------------------------------------

pub struct LoggingHook {
    pub prefix: String,
}

impl LoggingHook {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self { prefix: prefix.into() }
    }
}

#[async_trait]
impl Hook for LoggingHook {
    fn name(&self) -> &str { "logging_hook" }

    async fn run(&self, payload: &HookPayload) -> Result<HookResult> {
        info!("[{}] Hook fired: {:?}", self.prefix, payload.phase());
        Ok(HookResult::pass())
    }
}

// ---------------------------------------------------------------------------
// Content filter hook — blocks messages matching a word list
// ---------------------------------------------------------------------------

pub struct ContentFilterHook {
    pub blocked_words: Vec<String>,
}

impl ContentFilterHook {
    pub fn new(blocked_words: Vec<String>) -> Self {
        Self { blocked_words }
    }
}

#[async_trait]
impl Hook for ContentFilterHook {
    fn name(&self) -> &str { "content_filter_hook" }

    async fn run(&self, payload: &HookPayload) -> Result<HookResult> {
        let content = match payload {
            HookPayload::PreMessage(p) => &p.content,
            HookPayload::PostMessage(p) => &p.content,
            _ => return Ok(HookResult::pass()),
        };

        let lower = content.to_lowercase();
        for word in &self.blocked_words {
            if lower.contains(word.as_str()) {
                return Ok(HookResult::abort(format!("Blocked word detected: {}", word)));
            }
        }
        Ok(HookResult::pass())
    }
}

// ---------------------------------------------------------------------------
// Model override hook — switches model based on channel
// ---------------------------------------------------------------------------

pub struct ChannelModelOverrideHook {
    /// Map of channel prefix → model ID to use
    pub channel_model_map: std::collections::HashMap<String, String>,
}

#[async_trait]
impl Hook for ChannelModelOverrideHook {
    fn name(&self) -> &str { "channel_model_override_hook" }

    async fn run(&self, payload: &HookPayload) -> Result<HookResult> {
        let (session_id, channel) = match payload {
            HookPayload::ModelOverride(p) => (&p.session_id, None::<&str>),
            HookPayload::PreMessage(p) => (&p.session_id, Some(p.channel.as_str())),
            _ => return Ok(HookResult::pass()),
        };

        if let Some(ch) = channel {
            for (prefix, model) in &self.channel_model_map {
                if ch.starts_with(prefix.as_str()) {
                    info!("[ModelOverride] Channel {} → model {}", ch, model);
                    return Ok(HookResult {
                        model_override: Some(model.clone()),
                        ..HookResult::pass()
                    });
                }
            }
        }
        Ok(HookResult::pass())
    }
}

// ---------------------------------------------------------------------------
// Tool policy hook — blocks specific tools
// ---------------------------------------------------------------------------

pub struct ToolPolicyHook {
    /// Tools that are always blocked
    pub blocked_tools: Vec<String>,
}

#[async_trait]
impl Hook for ToolPolicyHook {
    fn name(&self) -> &str { "tool_policy_hook" }

    async fn run(&self, payload: &HookPayload) -> Result<HookResult> {
        let tool_name = match payload {
            HookPayload::PreToolCall(p) => &p.tool_name,
            _ => return Ok(HookResult::pass()),
        };

        if self.blocked_tools.iter().any(|b| b == tool_name) {
            return Ok(HookResult::abort(format!("Tool '{}' is blocked by policy", tool_name)));
        }
        Ok(HookResult::pass())
    }
}
