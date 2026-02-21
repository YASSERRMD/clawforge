//! Gateway Auth Health Check
//!
//! Mirrors `src/gateway/auth-health.ts`.

use axum::Json;
use serde::Serialize;
use tracing::info;

use crate::auth::RequireAuth;

#[derive(Serialize)]
pub struct AuthHealthResponse {
    pub status: String,
    pub key_id: String,
    pub roles: Vec<String>,
}

/// Endpoint: `GET /api/v1/auth/health`
/// Checks if the provided bearer token is valid and returns its associated roles.
pub async fn check_auth_health(auth: RequireAuth) -> Json<AuthHealthResponse> {
    info!("Auth health check passed for key {}", auth.0.key_id);
    
    Json(AuthHealthResponse {
        status: "healthy".into(),
        key_id: auth.0.key_id,
        roles: auth.0.roles,
    })
}
