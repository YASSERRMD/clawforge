/// Session key — a stable composite key for mapping inbound messages to sessions.
///
/// Mirrors `src/routing/session-key.ts` from OpenClaw.
/// The session key combines (channel, thread_id, user_id) into a stable string
/// that can be used to look up or create the right agent session.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A stable routing key for session lookup.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionKey {
    pub channel: String,
    pub thread_id: Option<String>,
    pub user_id: Option<String>,
}

impl SessionKey {
    pub fn new(
        channel: impl Into<String>,
        thread_id: Option<impl Into<String>>,
        user_id: Option<impl Into<String>>,
    ) -> Self {
        Self {
            channel: channel.into(),
            thread_id: thread_id.map(Into::into),
            user_id: user_id.map(Into::into),
        }
    }

    /// A short stable hash usable as a session/file identifier.
    pub fn hash(&self) -> String {
        let raw = format!(
            "{}|{}|{}",
            self.channel,
            self.thread_id.as_deref().unwrap_or("_"),
            self.user_id.as_deref().unwrap_or("_")
        );
        let digest = Sha256::digest(raw.as_bytes());
        hex::encode(&digest[..8])
    }

    pub fn to_display_string(&self) -> String {
        let mut parts = vec![self.channel.clone()];
        if let Some(t) = &self.thread_id {
            parts.push(format!("thread:{}", t));
        }
        if let Some(u) = &self.user_id {
            parts.push(format!("user:{}", u));
        }
        parts.join("/")
    }
}

impl std::fmt::Display for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}
