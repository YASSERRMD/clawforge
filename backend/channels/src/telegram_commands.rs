//! Telegram Bot Commands
//!
//! Handles deep command routing for `/start`, `/help`, and `/agent`.

use anyhow::Result;
use tracing::info;

pub struct TelegramCommands;

impl TelegramCommands {
    /// Dispatches a command payload to the appropriate handler.
    pub async fn dispatch(command: &str, chat_id: i64) -> Result<()> {
        info!("Handling Telegram command: {} for chat_id: {}", command, chat_id);
        
        match command {
            "/start" => Self::handle_start(chat_id).await?,
            "/help" => Self::handle_help(chat_id).await?,
            "/agent" => Self::handle_agent(chat_id).await?,
            _ => {
                info!("Unknown command: {}", command);
            }
        }
        
        Ok(())
    }

    async fn handle_start(chat_id: i64) -> Result<()> {
        info!("Sending start welcome to {}", chat_id);
        Ok(())
    }

    async fn handle_help(chat_id: i64) -> Result<()> {
        info!("Sending help menu to {}", chat_id);
        Ok(())
    }

    async fn handle_agent(chat_id: i64) -> Result<()> {
        info!("Handling /agent command for {}", chat_id);
        Ok(())
    }
}
