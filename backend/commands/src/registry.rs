/// Slash command registry — 30+ built-in commands.
///
/// Mirrors `src/auto-reply/commands-registry.data.ts` from OpenClaw.
use crate::types::{ArgType, CommandArg, CommandCategory, CommandDef, CommandScope};

fn arg(
    name: &str, description: &str, ty: ArgType, required: bool, choices: &[&str], capture: bool,
) -> CommandArg {
    CommandArg {
        name: name.to_string(),
        description: description.to_string(),
        arg_type: ty,
        required,
        capture_remaining: capture,
        choices: choices.iter().map(|s| s.to_string()).collect(),
    }
}

fn string_arg(name: &str, description: &str) -> CommandArg {
    arg(name, description, ArgType::String, false, &[], false)
}

fn required_string_arg(name: &str, description: &str) -> CommandArg {
    arg(name, description, ArgType::String, true, &[], false)
}

fn choice_arg(name: &str, description: &str, choices: &[&str]) -> CommandArg {
    arg(name, description, ArgType::String, false, choices, false)
}

fn remaining_arg(name: &str, description: &str) -> CommandArg {
    arg(name, description, ArgType::String, false, &[], true)
}

/// Build the full built-in command registry.
pub fn builtin_commands() -> Vec<CommandDef> {
    vec![
        CommandDef {
            key: "help".into(),
            native_name: Some("help".into()),
            description: "Show available commands.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Status,
            text_aliases: vec!["/help".into()],
            args: vec![],
            accepts_args: false,
        },
        CommandDef {
            key: "commands".into(),
            native_name: Some("commands".into()),
            description: "List all slash commands.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Status,
            text_aliases: vec!["/commands".into()],
            args: vec![],
            accepts_args: false,
        },
        CommandDef {
            key: "status".into(),
            native_name: Some("status".into()),
            description: "Show current agent status.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Status,
            text_aliases: vec!["/status".into()],
            args: vec![],
            accepts_args: false,
        },
        CommandDef {
            key: "whoami".into(),
            native_name: Some("whoami".into()),
            description: "Show your sender id.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Status,
            text_aliases: vec!["/whoami".into(), "/id".into()],
            args: vec![],
            accepts_args: false,
        },
        CommandDef {
            key: "context".into(),
            native_name: Some("context".into()),
            description: "Explain how context is built and used.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Status,
            text_aliases: vec!["/context".into()],
            args: vec![],
            accepts_args: true,
        },
        // Session management
        CommandDef {
            key: "stop".into(),
            native_name: Some("stop".into()),
            description: "Stop the current run.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Session,
            text_aliases: vec!["/stop".into()],
            args: vec![],
            accepts_args: false,
        },
        CommandDef {
            key: "reset".into(),
            native_name: Some("reset".into()),
            description: "Reset the current session.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Session,
            text_aliases: vec!["/reset".into()],
            args: vec![remaining_arg("instructions", "Optional reset instructions")],
            accepts_args: true,
        },
        CommandDef {
            key: "new".into(),
            native_name: Some("new".into()),
            description: "Start a new session.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Session,
            text_aliases: vec!["/new".into()],
            args: vec![remaining_arg("prompt", "Opening prompt for new session")],
            accepts_args: true,
        },
        CommandDef {
            key: "compact".into(),
            native_name: Some("compact".into()),
            description: "Compact the session context.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Session,
            text_aliases: vec!["/compact".into()],
            args: vec![remaining_arg("instructions", "Extra compaction instructions")],
            accepts_args: true,
        },
        CommandDef {
            key: "export-session".into(),
            native_name: Some("export-session".into()),
            description: "Export current session to HTML.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Status,
            text_aliases: vec!["/export-session".into(), "/export".into()],
            args: vec![string_arg("path", "Output path (default: workspace)")],
            accepts_args: true,
        },
        // Options
        CommandDef {
            key: "think".into(),
            native_name: Some("think".into()),
            description: "Set thinking/reasoning level.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Options,
            text_aliases: vec!["/think".into(), "/thinking".into(), "/t".into()],
            args: vec![choice_arg("level", "off, minimal, low, medium, high, xhigh",
                &["off", "minimal", "low", "medium", "high", "xhigh"])],
            accepts_args: true,
        },
        CommandDef {
            key: "verbose".into(),
            native_name: Some("verbose".into()),
            description: "Toggle verbose tool output mode.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Options,
            text_aliases: vec!["/verbose".into(), "/v".into()],
            args: vec![choice_arg("mode", "on or off", &["on", "off"])],
            accepts_args: true,
        },
        CommandDef {
            key: "reasoning".into(),
            native_name: Some("reasoning".into()),
            description: "Toggle reasoning visibility (on/off/stream).".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Options,
            text_aliases: vec!["/reasoning".into(), "/reason".into()],
            args: vec![choice_arg("mode", "on, off, or stream", &["on", "off", "stream"])],
            accepts_args: true,
        },
        CommandDef {
            key: "elevated".into(),
            native_name: Some("elevated".into()),
            description: "Toggle elevated mode (on/off/ask/full).".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Options,
            text_aliases: vec!["/elevated".into(), "/elev".into()],
            args: vec![choice_arg("mode", "on, off, ask, or full", &["on", "off", "ask", "full"])],
            accepts_args: true,
        },
        CommandDef {
            key: "model".into(),
            native_name: Some("model".into()),
            description: "Show or set the active model.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Options,
            text_aliases: vec!["/model".into()],
            args: vec![string_arg("model", "Model id (provider/model or id)")],
            accepts_args: true,
        },
        CommandDef {
            key: "models".into(),
            native_name: Some("models".into()),
            description: "List model providers or provider models.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Options,
            text_aliases: vec!["/models".into()],
            args: vec![],
            accepts_args: true,
        },
        CommandDef {
            key: "usage".into(),
            native_name: Some("usage".into()),
            description: "Token or cost usage summary.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Options,
            text_aliases: vec!["/usage".into()],
            args: vec![choice_arg("mode", "off, tokens, full, or cost", &["off", "tokens", "full", "cost"])],
            accepts_args: true,
        },
        CommandDef {
            key: "queue".into(),
            native_name: Some("queue".into()),
            description: "Adjust inbound message queue settings.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Options,
            text_aliases: vec!["/queue".into()],
            args: vec![
                choice_arg("mode", "steer, interrupt, followup, collect, steer-backlog",
                    &["steer", "interrupt", "followup", "collect", "steer-backlog"]),
                string_arg("debounce", "Debounce duration (e.g. 500ms, 2s)"),
                arg("cap", "Queue cap", ArgType::Number, false, &[], false),
                choice_arg("drop", "old, new, or summarize", &["old", "new", "summarize"]),
            ],
            accepts_args: true,
        },
        CommandDef {
            key: "exec".into(),
            native_name: Some("exec".into()),
            description: "Set exec host/security policy for this session.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Options,
            text_aliases: vec!["/exec".into()],
            args: vec![
                choice_arg("host", "sandbox, gateway, or node", &["sandbox", "gateway", "node"]),
                choice_arg("security", "deny, allowlist, or full", &["deny", "allowlist", "full"]),
                choice_arg("ask", "off, on-miss, or always", &["off", "on-miss", "always"]),
            ],
            accepts_args: true,
        },
        // Management
        CommandDef {
            key: "allowlist".into(),
            native_name: None,
            description: "List/add/remove allowlist entries.".into(),
            scope: CommandScope::Text,
            category: CommandCategory::Management,
            text_aliases: vec!["/allowlist".into()],
            args: vec![
                choice_arg("action", "list, add, or remove", &["list", "add", "remove"]),
                string_arg("entry", "Sender or channel to add/remove"),
            ],
            accepts_args: true,
        },
        CommandDef {
            key: "approve".into(),
            native_name: Some("approve".into()),
            description: "Approve or deny a pending exec request.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Management,
            text_aliases: vec!["/approve".into()],
            args: vec![
                choice_arg("verdict", "yes or no", &["yes", "no"]),
                string_arg("id", "Approval request id"),
            ],
            accepts_args: true,
        },
        CommandDef {
            key: "config".into(),
            native_name: Some("config".into()),
            description: "Show or set config values.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Management,
            text_aliases: vec!["/config".into()],
            args: vec![
                choice_arg("action", "show, get, set, unset", &["show", "get", "set", "unset"]),
                string_arg("path", "Config path"),
                remaining_arg("value", "Value for set"),
            ],
            accepts_args: true,
        },
        CommandDef {
            key: "debug".into(),
            native_name: Some("debug".into()),
            description: "Set runtime debug overrides.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Management,
            text_aliases: vec!["/debug".into()],
            args: vec![
                choice_arg("action", "show, reset, set, unset", &["show", "reset", "set", "unset"]),
                string_arg("path", "Debug path"),
                remaining_arg("value", "Value for set"),
            ],
            accepts_args: true,
        },
        CommandDef {
            key: "send".into(),
            native_name: Some("send".into()),
            description: "Set send policy (on/off/inherit).".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Management,
            text_aliases: vec!["/send".into()],
            args: vec![choice_arg("mode", "on, off, or inherit", &["on", "off", "inherit"])],
            accepts_args: true,
        },
        CommandDef {
            key: "activation".into(),
            native_name: Some("activation".into()),
            description: "Set group activation mode (mention/always).".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Management,
            text_aliases: vec!["/activation".into()],
            args: vec![choice_arg("mode", "mention or always", &["mention", "always"])],
            accepts_args: true,
        },
        // Tools
        CommandDef {
            key: "skill".into(),
            native_name: Some("skill".into()),
            description: "Run a named skill.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Tools,
            text_aliases: vec!["/skill".into()],
            args: vec![
                required_string_arg("name", "Skill name"),
                remaining_arg("input", "Skill input"),
            ],
            accepts_args: true,
        },
        CommandDef {
            key: "bash".into(),
            native_name: None,
            description: "Run a host shell command (host-only).".into(),
            scope: CommandScope::Text,
            category: CommandCategory::Tools,
            text_aliases: vec!["/bash".into()],
            args: vec![remaining_arg("command", "Shell command to run")],
            accepts_args: true,
        },
        CommandDef {
            key: "restart".into(),
            native_name: Some("restart".into()),
            description: "Restart the ClawForge gateway.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Tools,
            text_aliases: vec!["/restart".into()],
            args: vec![],
            accepts_args: false,
        },
        // Sub-agent management
        CommandDef {
            key: "subagents".into(),
            native_name: Some("subagents".into()),
            description: "List, kill, log, spawn, or steer sub-agents.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Management,
            text_aliases: vec!["/subagents".into()],
            args: vec![
                choice_arg("action", "list | kill | log | info | send | steer | spawn",
                    &["list", "kill", "log", "info", "send", "steer", "spawn"]),
                string_arg("target", "Run id, index, or session key"),
                remaining_arg("value", "Additional input"),
            ],
            accepts_args: true,
        },
        CommandDef {
            key: "kill".into(),
            native_name: Some("kill".into()),
            description: "Kill a running sub-agent (or all).".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Management,
            text_aliases: vec!["/kill".into()],
            args: vec![string_arg("target", "Label, run id, index, or all")],
            accepts_args: true,
        },
        CommandDef {
            key: "steer".into(),
            native_name: Some("steer".into()),
            description: "Send guidance to a running sub-agent.".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Management,
            text_aliases: vec!["/steer".into(), "/tell".into()],
            args: vec![
                string_arg("target", "Label, run id, or index"),
                remaining_arg("message", "Steering message"),
            ],
            accepts_args: true,
        },
        // Media
        CommandDef {
            key: "tts".into(),
            native_name: Some("tts".into()),
            description: "Control text-to-speech (on/off/provider/limit/audio).".into(),
            scope: CommandScope::Both,
            category: CommandCategory::Media,
            text_aliases: vec!["/tts".into()],
            args: vec![
                choice_arg("action", "on, off, status, provider, limit, summary, audio, help",
                    &["on", "off", "status", "provider", "limit", "summary", "audio", "help"]),
                remaining_arg("value", "Provider name, limit, or text"),
            ],
            accepts_args: true,
        },
    ]
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

pub struct CommandRegistry {
    commands: Vec<CommandDef>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self { commands: builtin_commands() }
    }

    /// Register an additional command (e.g. plugin-provided).
    pub fn register(&mut self, def: CommandDef) {
        self.commands.push(def);
    }

    pub fn all(&self) -> &[CommandDef] {
        &self.commands
    }

    /// Find a command by slash-text alias (e.g. "/think").
    pub fn find_by_alias(&self, alias: &str) -> Option<&CommandDef> {
        let lower = alias.to_lowercase();
        self.commands.iter().find(|c| {
            c.text_aliases.iter().any(|a| a.to_lowercase() == lower)
        })
    }

    /// Find a command by its key.
    pub fn find_by_key(&self, key: &str) -> Option<&CommandDef> {
        self.commands.iter().find(|c| c.key == key)
    }
}

impl Default for CommandRegistry {
    fn default() -> Self { Self::new() }
}
