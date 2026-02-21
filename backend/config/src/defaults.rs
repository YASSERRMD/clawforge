//! Config defaults: applies sensible default values to parsed config.

use crate::schema::{
    AgentDefaults, AgentsConfig, ClawForgeConfig, CompactionConfig, LoggingConfig,
    MessagesConfig, SubagentDefaults,
};

/// Default max concurrent agents.
pub const DEFAULT_AGENT_MAX_CONCURRENT: u32 = 10;

/// Default max concurrent subagents.
pub const DEFAULT_SUBAGENT_MAX_CONCURRENT: u32 = 5;

/// Default max subagent nesting depth.
pub const DEFAULT_SUBAGENT_MAX_DEPTH: u32 = 3;

/// Default context window size (tokens).
pub const DEFAULT_CONTEXT_TOKENS: u64 = 200_000;

/// Default max tokens for model responses.
pub const DEFAULT_MODEL_MAX_TOKENS: u64 = 8192;

/// Apply all defaults to a freshly loaded config.
pub fn apply_all_defaults(config: ClawForgeConfig) -> ClawForgeConfig {
    let config = apply_message_defaults(config);
    let config = apply_agent_defaults(config);
    let config = apply_compaction_defaults(config);
    let config = apply_logging_defaults(config);
    config
}

/// Ensure messages.ackReactionScope is set.
fn apply_message_defaults(mut config: ClawForgeConfig) -> ClawForgeConfig {
    let messages = config.messages.get_or_insert_with(MessagesConfig::default);
    if messages.ack_reaction_scope.is_none() {
        messages.ack_reaction_scope = Some("group-mentions".to_string());
    }
    config
}

/// Ensure agent concurrency and subagent depth limits are set.
fn apply_agent_defaults(mut config: ClawForgeConfig) -> ClawForgeConfig {
    let agents = config.agents.get_or_insert_with(AgentsConfig::default);
    let defaults = agents.defaults.get_or_insert_with(AgentDefaults::default);

    if defaults.max_concurrent.is_none() {
        defaults.max_concurrent = Some(DEFAULT_AGENT_MAX_CONCURRENT);
    }

    let subagents = defaults.subagents.get_or_insert_with(SubagentDefaults::default);
    if subagents.max_concurrent.is_none() {
        subagents.max_concurrent = Some(DEFAULT_SUBAGENT_MAX_CONCURRENT);
    }
    if subagents.max_depth.is_none() {
        subagents.max_depth = Some(DEFAULT_SUBAGENT_MAX_DEPTH);
    }

    config
}

/// Default compaction mode if not set.
fn apply_compaction_defaults(mut config: ClawForgeConfig) -> ClawForgeConfig {
    if let Some(agents) = &mut config.agents {
        if let Some(defaults) = &mut agents.defaults {
            let compaction = defaults.compaction.get_or_insert_with(CompactionConfig::default);
            if compaction.mode.is_none() {
                compaction.mode = Some("safeguard".to_string());
            }
        }
    }
    config
}

/// Default logging redact mode.
fn apply_logging_defaults(mut config: ClawForgeConfig) -> ClawForgeConfig {
    let logging = config.logging.get_or_insert_with(LoggingConfig::default);
    if logging.redact_sensitive.is_none() {
        logging.redact_sensitive = Some("tools".to_string());
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_ack_reaction_scope() {
        let cfg = ClawForgeConfig::default();
        let cfg = apply_all_defaults(cfg);
        assert_eq!(
            cfg.messages.unwrap().ack_reaction_scope.unwrap(),
            "group-mentions"
        );
    }

    #[test]
    fn applies_agent_max_concurrent() {
        let cfg = ClawForgeConfig::default();
        let cfg = apply_all_defaults(cfg);
        assert_eq!(
            cfg.agents.unwrap().defaults.unwrap().max_concurrent.unwrap(),
            DEFAULT_AGENT_MAX_CONCURRENT
        );
    }

    #[test]
    fn does_not_override_user_set_concurrent() {
        let mut cfg = ClawForgeConfig::default();
        cfg.agents = Some(AgentsConfig {
            defaults: Some(AgentDefaults {
                max_concurrent: Some(3),
                ..Default::default()
            }),
            ..Default::default()
        });
        let cfg = apply_all_defaults(cfg);
        assert_eq!(
            cfg.agents.unwrap().defaults.unwrap().max_concurrent.unwrap(),
            3
        );
    }
}
