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

/// Application state shared across routes.
#[derive(Clone)]
pub struct GatewayState {
    // In a full implementation, this holds references to the AgentRunner, registry, config, etc.
}

/// Starts the main Axum HTTP server for the gateway.
#[instrument(skip(state))]
pub async fn start_server(addr: SocketAddr, state: GatewayState) -> Result<()> {
    // Build our application with routes
    let app = Router::new()
        // API Endpoints
        .route("/v1/chat/completions", post(openai_compat::chat_completions))
        .route("/api/health", get(|| async { "OK" }))
        // Control UI Static Files
        .nest("/ui", control_ui::ui_router())
        .with_state(state);

    info!("Gateway HTTP server listening on {}", addr);
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
