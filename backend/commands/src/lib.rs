pub mod detection;
pub mod dispatch;
pub mod handlers;
pub mod registry;
pub mod types;

pub use detection::detect_command;
pub use dispatch::{CommandContext, CommandDispatcher, CommandHandler, CommandResponse};
pub use handlers::{
    CompactHandler, HelpHandler, ModelHandler, ResetHandler, SkillHandler,
    SkillHandler as KillHandler, StatusHandler, StopHandler, SubagentHandler,
    ThinkHandler, ToggleHandler, TtsHandler, WhoAmIHandler,
};
pub use registry::{builtin_commands, CommandRegistry};
pub use types::{CommandArg, CommandCategory, CommandDef, CommandInvocation, CommandScope};

/// Build a dispatcher pre-wired with all built-in handlers.
pub fn build_default_dispatcher() -> CommandDispatcher {
    let registry = CommandRegistry::new();
    let mut dispatcher = CommandDispatcher::new();

    use std::sync::Arc;
    dispatcher.register("help", Arc::new(HelpHandler { registry: CommandRegistry::new() }));
    dispatcher.register("commands", Arc::new(HelpHandler { registry: CommandRegistry::new() }));
    dispatcher.register("status", Arc::new(StatusHandler));
    dispatcher.register("whoami", Arc::new(WhoAmIHandler));
    dispatcher.register("think", Arc::new(ThinkHandler));
    dispatcher.register("stop", Arc::new(StopHandler));
    dispatcher.register("reset", Arc::new(ResetHandler));
    dispatcher.register("compact", Arc::new(CompactHandler));
    dispatcher.register("model", Arc::new(ModelHandler));
    dispatcher.register("verbose", Arc::new(ToggleHandler { label: "Verbose".into() }));
    dispatcher.register("reasoning", Arc::new(ToggleHandler { label: "Reasoning".into() }));
    dispatcher.register("elevated", Arc::new(ToggleHandler { label: "Elevated".into() }));
    dispatcher.register("subagents", Arc::new(SubagentHandler));
    dispatcher.register("kill", Arc::new(SubagentHandler));
    dispatcher.register("steer", Arc::new(SubagentHandler));
    dispatcher.register("skill", Arc::new(SkillHandler));
    dispatcher.register("tts", Arc::new(TtsHandler));

    dispatcher
}
