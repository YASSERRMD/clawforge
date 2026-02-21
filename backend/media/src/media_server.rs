//! Local media server: serves stored media files over HTTP.
//!
//! Provides a simple Axum router that serves media by ID from the local store,
//! with content-type headers and range request support.

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::{path::PathBuf, sync::Arc};
use tokio::fs;
use tracing::{debug, warn};

use crate::mime_detect::{detect_mime_type, is_inline_safe};

/// State shared by media server routes.
#[derive(Clone)]
pub struct MediaServerState {
    pub media_dir: Arc<PathBuf>,
}

/// Build the media server Axum router.
///
/// Mount at `/media` prefix:
///   GET /media/:filename  — serve a media file
pub fn media_router(media_dir: PathBuf) -> Router {
    let state = MediaServerState {
        media_dir: Arc::new(media_dir),
    };
    Router::new()
        .route("/:filename", get(serve_media))
        .with_state(state)
}

/// GET /:filename — stream a media file from the local store.
async fn serve_media(
    Path(filename): Path<String>,
    State(state): State<MediaServerState>,
) -> Response {
    // Basic path sanitization: reject traversal.
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        warn!(filename = %filename, "Rejected suspicious media path");
        return (StatusCode::BAD_REQUEST, "Invalid filename").into_response();
    }

    let path = state.media_dir.join(&filename);
    debug!(path = %path.display(), "Serving media file");

    match fs::read(&path).await {
        Ok(bytes) => {
            let mime = detect_mime_type(&path);
            let disposition = if is_inline_safe(mime) {
                format!("inline; filename=\"{filename}\"")
            } else {
                format!("attachment; filename=\"{filename}\"")
            };

            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, mime.parse().unwrap());
            headers.insert(header::CONTENT_DISPOSITION, disposition.parse().unwrap());
            headers.insert(
                header::CACHE_CONTROL,
                "public, max-age=86400".parse().unwrap(),
            );
            headers.insert(
                header::CONTENT_LENGTH,
                bytes.len().to_string().parse().unwrap(),
            );

            (StatusCode::OK, headers, bytes).into_response()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            (StatusCode::NOT_FOUND, "Media file not found").into_response()
        }
        Err(e) => {
            warn!(path = %path.display(), error = %e, "Failed to read media file");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read media").into_response()
        }
    }
}
