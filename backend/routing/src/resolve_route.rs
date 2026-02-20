/// Route resolver — map an inbound message to the correct agent session.
///
/// Mirrors `src/routing/resolve-route.ts` from OpenClaw.
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{debug, info};
use serde::{Deserialize, Serialize};

use crate::session_key::SessionKey;

// ---------------------------------------------------------------------------
// Route binding
// ---------------------------------------------------------------------------

/// An explicit channel → agent binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteBinding {
    pub channel: String,
    pub agent_id: String,
    /// If set, only messages from this thread are routed here.
    pub thread_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Route result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum RouteResult {
    /// Route to an existing session.
    ExistingSession { session_id: String, agent_id: String },
    /// Create a new session for this agent.
    NewSession { agent_id: String },
    /// No matching route found.
    Unrouted,
}

// ---------------------------------------------------------------------------
// Resolver
// ---------------------------------------------------------------------------

/// Thread-safe route resolver.
#[derive(Default, Clone)]
pub struct RouteResolver {
    /// Explicit channel bindings configured by the operator.
    bindings: Arc<RwLock<Vec<RouteBinding>>>,
    /// Active session map: SessionKey.hash() → session_id
    sessions: Arc<RwLock<HashMap<String, String>>>,
}

impl RouteResolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a static routing binding.
    pub async fn add_binding(&self, binding: RouteBinding) {
        self.bindings.write().await.push(binding);
    }

    /// Register an active session for a session key.
    pub async fn register_session(&self, key: &SessionKey, session_id: impl Into<String>) {
        let hash = key.hash();
        info!("[Router] Registered session {} → {}", key, hash);
        self.sessions.write().await.insert(hash, session_id.into());
    }

    /// Remove a session from the registry.
    pub async fn unregister_session(&self, key: &SessionKey) {
        self.sessions.write().await.remove(&key.hash());
    }

    /// Resolve an inbound message to a route.
    pub async fn resolve(&self, key: &SessionKey) -> RouteResult {
        // 1. Check for an existing active session
        let hash = key.hash();
        if let Some(session_id) = self.sessions.read().await.get(&hash) {
            debug!("[Router] {} → existing session {}", key, session_id);
            // We'd need the agent_id here; in a full impl we'd store it alongside
            return RouteResult::ExistingSession {
                session_id: session_id.clone(),
                agent_id: "unknown".to_string(),
            };
        }

        // 2. Check explicit bindings
        let bindings = self.bindings.read().await;
        for binding in bindings.iter() {
            if binding.channel == key.channel {
                if let Some(tid) = &binding.thread_id {
                    if key.thread_id.as_deref() != Some(tid.as_str()) {
                        continue;
                    }
                }
                info!("[Router] {} → binding agent {}", key, binding.agent_id);
                return RouteResult::NewSession { agent_id: binding.agent_id.clone() };
            }
        }

        RouteResult::Unrouted
    }
}
