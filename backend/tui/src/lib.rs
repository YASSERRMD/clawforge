//! TUI (Terminal User Interface) logic for ClawForge CLI.
//!
//! Exposes ratatui elements and core state required to run "clawforge ui".

pub mod app;
pub mod input;
pub mod render;
pub mod streaming;

pub use app::AppState;
pub use input::handle_key_event;
pub use render::draw_ui;
pub use streaming::start_sse_consumer;
