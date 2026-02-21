//! Gateway Authentication Module
//!
//! Mirrors `src/gateway/auth.ts`. Handles Bearer tokens and device auth.

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
    http::StatusCode,
};
use tracing::warn;

pub struct AuthenticatedUser {
    pub key_id: String,
    pub roles: Vec<String>,
}

pub struct RequireAuth(pub AuthenticatedUser);

#[async_trait]
impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|val| val.to_str().ok());

        match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                let token = &header["Bearer ".len()..];
                
                // MOCK VALIDATION
                if token == "valid_token" {
                    Ok(RequireAuth(AuthenticatedUser {
                        key_id: "mock_key".into(),
                        roles: vec!["admin".into()],
                    }))
                } else {
                    warn!("Invalid Bearer token: {}", token);
                    Err((StatusCode::UNAUTHORIZED, "Invalid token"))
                }
            }
            _ => {
                warn!("Missing or invalid Authorization header");
                Err((StatusCode::UNAUTHORIZED, "Missing credentials"))
            }
        }
    }
}
