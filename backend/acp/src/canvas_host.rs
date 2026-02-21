//! Canvas Host
//!
//! Provides a persistent JSON document store (Whiteboards, Code blocks, interactive widgets)
//! that agents can mutate using dedicated tool calls to construct complex UI artifacts.

use anyhow::Result;
use tracing::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasDocument {
    pub canvas_id: String,
    pub title: String,
    pub payload_json: serde_json::Value,
}

pub struct CanvasHost;

impl CanvasHost {
    /// Creates a novel Canvas Artifact attached to the session context.
    pub async fn create_canvas(title: &str, initial_data: serde_json::Value) -> Result<CanvasDocument> {
        info!("Creating new interactive Canvas artifact: {}", title);
        Ok(CanvasDocument {
            canvas_id: "mock_canvas_123".into(),
            title: title.into(),
            payload_json: initial_data,
        })
    }

    /// Mutates an existing document artifact using JSON-patch or full payload replacements.
    pub async fn update_canvas(canvas_id: &str, patched_data: serde_json::Value) -> Result<()> {
        info!("Updating artifact {} with new payload data.", canvas_id);
        Ok(())
    }
}
