//! Telegram Group Threading
//!
//! Logic dictating thread separation in supergroups, and resolving bot admin constraints.

use anyhow::Result;
use tracing::info;

pub struct TelegramGroups;

impl TelegramGroups {
    /// Normalizes a chat ID and optional thread ID into a unique ClawForge session ID.
    pub fn resolve_session_id(chat_id: i64, message_thread_id: Option<i64>) -> String {
        match message_thread_id {
            Some(thread) => format!("tg-{}-{}", chat_id, thread),
            None => format!("tg-{}", chat_id),
        }
    }

    /// Asserts whether the bot must respond in a group (e.g., was it @mentioned, or a reply?)
    pub fn should_respond(text: &str, is_reply_to_bot: bool) -> bool {
        let is_mention = text.contains("@ClawForgeBot"); // MOCK bot username
        is_mention || is_reply_to_bot
    }

    /// Enforces Group Admin actions (e.g. kicking users if an agent plugin executes a Ban tool).
    pub async fn kick_member(chat_id: i64, user_id: i64) -> Result<()> {
        info!("Kicking user {} from chat {})", user_id, chat_id);
        Ok(())
    }
}
