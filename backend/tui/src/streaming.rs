//! Streaming Consumer
//!
//! Consumes Server-Sent Events (SSE) from the ClawForge Gateway
//! to render tokens incrementally inside the TUI state.

use anyhow::Result;
use reqwest::Client;
use futures_util::StreamExt;

/// Connects to the local Gateway and yields incoming deltas.
pub async fn start_sse_consumer(session_id: &str, tx: tokio::sync::mpsc::Sender<String>) -> Result<()> {
    let client = Client::new();
    
    // MOCK: connect to actual streaming endpoint
    let url = format!("http://localhost:4000/v1/chat/completions/stream?session={}", session_id);
    
    // Mock the incoming loop
    let mut chunks = vec!["Hello", " there", "!", " I", " am", " processing..."];
    
    for chunk in chunks {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        tx.send(chunk.into()).await.ok();
    }

    Ok(())
}
