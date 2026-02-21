//! Unix socket IPC server for interactive exec approval requests.
//!
//! When the agent needs to execute a command and the allowlist says "Ask",
//! it sends a request to this socket where a connected client (TUI, desktop app)
//! can grant/deny the approval in real-time.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, mpsc};
use std::path::Path;
use tracing::{debug, error, info, warn};

/// Request sent to the approval socket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    /// Unique request ID.
    pub id: String,
    /// The full command string awaiting approval.
    pub command: String,
    /// Agent session ID from which this request originated.
    pub session_id: String,
    /// Working directory for the command.
    pub cwd: Option<String>,
    /// Risk level from the analysis engine.
    pub risk_level: String,
    /// Human-readable reasons for this risk level.
    pub risk_reasons: Vec<String>,
}

/// Response from the approval socket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalResponse {
    /// Must match the request ID.
    pub id: String,
    /// "allow" | "deny" | "allow-session" | "deny-session"
    pub verdict: String,
}

/// Approval socket server: listens on a Unix socket for one client at a time.
pub struct ApprovalSocketServer {
    socket_path: std::path::PathBuf,
    response_tx: broadcast::Sender<ApprovalResponse>,
    request_tx: mpsc::Sender<ApprovalRequest>,
}

impl ApprovalSocketServer {
    /// Start the approval socket server at the given path.
    pub async fn start(
        socket_path: impl AsRef<Path>,
        request_tx: mpsc::Sender<ApprovalRequest>,
    ) -> Result<Self> {
        let socket_path = socket_path.as_ref().to_path_buf();

        // Remove stale socket file.
        if socket_path.exists() {
            tokio::fs::remove_file(&socket_path).await.ok();
        }

        let listener = UnixListener::bind(&socket_path)
            .with_context(|| format!("Failed to bind approval socket: {}", socket_path.display()))?;

        info!(socket = %socket_path.display(), "Approval socket server listening");

        let (response_tx, _) = broadcast::channel::<ApprovalResponse>(32);
        let response_tx_clone = response_tx.clone();
        let request_tx_clone = request_tx.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let tx = request_tx_clone.clone();
                        let rx = response_tx_clone.subscribe();
                        tokio::spawn(handle_client(stream, tx, rx));
                    }
                    Err(e) => {
                        error!("Approval socket accept error: {e}");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });

        Ok(Self {
            socket_path,
            response_tx,
            request_tx,
        })
    }

    /// Send an approval request and wait for a response (with timeout).
    pub async fn request_approval(
        &self,
        request: ApprovalRequest,
        timeout_secs: u64,
    ) -> Result<ApprovalResponse> {
        let id = request.id.clone();
        let mut rx = self.response_tx.subscribe();

        self.request_tx
            .send(request)
            .await
            .context("Failed to send approval request")?;

        let deadline = tokio::time::Instant::now()
            + tokio::time::Duration::from_secs(timeout_secs);

        loop {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Ok(resp)) if resp.id == id => return Ok(resp),
                Ok(Ok(_)) => continue, // different request, keep waiting
                Ok(Err(_)) => anyhow::bail!("Approval channel closed"),
                Err(_) => anyhow::bail!("Approval timeout after {timeout_secs}s"),
            }
        }
    }

    /// Broadcast a response (for testing or programmatic approval).
    pub fn send_response(&self, response: ApprovalResponse) {
        self.response_tx.send(response).ok();
    }
}

impl Drop for ApprovalSocketServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

async fn handle_client(
    stream: UnixStream,
    request_tx: mpsc::Sender<ApprovalRequest>,
    mut response_rx: broadcast::Receiver<ApprovalResponse>,
) {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // Relay incoming requests to the client.
    let write_task = tokio::spawn(async move {
        loop {
            match response_rx.recv().await {
                Ok(resp) => {
                    if let Ok(json) = serde_json::to_string(&resp) {
                        if write_half.write_all(json.as_bytes()).await.is_err()
                            || write_half.write_all(b"\n").await.is_err()
                        {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Read responses from client (approval verdicts).
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                // Client sends ApprovalResponse JSON lines
                if let Ok(resp) = serde_json::from_str::<ApprovalResponse>(trimmed) {
                    debug!(id = %resp.id, verdict = %resp.verdict, "Received approval verdict");
                    // Re-broadcast (write task will pick it up)
                    let _ = request_tx.send(ApprovalRequest {
                        id: resp.id,
                        command: String::new(),
                        session_id: String::new(),
                        cwd: None,
                        risk_level: String::new(),
                        risk_reasons: vec![],
                    }).await;
                } else {
                    warn!("Unparseable approval response: {trimmed}");
                }
            }
            Err(e) => {
                error!("Approval socket read error: {e}");
                break;
            }
        }
    }

    write_task.abort();
    info!("Approval socket client disconnected");
}
