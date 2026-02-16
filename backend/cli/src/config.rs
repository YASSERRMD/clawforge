use serde::Deserialize;

/// ClawForge runtime configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// HTTP server bind address
    pub bind_address: String,
    /// HTTP server port
    pub port: u16,
    /// SQLite database path
    pub db_path: String,
    /// OpenRouter API key
    pub openrouter_api_key: Option<String>,
    /// Ollama base URL
    pub ollama_url: Option<String>,
    /// Log level
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            db_path: "clawforge.db".to_string(),
            openrouter_api_key: None,
            ollama_url: Some("http://localhost:11434".to_string()),
            log_level: "info".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables with sensible defaults.
    pub fn from_env() -> Self {
        Self {
            bind_address: std::env::var("CLAWFORGE_BIND")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("CLAWFORGE_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            db_path: std::env::var("CLAWFORGE_DB")
                .unwrap_or_else(|_| "clawforge.db".to_string()),
            openrouter_api_key: std::env::var("OPENROUTER_API_KEY").ok(),
            ollama_url: std::env::var("OLLAMA_URL").ok().or(Some("http://localhost:11434".to_string())),
            log_level: std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info".to_string()),
        }
    }
}
