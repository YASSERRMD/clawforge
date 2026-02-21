//! Setup code generation and validation for device pairing.
//!
//! Generates short, human-readable setup codes for first-time device pairing.
//! Mirrors `src/pairing/setup-code.ts`.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use once_cell::sync::Lazy;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// A generated setup code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupCode {
    /// The display code shown to the user (e.g., "BLUE-TIGER-42").
    pub code: String,
    /// The session token to use after verification.
    pub session_token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

impl SetupCode {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn is_valid(&self) -> bool {
        !self.used && !self.is_expired()
    }
}

/// Adjectives and nouns for memorable code generation.
const ADJECTIVES: &[&str] = &[
    "BLUE", "RED", "SWIFT", "BRIGHT", "CALM", "BOLD", "COOL", "DARK",
    "FAST", "FINE", "GOLD", "HARD", "HIGH", "KEEN", "LOUD", "MILD",
];

const NOUNS: &[&str] = &[
    "TIGER", "EAGLE", "SHARK", "WOLF", "BEAR", "HAWK", "LION", "RAVEN",
    "FOX", "OWL", "SEAL", "STAG", "SWAN", "TOAD", "WREN", "LYNX",
];

/// Generate a human-readable code: ADJECTIVE-NOUN-NN (e.g., "BLUE-TIGER-42").
pub fn generate_code() -> String {
    let mut rng = rand::thread_rng();
    let adj = ADJECTIVES[rng.gen_range(0..ADJECTIVES.len())];
    let noun = NOUNS[rng.gen_range(0..NOUNS.len())];
    let num: u8 = rng.gen_range(10..99);
    format!("{adj}-{noun}-{num}")
}

/// Generate a cryptographically-random session token.
pub fn generate_session_token() -> String {
    use std::fmt::Write;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rand::Rng::r#gen::<u8>(&mut rng)).collect();
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// In-memory store of pending setup codes.
pub struct SetupCodeStore {
    codes: RwLock<HashMap<String, SetupCode>>,
    code_lifetime: Duration,
}

impl SetupCodeStore {
    pub fn new(lifetime_minutes: i64) -> Self {
        Self {
            codes: RwLock::new(HashMap::new()),
            code_lifetime: Duration::minutes(lifetime_minutes),
        }
    }

    /// Create and store a new setup code.
    pub async fn create(&self) -> SetupCode {
        let code = generate_code();
        let now = Utc::now();
        let entry = SetupCode {
            code: code.clone(),
            session_token: generate_session_token(),
            created_at: now,
            expires_at: now + self.code_lifetime,
            used: false,
        };
        self.codes.write().await.insert(code.clone(), entry.clone());
        info!(code = %code, expires_at = %entry.expires_at, "Setup code created");
        entry
    }

    /// Validate and consume a code. Returns the session token if valid.
    pub async fn consume(&self, code: &str) -> Result<String> {
        let mut codes = self.codes.write().await;
        let entry = codes
            .get_mut(code)
            .with_context(|| format!("Setup code '{code}' not found"))?;

        if entry.used {
            anyhow::bail!("Setup code '{code}' has already been used");
        }
        if entry.is_expired() {
            anyhow::bail!("Setup code '{code}' has expired");
        }

        let token = entry.session_token.clone();
        entry.used = true;
        debug!(code = %code, "Setup code consumed successfully");
        Ok(token)
    }

    /// Clean up expired codes.
    pub async fn cleanup_expired(&self) {
        let mut codes = self.codes.write().await;
        let before = codes.len();
        codes.retain(|_, v| !v.is_expired());
        let removed = before - codes.len();
        if removed > 0 {
            debug!(removed = %removed, "Cleaned up expired setup codes");
        }
    }

    /// Count currently valid (non-expired, non-used) setup codes.
    pub async fn valid_count(&self) -> usize {
        self.codes.read().await.values().filter(|c| c.is_valid()).count()
    }
}
