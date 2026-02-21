//! Docker-based sandbox: container lifecycle via bollard.
//!
//! Manages per-session sandboxed Docker containers for safe code execution.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Configuration for a sandbox container.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerSandboxConfig {
    /// Docker image to use (default: clawforge-sandbox:latest).
    pub image: String,
    /// Memory limit (e.g. "512m", "1g").
    pub memory_limit: Option<String>,
    /// CPU quota (0.0–1.0 fraction of one core).
    pub cpu_quota: Option<f64>,
    /// Network mode ("none", "bridge", "host").
    pub network_mode: String,
    /// Environment variables to inject.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Workspace directory mount: (host_path, container_path).
    pub workspace_mount: Option<(String, String)>,
    /// Max container lifetime in seconds before forced kill.
    pub max_lifetime_secs: Option<u64>,
}

impl Default for DockerSandboxConfig {
    fn default() -> Self {
        Self {
            image: "clawforge-sandbox:latest".to_string(),
            memory_limit: Some("512m".to_string()),
            cpu_quota: Some(0.5),
            network_mode: "none".to_string(),
            env: HashMap::new(),
            workspace_mount: None,
            max_lifetime_secs: Some(3600),
        }
    }
}

/// Result of executing a command inside a container.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerExecResult {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

/// Lightweight Docker client wrapper.
/// Uses the Docker socket directly via HTTP over Unix socket.
pub struct DockerSandbox {
    config: DockerSandboxConfig,
    container_id: Option<String>,
}

impl DockerSandbox {
    pub fn new(config: DockerSandboxConfig) -> Self {
        Self { config, container_id: None }
    }

    /// Start a new container for this sandbox session.
    pub async fn start(&mut self, session_id: &str) -> Result<String> {
        let container_name = format!("clawforge-sandbox-{}", sanitize_id(session_id));
        info!(container = %container_name, image = %self.config.image, "Starting sandbox container");

        // Build docker run args.
        let mut args = vec![
            "docker".to_string(),
            "run".to_string(),
            "-d".to_string(),
            "--name".to_string(), container_name.clone(),
            "--network".to_string(), self.config.network_mode.clone(),
        ];

        if let Some(mem) = &self.config.memory_limit {
            args.push("-m".to_string());
            args.push(mem.clone());
        }

        if let Some(cpu) = self.config.cpu_quota {
            // Convert fraction to period/quota (100000 period).
            let quota = (cpu * 100_000.0) as i64;
            args.push("--cpu-period=100000".to_string());
            args.push(format!("--cpu-quota={quota}"));
        }

        if let Some((host, container)) = &self.config.workspace_mount {
            args.push("-v".to_string());
            args.push(format!("{host}:{container}:rw"));
        }

        for (key, val) in &self.config.env {
            args.push("-e".to_string());
            args.push(format!("{key}={val}"));
        }

        // Add image and a sleep to keep container alive.
        args.push(self.config.image.clone());
        args.push("sleep".to_string());
        args.push(
            self.config
                .max_lifetime_secs
                .unwrap_or(3600)
                .to_string(),
        );

        let output = tokio::process::Command::new(&args[0])
            .args(&args[1..])
            .output()
            .await
            .context("Failed to run docker command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("docker run failed: {stderr}");
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!(container_id = %container_id, "Sandbox container started");
        self.container_id = Some(container_id.clone());
        Ok(container_id)
    }

    /// Execute a command inside the running container.
    pub async fn exec(
        &self,
        command: &[&str],
        timeout_secs: Option<u64>,
    ) -> Result<ContainerExecResult> {
        let container_id = self
            .container_id
            .as_deref()
            .context("Container not started")?;

        let mut args = vec!["docker", "exec", container_id];
        args.extend_from_slice(command);

        debug!(container = %container_id, cmd = ?command, "Executing in sandbox");

        let timeout = std::time::Duration::from_secs(timeout_secs.unwrap_or(30));
        let result = tokio::time::timeout(timeout, async {
            tokio::process::Command::new(args[0])
                .args(&args[1..])
                .output()
                .await
        })
        .await;

        match result {
            Ok(Ok(output)) => {
                let exit_code = output.status.code().unwrap_or(-1) as i64;
                Ok(ContainerExecResult {
                    exit_code,
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    timed_out: false,
                })
            }
            Ok(Err(e)) => anyhow::bail!("docker exec failed: {e}"),
            Err(_) => Ok(ContainerExecResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Command timed out after {}s", timeout_secs.unwrap_or(30)),
                timed_out: true,
            }),
        }
    }

    /// Stop and remove the container.
    pub async fn stop(&mut self) -> Result<()> {
        let Some(id) = self.container_id.take() else {
            return Ok(());
        };
        info!(container = %id, "Stopping sandbox container");
        let _ = tokio::process::Command::new("docker")
            .args(["rm", "-f", &id])
            .output()
            .await;
        Ok(())
    }

    /// Copy a file from the host into the container.
    pub async fn copy_in(&self, host_path: &str, container_path: &str) -> Result<()> {
        let id = self.container_id.as_deref().context("Container not started")?;
        let output = tokio::process::Command::new("docker")
            .args(["cp", host_path, &format!("{id}:{container_path}")])
            .output()
            .await?;
        if !output.status.success() {
            anyhow::bail!(
                "docker cp failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    /// Copy a file from the container to the host.
    pub async fn copy_out(&self, container_path: &str, host_path: &str) -> Result<()> {
        let id = self.container_id.as_deref().context("Container not started")?;
        let output = tokio::process::Command::new("docker")
            .args(["cp", &format!("{id}:{container_path}"), host_path])
            .output()
            .await?;
        if !output.status.success() {
            anyhow::bail!(
                "docker cp failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    pub fn container_id(&self) -> Option<&str> {
        self.container_id.as_deref()
    }
}

impl Drop for DockerSandbox {
    fn drop(&mut self) {
        if let Some(id) = &self.container_id {
            let id = id.clone();
            warn!(container = %id, "DockerSandbox dropped without explicit stop; removing container");
            let _ = std::process::Command::new("docker")
                .args(["rm", "-f", &id])
                .output();
        }
    }
}

fn sanitize_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect()
}
