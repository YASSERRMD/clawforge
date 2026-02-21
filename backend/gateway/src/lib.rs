//! ClawForge Gateway HTTP API Server
//!
//! Provides the REST API, OpenAI compatibility layer, and Control UI static hosting.

pub mod config_reload;
pub mod control_ui;
pub mod openai_compat;
pub mod server;

pub use config_reload::ConfigReloader;
pub use server::{start_server, GatewayState};
