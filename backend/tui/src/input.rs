//! Keyboard Input Handler
//!
//! Processes crossterm events (Key, Char, Enter) and updates `AppState`.

use crossterm::event::{Event, KeyCode, KeyEvent};
use crate::app::AppState;

/// Handles a single synchronous keyboard event.
pub fn handle_key_event(key: KeyEvent, state: &mut AppState) {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
            state.should_quit = true;
        }
        KeyCode::Esc => {
            state.should_quit = true;
        }
        KeyCode::Enter => {
            if !state.input.is_empty() {
                let msg = state.input.clone();
                state.input.clear();
                
                // MOCK: Check if slash command
                if msg.starts_with('/') {
                    state.push_message(format!("Command Executed: {}", msg));
                } else {
                    state.push_message(format!("You: {}", msg));
                    // MOCK: notify streaming thread
                }
            }
        }
        KeyCode::Backspace => {
            state.input.pop();
        }
        KeyCode::Char(c) => {
            state.input.push(c);
        }
        _ => {}
    }
}
