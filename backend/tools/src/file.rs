use async_trait::async_trait;
use clawforge_core::Tool;
use serde_json::Value;
use std::path::Path;
use tokio::fs;

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<String> {
        let path_str = args["path"].as_str().ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
        
        // Basic safety check: ensure path doesn't contain ".."
        if path_str.contains("..") {
             return Err(anyhow::anyhow!("Security violation: Path cannot contain '..'"));
        }

        let content = fs::read_to_string(path_str).await?;
        Ok(content)
    }
}

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file. Overwrites if exists."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<String> {
        let path_str = args["path"].as_str().ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
        let content = args["content"].as_str().ok_or_else(|| anyhow::anyhow!("Missing 'content' argument"))?;

        // Basic safety check
        if path_str.contains("..") {
             return Err(anyhow::anyhow!("Security violation: Path cannot contain '..'"));
        }

        // Ensure parent directory exists
        if let Some(parent) = Path::new(path_str).parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(path_str, content).await?;
        Ok(format!("Successfully wrote to {}", path_str))
    }
}
