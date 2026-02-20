/// Bash exec runtime — spawn shell commands with optional PTY.
///
/// Mirrors `src/agents/bash-tools.exec.ts` + `bash-tools.process.ts` from OpenClaw.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tracing::{info, warn};

use crate::process_registry::{ProcessEntry, ProcessRegistry};

// ---------------------------------------------------------------------------
// Exec config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecConfig {
    pub command: String,
    /// Working directory (defaults to agent workspace).
    pub cwd: Option<String>,
    /// Optional label for process registry.
    pub label: Option<String>,
    /// If true, run in background (don't wait for completion).
    pub background: bool,
    /// Timeout in seconds (0 = no timeout).
    pub timeout_secs: u64,
    /// Maximum output bytes to capture.
    pub max_output_bytes: usize,
    /// Environment overrides.
    pub env: Vec<(String, String)>,
}

impl Default for ExecConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            cwd: None,
            label: None,
            background: false,
            timeout_secs: 120,
            max_output_bytes: 200_000,
            env: vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Exec result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
    pub truncated: bool,
}

// ---------------------------------------------------------------------------
// Exec host
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecHost {
    /// Run on the gateway's local OS.
    Gateway,
    /// Run inside a sandbox (Docker/ephemeral environment).
    Sandbox,
}

// ---------------------------------------------------------------------------
// Spawn + run
// ---------------------------------------------------------------------------

pub async fn exec_command(
    session_id: &str,
    config: &ExecConfig,
    registry: Option<&ProcessRegistry>,
) -> Result<ExecResult> {
    if config.command.trim().is_empty() {
        bail!("Empty command");
    }
    info!("[BashExec] Running: {:?} (bg={})", &config.command[..config.command.len().min(80)], config.background);

    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&config.command);

    if let Some(cwd) = &config.cwd {
        cmd.current_dir(cwd);
    }

    for (k, v) in &config.env {
        cmd.env(k, v);
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let pid = child.id().unwrap_or(0);

    if let Some(reg) = registry {
        reg.register(ProcessEntry {
            pid,
            session_id: session_id.to_string(),
            command: config.command.clone(),
            started_at: std::time::Instant::now(),
            label: config.label.clone(),
        });
    }

    if config.background {
        // Detach — don't wait for completion
        tokio::spawn(async move { let _ = child.wait().await; });
        return Ok(ExecResult {
            stdout: format!("[Background process started, pid={}]", pid),
            stderr: String::new(),
            exit_code: 0,
            timed_out: false,
            truncated: false,
        });
    }

    // Collect output with timeout
    let timeout = if config.timeout_secs > 0 {
        Duration::from_secs(config.timeout_secs)
    } else {
        Duration::from_secs(600)
    };

    let max = config.max_output_bytes;

    let result = tokio::time::timeout(timeout, async move {
        let mut stdout_handle = child.stdout.take().unwrap();
        let mut stderr_handle = child.stderr.take().unwrap();

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        let _ = stdout_handle.read_to_end(&mut stdout_buf).await;
        let _ = stderr_handle.read_to_end(&mut stderr_buf).await;

        let status = child.wait().await?;
        let code = status.code().unwrap_or(-1);

        let truncated = stdout_buf.len() > max || stderr_buf.len() > max;
        stdout_buf.truncate(max);
        stderr_buf.truncate(max);

        Ok::<_, anyhow::Error>(ExecResult {
            stdout: String::from_utf8_lossy(&stdout_buf).to_string(),
            stderr: String::from_utf8_lossy(&stderr_buf).to_string(),
            exit_code: code,
            timed_out: false,
            truncated,
        })
    })
    .await;

    if let Some(reg) = registry {
        reg.remove(pid);
    }

    match result {
        Ok(Ok(r)) => Ok(r),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            warn!("[BashExec] Command timed out after {}s", config.timeout_secs);
            Ok(ExecResult {
                stdout: String::new(),
                stderr: format!("Command timed out after {}s", config.timeout_secs),
                exit_code: -1,
                timed_out: true,
                truncated: false,
            })
        }
    }
}
