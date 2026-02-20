use std::collections::HashMap;
use std::process::Stdio;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use clawforge_core::{
    AuditEventPayload, Capabilities, ClawError, Component, Event, EventKind,
    Message, ProposedAction,
    tools::ToolRegistry,
};

/// The Executor component receives ActionProposals, validates capabilities,
/// and executes approved actions.
pub struct Executor {
    supervisor_tx: mpsc::Sender<Message>,
}

impl Executor {
    pub fn new(supervisor_tx: mpsc::Sender<Message>) -> Self {
        Self { supervisor_tx }
    }

    /// Check if the proposed action is allowed by the agent's capabilities.
    fn check_capability(
        capabilities: &Capabilities,
        action: &ProposedAction,
    ) -> Result<(), ClawError> {
        match action {
            ProposedAction::ShellCommand { .. } => {
                if !capabilities.can_execute_commands {
                    return Err(ClawError::CapabilityDenied(
                        "shell command execution not allowed".to_string(),
                    ));
                }
            }
            ProposedAction::HttpRequest { url, .. } => {
                if !capabilities.can_make_http_requests {
                    return Err(ClawError::CapabilityDenied(
                        "HTTP requests not allowed".to_string(),
                    ));
                }
                // Check domain allowlist
                if !capabilities.allowed_domains.is_empty() {
                    let domain = url::Url::parse(url)
                        .ok()
                        .and_then(|u| u.host_str().map(|s| s.to_string()));
                    if let Some(domain) = domain {
                        if !capabilities
                            .allowed_domains
                            .iter()
                            .any(|d| domain.ends_with(d))
                        {
                            return Err(ClawError::CapabilityDenied(format!(
                                "domain '{}' not in allowed list",
                                domain
                            )));
                        }
                    }
                }
            }
            ProposedAction::LlmResponse { .. } => {
                // LLM responses are always allowed (they're data, not side-effects)
            }
            ProposedAction::ToolCall { .. } => {
                // For now, treat tools as always allowed if capabilities check passed upstream
                // In future, check specific tool permissions here
            }
        }
        Ok(())
    }

    /// Execute a shell command and return its output.
    async fn execute_shell(
        command: &str,
        args: &[String],
        working_dir: &Option<String>,
    ) -> Result<serde_json::Value> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        info!(command = %command, args = ?args, "Executing shell command");

        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(serde_json::json!({
            "exit_code": output.status.code(),
            "stdout": stdout,
            "stderr": stderr,
            "success": output.status.success(),
        }))
    }

    /// Execute an HTTP request and return the response.
    async fn execute_http(
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: &Option<String>,
    ) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();

        let mut request = match method.to_uppercase().as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            _ => anyhow::bail!("Unsupported HTTP method: {}", method),
        };

        for (key, value) in headers {
            request = request.header(key, value);
        }

        if let Some(body) = body {
            request = request.body(body.clone());
        }

        info!(method = %method, url = %url, "Executing HTTP request");

        let response = request.send().await?;
        let status = response.status().as_u16();
        let response_headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = response.text().await?;

        Ok(serde_json::json!({
            "status": status,
            "headers": response_headers,
            "body": body,
        }))
    }

    /// Execute a tool call.
    async fn execute_tool(
        registry: &ToolRegistry,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let tool = registry.get(name).ok_or_else(|| {
            anyhow::anyhow!("Tool '{}' not found", name)
        })?;
        
        info!(tool = %name, "Executing tool");
        let output = tool.execute(args).await?;
        
        Ok(serde_json::json!({
            "tool": name,
            "output": output
        }))
    }

    /// Send an audit event to the supervisor.
    async fn emit_event(&self, run_id: Uuid, agent_id: Uuid, kind: EventKind, payload: serde_json::Value) {
        let _ = self
            .supervisor_tx
            .send(Message::AuditEvent(AuditEventPayload {
                event: Event::new(run_id, agent_id, kind, payload),
            }))
            .await;
    }
}

#[async_trait]
impl Component for Executor {
    fn name(&self) -> &str {
        "executor"
    }

    async fn start(&self, mut rx: mpsc::Receiver<Message>) -> Result<()> {
        info!("Executor started");
        
        // Initialize standard tools
        let mut registry = ToolRegistry::new();
        registry.register(std::sync::Arc::new(clawforge_tools::ShellTool));
        registry.register(std::sync::Arc::new(clawforge_tools::FileReadTool));
        registry.register(std::sync::Arc::new(clawforge_tools::FileWriteTool));
        // Simple HTTP tool wrapper could be added here or we rely on built-in capability for now

        while let Some(msg) = rx.recv().await {
            match msg {
                Message::ExecuteAction(proposal) => {
                    let run_id = proposal.run_id;
                    let agent_id = proposal.agent_id;

                    info!(
                        run_id = %proposal.run_id,
                        step = proposal.step_index,
                        "Executing action"
                    );

                    // TODO: Check RunState from Supervisor before executing?
                    // For now, we proceed. In a real implementation, we would 
                    // check if run_state == Cancelled.
                    
                    // For capability checking we need the agent spec — for now use permissive defaults.
                    // In Phase 2 this will be looked up from the agent registry.
                    let capabilities = Capabilities {
                        can_read_files: true,
                        can_write_files: true,
                        can_execute_commands: true,
                        can_make_http_requests: true,
                        allowed_domains: vec![],
                        max_tokens_per_run: None,
                        max_cost_per_run_usd: None,
                    };

                    // Capability check
                    match Self::check_capability(&capabilities, &proposal.action) {
                        Ok(()) => {
                            self.emit_event(
                                run_id,
                                agent_id,
                                EventKind::ActionApproved,
                                serde_json::json!({"step": proposal.step_index}),
                            )
                            .await;
                        }
                        Err(e) => {
                            warn!(run_id = %run_id, error = %e, "Capability denied");
                            self.emit_event(
                                run_id,
                                agent_id,
                                EventKind::ActionDenied,
                                serde_json::json!({"error": e.to_string()}),
                            )
                            .await;
                            continue;
                        }
                    }

                    // Execute the action
                    let result = match &proposal.action {
                        ProposedAction::ShellCommand {
                            command,
                            args,
                            working_dir,
                        } => Self::execute_shell(command, args, working_dir).await,
                        ProposedAction::HttpRequest {
                            method,
                            url,
                            headers,
                            body,
                        } => Self::execute_http(method, url, headers, body).await,
                        ProposedAction::LlmResponse {
                            content,
                            provider,
                            model,
                            tokens_used,
                        } => {
                            info!(
                                provider = %provider,
                                model = %model,
                                tokens = tokens_used,
                                "LLM response received (no execution needed)"
                            );
                            Ok(serde_json::json!({
                                "type": "llm_response",
                                "content": content,
                                "provider": provider,
                                "model": model,
                                "tokens_used": tokens_used,
                            }))
                        },
                         ProposedAction::ToolCall {
                            name,
                            args,
                        } => Self::execute_tool(&registry, name, args.clone()).await,
                    };

                    match result {
                        Ok(output) => {
                            info!(run_id = %run_id, "Action executed successfully");
                            self.emit_event(
                                run_id,
                                agent_id,
                                EventKind::ActionExecuted,
                                output,
                            )
                            .await;
                            self.emit_event(
                                run_id,
                                agent_id,
                                EventKind::RunCompleted,
                                serde_json::json!({"completed_at": Utc::now().to_rfc3339()}),
                            )
                            .await;
                        }
                        Err(e) => {
                            error!(run_id = %run_id, error = %e, "Action execution failed");
                            self.emit_event(
                                run_id,
                                agent_id,
                                EventKind::ActionFailed,
                                serde_json::json!({"error": e.to_string()}),
                            )
                            .await;
                        }
                    }
                }
                other => {
                    debug!(msg_type = ?other, "Executor ignoring non-execute message");
                }
            }
        }

        info!("Executor channel closed, shutting down");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_check_shell_denied() {
        let caps = Capabilities::default(); // all false
        let action = ProposedAction::ShellCommand {
            command: "ls".into(),
            args: vec![],
            working_dir: None,
        };
        assert!(Executor::check_capability(&caps, &action).is_err());
    }

    #[test]
    fn test_capability_check_shell_allowed() {
        let caps = Capabilities {
            can_execute_commands: true,
            ..Default::default()
        };
        let action = ProposedAction::ShellCommand {
            command: "echo".into(),
            args: vec!["hello".into()],
            working_dir: None,
        };
        assert!(Executor::check_capability(&caps, &action).is_ok());
    }

    #[test]
    fn test_capability_check_http_domain_denied() {
        let caps = Capabilities {
            can_make_http_requests: true,
            allowed_domains: vec!["github.com".into()],
            ..Default::default()
        };
        let action = ProposedAction::HttpRequest {
            method: "GET".into(),
            url: "https://evil.com/data".into(),
            headers: HashMap::new(),
            body: None,
        };
        assert!(Executor::check_capability(&caps, &action).is_err());
    }

    #[test]
    fn test_capability_check_http_domain_allowed() {
        let caps = Capabilities {
            can_make_http_requests: true,
            allowed_domains: vec!["github.com".into()],
            ..Default::default()
        };
        let action = ProposedAction::HttpRequest {
            method: "GET".into(),
            url: "https://api.github.com/repos".into(),
            headers: HashMap::new(),
            body: None,
        };
        assert!(Executor::check_capability(&caps, &action).is_ok());
    }

    #[test]
    fn test_capability_check_llm_response_always_allowed() {
        let caps = Capabilities::default(); // all false
        let action = ProposedAction::LlmResponse {
            content: "analysis".into(),
            provider: "test".into(),
            model: "test".into(),
            tokens_used: 100,
        };
        assert!(Executor::check_capability(&caps, &action).is_ok());
    }
}
