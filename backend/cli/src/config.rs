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
    
    // BlueBubbles
    pub bluebubbles_server_url: Option<String>,
    pub bluebubbles_password: Option<String>,
    pub bluebubbles_webhook_path: String,
    
    // Slack
    pub slack_signing_secret: Option<String>,
    pub slack_bot_token: Option<String>,
    pub slack_webhook_path: String,
    
    // Matrix
    pub matrix_homeserver_url: Option<String>,
    pub matrix_access_token: Option<String>,
    pub matrix_user_id: Option<String>,
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
            bluebubbles_server_url: None,
            bluebubbles_password: None,
            bluebubbles_webhook_path: "/webhooks/bluebubbles".to_string(),
            slack_signing_secret: None,
            slack_bot_token: None,
            slack_webhook_path: "/webhooks/slack".to_string(),
            matrix_homeserver_url: None,
            matrix_access_token: None,
            matrix_user_id: None,
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
            bluebubbles_server_url: std::env::var("BLUEBUBBLES_SERVER_URL").ok(),
            bluebubbles_password: std::env::var("BLUEBUBBLES_PASSWORD").ok(),
            bluebubbles_webhook_path: std::env::var("BLUEBUBBLES_WEBHOOK_PATH")
                .unwrap_or_else(|_| "/webhooks/bluebubbles".to_string()),
            slack_signing_secret: std::env::var("SLACK_SIGNING_SECRET").ok(),
            slack_bot_token: std::env::var("SLACK_BOT_TOKEN").ok(),
            slack_webhook_path: std::env::var("SLACK_WEBHOOK_PATH")
                .unwrap_or_else(|_| "/webhooks/slack".to_string()),
            matrix_homeserver_url: std::env::var("MATRIX_HOMESERVER_URL").ok(),
            matrix_access_token: std::env::var("MATRIX_ACCESS_TOKEN").ok(),
            matrix_user_id: std::env::var("MATRIX_USER_ID").ok(),
        }
    }
}
