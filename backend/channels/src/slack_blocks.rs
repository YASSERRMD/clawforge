//! Slack Block Kit Builder
//!
//! Converts general markdown AI responses into rich, interactive Slack blocks.

use anyhow::Result;

pub struct SlackBlocks;

impl SlackBlocks {
    /// Translates markdown text into a Slack `section` block.
    pub fn build_text_section(text: &str) -> String {
        // MOCK: JSON payload for a Slack Block
        format!(
            r#"{{ "type": "section", "text": {{ "type": "mrkdwn", "text": "{}" }} }}"#,
            text.replace('"', "\\\"")
        )
    }

    /// Appends interactive action buttons underneath an agent's response.
    pub fn build_action_buttons(button_texts: &[&str]) -> String {
        // MOCK: Output Action Block
        let mut buttons = String::new();
        for (i, text) in button_texts.iter().enumerate() {
            if i > 0 { buttons.push_str(", "); }
            buttons.push_str(&format!(
                r#"{{ "type": "button", "text": {{ "type": "plain_text", "text": "{}" }}, "action_id": "btn_{}" }}"#,
                text, i
            ));
        }
        format!(r#"{{ "type": "actions", "elements": [{}] }}"#, buttons)
    }
}
