//! Gateway Authentication Module
//!
//! Validates Bearer tokens against the CLAWFORGE_API_KEY environment variable.
//! Set CLAWFORGE_API_KEY to a strong random secret before deployment.
//! If the env var is unset the gateway rejects all authenticated requests.

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
        let expected = match std::env::var("CLAWFORGE_API_KEY") {
            Ok(k) if !k.is_empty() => k,
            _ => {
                warn!("CLAWFORGE_API_KEY is not set — all authenticated requests will be rejected");
                return Err((StatusCode::UNAUTHORIZED, "Server not configured for auth"));
            }
        };

        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|val| val.to_str().ok());

        match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                let token = &header["Bearer ".len()..];
                if token == expected {
                    Ok(RequireAuth(AuthenticatedUser {
                        key_id: "api_key".into(),
                        roles: vec!["admin".into()],
                    }))
                } else {
                    warn!("Invalid Bearer token presented");
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
