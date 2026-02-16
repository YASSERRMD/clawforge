use thiserror::Error;

/// Top-level error type for the ClawForge runtime.
#[derive(Debug, Error)]
pub enum ClawError {
    #[error("capability denied: {0}")]
    CapabilityDenied(String),

    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),

    #[error("LLM provider error ({provider}): {message}")]
    LlmError { provider: String, message: String },

    #[error("all LLM providers failed")]
    AllProvidersFailed,

    #[error("action execution failed: {0}")]
    ExecutionFailed(String),

    #[error("channel closed: {0}")]
    ChannelClosed(String),

    #[error("configuration error: {0}")]
    ConfigError(String),

    #[error("storage error: {0}")]
    StorageError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
