/// ACP HTTP client — sends prompts to a remote ClawForge gateway acting as an agent.
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

use crate::types::{PermissionRequest, PermissionResponse, SpawnRequest, SubAgentSession};

pub struct AcpClient {
    base_url: String,
    http: Client,
    /// Optional bearer token for authentication
    token: Option<String>,
}

impl AcpClient {
    pub fn new(base_url: String, token: Option<String>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: Client::new(),
            token,
        }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(t) = &self.token {
            req.bearer_auth(t)
        } else {
            req
        }
    }

    /// Spawn a sub-agent on the remote gateway.
    pub async fn spawn_agent(&self, req: SpawnRequest) -> Result<SubAgentSession> {
        let url = format!("{}/api/acp/spawn", self.base_url);
        info!("[ACP] Spawning sub-agent via {}", url);
        let res = self
            .auth(self.http.post(&url).json(&req))
            .send()
            .await?
            .error_for_status()?
            .json::<SubAgentSession>()
            .await?;
        Ok(res)
    }

    /// Poll for the result of a sub-agent session.
    pub async fn get_session(&self, session_id: Uuid) -> Result<SubAgentSession> {
        let url = format!("{}/api/acp/sessions/{}", self.base_url, session_id);
        let res = self
            .auth(self.http.get(&url))
            .send()
            .await?
            .error_for_status()?
            .json::<SubAgentSession>()
            .await?;
        Ok(res)
    }

    /// Send a permission decision back to the requesting agent.
    pub async fn respond_permission(
        &self,
        session_id: Uuid,
        approved: bool,
    ) -> Result<()> {
        let url = format!("{}/api/acp/sessions/{}/permission", self.base_url, session_id);
        self.auth(self.http.post(&url).json(&PermissionResponse { approved }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
