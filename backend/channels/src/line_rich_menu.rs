//! LINE Rich Menus
//!
//! State logic controlling persistent keyboard menus linked to user/channel accounts.

use anyhow::Result;
use tracing::info;

pub struct LineRichMenu;

impl LineRichMenu {
    /// Uploads a JSON layout and image to create a new persistent Rich Menu.
    pub async fn create_rich_menu(name: &str, chat_bar_text: &str) -> Result<String> {
        info!("Creating rich menu '{}' with touch text '{}'", name, chat_bar_text);
        
        // MOCK: POST https://api.line.me/v2/bot/richmenu
        Ok("mock_rich_menu_id_12345".into())
    }

    /// Binds a registered Rich Menu ID to a specific User ID contextually.
    pub async fn link_menu_to_user(user_id: &str, rich_menu_id: &str) -> Result<()> {
        info!("Linking User '{}' to Rich Menu '{}'", user_id, rich_menu_id);
        Ok(())
    }
}
