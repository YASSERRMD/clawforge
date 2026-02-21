//! Main HTTP Gateway Server.
//!
//! Mirrors `src/gateway/server.ts` and general routing.

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{info, instrument};

use crate::control_ui;
use crate::openai_compat;
use crate::ws_server;
use crate::session_registry::SessionRegistry;
use crate::rate_limit::RateLimiter;
use crate::auth_health;
use crate::health_api;
use crate::health_monitor::HealthMonitor;

/// Application state shared across routes.
#[derive(Clone)]
pub struct GatewayState {
    pub session_registry: SessionRegistry,
    pub rate_limiter: RateLimiter,
    pub health_monitor: HealthMonitor,
}

/// Starts the main Axum HTTP server for the gateway.
#[instrument(skip(state))]
pub async fn start_server(addr: SocketAddr, state: GatewayState) -> Result<()> {
    // Build our application with routes
    let app = Router::new()
        // API Endpoints
        .route("/v1/chat/completions", post(openai_compat::chat_completions))
        .route("/api/health", get(health_api::get_health))
        .route("/api/v1/auth/health", get(auth_health::check_auth_health))
        // WebSocket Endpoint
        .route("/ws", get(ws_server::ws_handler))
        // Control UI Static Files
        .nest("/ui", control_ui::ui_router())
        .with_state(state);

    info!("Gateway HTTP server listening on {}", addr);
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
