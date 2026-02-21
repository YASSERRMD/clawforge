//! Node Host invocation runner: manages connected device (nodes) and invokes tasks on them.
//!
//! Mirrors `src/node-host/index.ts` and `runner.ts`.

use anyhow::Result;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// A connected node (device/peer) registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRegistration {
    pub node_id: String,
    pub display_name: String,
    pub platform: String,
    pub capabilities: Vec<String>,
    /// Whether the node accepts task invocations.
    pub accepts_tasks: bool,
    /// Connection metadata (transport-specific).
    pub metadata: serde_json::Value,
}

/// Invocation request sent to a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInvocation {
    pub invocation_id: String,
    pub node_id: String,
    pub task: String,
    pub args: serde_json::Value,
    pub timeout_secs: Option<u64>,
}

/// Result from a node invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInvocationResult {
    pub invocation_id: String,
    pub node_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Status of a node in the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Online,
    Offline,
    Busy,
    Unknown,
}

/// Trait for node transport backends.
/// Not object-safe — use a concrete type with NodeHostRegistry<T>.
pub trait NodeTransport: Send + Sync + 'static {
    fn invoke(
        &self,
        invocation: NodeInvocation,
    ) -> impl std::future::Future<Output = Result<NodeInvocationResult>> + Send;

    fn ping(
        &self,
        node_id: &str,
    ) -> impl std::future::Future<Output = Result<bool>> + Send;
}

/// Node registry: tracks registered nodes and their status.
pub struct NodeHostRegistry<T: NodeTransport> {
    nodes: Arc<RwLock<HashMap<String, (NodeRegistration, NodeStatus)>>>,
    transport: Arc<T>,
}

impl<T: NodeTransport> NodeHostRegistry<T> {
    pub fn new(transport: T) -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            transport: Arc::new(transport),
        }
    }

    /// Register a new node.
    pub async fn register(&self, registration: NodeRegistration) {
        let id = registration.node_id.clone();
        self.nodes.write().await.insert(id.clone(), (registration, NodeStatus::Online));
        info!(node_id = %id, "Node registered");
    }

    /// Remove a node from the registry.
    pub async fn deregister(&self, node_id: &str) {
        self.nodes.write().await.remove(node_id);
        info!(node_id = %node_id, "Node deregistered");
    }

    /// Invoke a task on a specific node.
    pub async fn invoke(&self, node_id: &str, task: &str, args: serde_json::Value, timeout_secs: Option<u64>) -> Result<NodeInvocationResult> {
        let nodes = self.nodes.read().await;
        let (reg, status) = nodes
            .get(node_id)
            .ok_or_else(|| anyhow::anyhow!("Node '{node_id}' not found"))?;

        if *status == NodeStatus::Offline {
            anyhow::bail!("Node '{node_id}' is offline");
        }

        if !reg.accepts_tasks {
            anyhow::bail!("Node '{node_id}' does not accept task invocations");
        }

        drop(nodes);

        let invocation = NodeInvocation {
            invocation_id: Uuid::new_v4().simple().to_string(),
            node_id: node_id.to_string(),
            task: task.to_string(),
            args,
            timeout_secs,
        };

        debug!(node_id = %node_id, task = %task, "Invoking task on node");
        self.transport.invoke(invocation).await
    }

    /// Ping all nodes to update their status.
    pub async fn refresh_all(&self) {
        let ids: Vec<String> = self.nodes.read().await.keys().cloned().collect();
        for id in ids {
            let alive = self.transport.ping(&id).await.unwrap_or(false);
            let status = if alive { NodeStatus::Online } else { NodeStatus::Offline };
            if let Some(entry) = self.nodes.write().await.get_mut(&id) {
                if entry.1 != status {
                    warn!(node_id = %id, ?status, "Node status changed");
                    entry.1 = status;
                }
            }
        }
    }

    /// List all registered nodes.
    pub async fn list(&self) -> Vec<(NodeRegistration, NodeStatus)> {
        self.nodes.read().await.values().cloned().collect()
    }
}
