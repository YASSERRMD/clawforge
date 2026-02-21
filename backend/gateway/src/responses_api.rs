//! OpenResponses API Compatibility Layer
//!
//! Mirrors `src/gateway/call.ts` extended logic for SSE stream support.

use axum::response::sse::{Event, Sse};
use futures::stream::{self, Stream};
use futures::StreamExt;
use std::convert::Infallible;
use tokio::time::Duration;
use tracing::info;

/// Endpoint for streamed completions.
/// Returns a Server-Sent Events (SSE) stream modeling OpenAI's delta chunks.
pub async fn stream_completions() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("Starting SSE stream for completions");
    
    // MOCK: Emit some chunks and then stop.
    // In a real implementation this would listen to an mpsc receiver from AgentRunner.
    let stream = stream::unfold(0, |state| async move {
        if state < 5 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let data = serde_json::json!({
                "choices": [{
                    "delta": {
                        "content": format!("chunk {}", state)
                    }
                }]
            });
            let event = Event::default().json_data(data).unwrap();
            Some((Ok(event), state + 1))
        } else {
            // Send [DONE] marker as expected by OpenAI compat clients
            let event = Event::default().data("[DONE]");
            Some((Ok(event), state + 1)) // But actually we should just return None to stop. Let's finish properly.
        }
    }).take(6); // 5 chunks + 1 done

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}
