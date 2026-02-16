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
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<String> {
        let command = args["command"].as_str().ok_or_else(|| anyhow::anyhow!("Missing 'command' argument"))?;

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        Ok(format!("Stdout:\n{}\nStderr:\n{}", stdout, stderr))
    }
}
