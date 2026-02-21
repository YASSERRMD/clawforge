//! Web Attachments Uploader
//!
//! Exposes an endpoint to upload attachments for multimodal inputs.

use axum::{body::Bytes, Json};
use serde::Serialize;
use tracing::info;

#[derive(Serialize)]
pub struct UploadResponse {
    pub attachment_id: String,
    pub filename: String,
    pub size_bytes: usize,
}

/// Endpoint for uploading attachments directly to Gateway.
pub async fn upload_attachment(body: Bytes) -> Json<UploadResponse> {
    info!("Received attachment payload of size {}", body.len());
    
    // MOCK: Real implementation would calculate hash, store to blob storage, and return ID.
    Json(UploadResponse {
        attachment_id: uuid::Uuid::new_v4().to_string(),
        filename: "upload.bin".into(),
        size_bytes: body.len(),
    })
}
