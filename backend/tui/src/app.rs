//! TUI App State
//!
//! Manages the top-level application state for the Ratatui terminal UI.

use ratatui::widgets::ListState;

pub struct AppState {
    pub messages: Vec<String>,
    pub input: String,
    pub session_list_state: ListState,
    pub agents: Vec<String>,
    pub selected_agent: usize,
    pub is_streaming: bool,
    pub should_quit: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            messages: vec!["Welcome to ClawForge TUI!".into()],
            input: String::new(),
            session_list_state: ListState::default(),
            agents: vec!["default_agent".into(), "researcher".into()],
            selected_agent: 0,
            is_streaming: false,
            should_quit: false,
        }
    }

    pub fn push_message(&mut self, msg: String) {
        self.messages.push(msg);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
