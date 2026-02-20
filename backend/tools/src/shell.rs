use async_trait::async_trait;
use clawforge_core::Tool;
use serde_json::Value;
use std::process::Command;

pub struct ShellTool;

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell_execute"
    }

    fn description(&self) -> &str {
        "Execute a shell command. Use this to run scripts, install packages, or interact with the system."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command line to execute"
                },
                "use_docker": {
                    "type": "boolean",
                    "description": "If true, run command inside a sandboxed ubuntu docker container"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<String> {
        let command = args["command"].as_str().ok_or_else(|| anyhow::anyhow!("Missing 'command' argument"))?;
        let use_docker = args["use_docker"].as_bool().unwrap_or(false);

        let output = if use_docker {
            Command::new("docker")
                .arg("run")
                .arg("--rm")
                .arg("ubuntu:latest")
                .arg("sh")
                .arg("-c")
                .arg(command)
                .output()?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()?
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        Ok(format!("Stdout:\n{}\nStderr:\n{}", stdout, stderr))
    }
}
