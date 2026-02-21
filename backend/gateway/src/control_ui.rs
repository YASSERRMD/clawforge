//! Control UI Static Asset Server.
//!
//! Mirrors `src/gateway/control-ui.ts`.

use axum::{routing::get, Router};
use std::convert::Infallible;
use crate::server::GatewayState;

/// Returns a router that serves the control UI static SPA assets.
pub fn ui_router() -> Router<GatewayState> {
    Router::new()
        // Here we would typically use `tower_http::services::ServeDir` to serve from a dir
        // e.g. .fallback_service(ServeDir::new("public/ui"))
        // But since we want to keep dependencies limited while compiling, we provide a mock.
        .route(
            "/",
            get(|| async { "ClawForge Control UI Dashboard (Mocked Static Server)" }),
        )
}
