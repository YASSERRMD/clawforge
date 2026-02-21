//! TUI Rendering
//!
//! Translates `AppState` into Ratatui `Widget`s and draws to the terminal frame.

use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;

/// Main draw loop function.
pub fn draw_ui<B: Backend>(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(3),    // Chat Messages
            Constraint::Length(3), // Input Box
        ])
        .split(f.size());

    // Messages Pane
    let messages_text = state.messages.join("\n");
    let messages_widget = Paragraph::new(messages_text)
        .block(Block::default().title("Chat History").borders(Borders::ALL));
    f.render_widget(messages_widget, chunks[0]);

    // Input Box Pane
    let input_widget = Paragraph::new(state.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().title("Message (Prefix '/' for commands)").borders(Borders::ALL));
    f.render_widget(input_widget, chunks[1]);
}
