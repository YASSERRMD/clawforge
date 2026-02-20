use async_trait::async_trait;
use clawforge_core::traits::Tool;
use serde_json::json;

pub struct NodeInvocationTool {}

impl NodeInvocationTool {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Tool for NodeInvocationTool {
    fn name(&self) -> &str {
        "node.invoke"
    }

    fn description(&self) -> &str {
        "Invokes a command on a connected edge node (like a mobile phone or macos app)."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "node_id": { "type": "string" },
                "command": { "type": "string", "enum": ["camera.snap", "location.get", "system.run"] }
            },
            "required": ["node_id", "command"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<String> {
        let cmd = args.get("command").and_then(|c| c.as_str()).unwrap_or("unknown");
        Ok(format!("NodeInvocation Mock: Executed '{}' on remote node", cmd))
    }
}
