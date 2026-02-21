//! ClawForge Gateway HTTP API Server
//!
//! Provides the REST API, OpenAI compatibility layer, and Control UI static hosting.

pub mod auth;
pub mod auth_health;
pub mod config_reload;
pub mod control_ui;
pub mod openai_compat;
pub mod rate_limit;
pub mod server;
pub mod session_registry;
pub mod ws_protocol;
pub mod ws_server;

pub use config_reload::ConfigReloader;
pub use server::{start_server, GatewayState};
