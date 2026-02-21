//! OpenAI Compatible Endpoint (`/v1/chat/completions`).
//!
//! Mirrors `src/gateway/call.ts` / OpenResponses endpoints.

use axum::{
    extract::State,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::server::GatewayState;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<serde_json::Value>, // Chat completions messages array
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub choices: Vec<serde_json::Value>,
}

/// Handler for `POST /v1/chat/completions`.
/// Maps the OpenAI request to the internal ClawForge agent invocation.
pub async fn chat_completions(
    State(_state): State<GatewayState>,
    Json(payload): Json<ChatRequest>,
) -> Json<ChatResponse> {
    // MOCK: In reality, we route this to `AgentRunner`, wait for response, and map back to OpenAI format.
    
    let resp = ChatResponse {
        id: "chatcmpl-mock".into(),
        object: "chat.completion".into(),
        created: 1234567890,
        choices: vec![serde_json::json!({
            "index": 0,
            "message": {
                "role": "assistant",
                "content": format!("Echoing your model request for {}", payload.model)
            },
            "finish_reason": "stop"
        })],
    };
    
    Json(resp)
}
