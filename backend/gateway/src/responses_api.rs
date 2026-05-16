//! OpenResponses API Compatibility Layer
//!
//! Mirrors `src/gateway/call.ts` extended logic for SSE stream support.

use axum::response::sse::{Event, Sse};
use futures::stream::{self, Stream};
use std::convert::Infallible;
use tokio::time::Duration;
use tracing::info;

/// Endpoint for streamed completions.
/// Returns a Server-Sent Events (SSE) stream modeling OpenAI's delta chunks.
pub async fn stream_completions() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("Starting SSE stream for completions");
    
    // MOCK: Emit 5 chunks then the [DONE] terminator.
    // In a real implementation this listens to an mpsc receiver from AgentRunner.
    let stream = stream::unfold(0u32, |state| async move {
        if state < 5 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let data = serde_json::json!({
                "choices": [{ "delta": { "content": format!("chunk {}", state) } }]
            });
            match Event::default().json_data(data) {
                Ok(event) => Some((Ok(event), state + 1)),
                Err(e) => {
                    tracing::error!(error = %e, "Failed to serialize SSE chunk");
                    None
                }
            }
        } else if state == 5 {
            // Terminal [DONE] marker — return None next iteration to close the stream.
            Some((Ok(Event::default().data("[DONE]")), state + 1))
        } else {
            None
        }
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}
