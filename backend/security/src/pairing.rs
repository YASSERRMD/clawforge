/// Device pairing system — one-time setup codes and device token issuance.
///
/// Mirrors `src/pairing/pairing-store.ts` + `setup-code.ts` from OpenClaw.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Setup code
// ---------------------------------------------------------------------------

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn gen_code() -> String {
    // 6-digit numeric code derived from timestamp + subsec nanos
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    format!("{:06}", nanos % 1_000_000)
}

fn gen_token() -> String {
    let h = {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
        format!("{:x}{:x}", nanos, nanos.wrapping_mul(0xdeadbeef))
    };
    format!("cf_{}", h)
}

// ---------------------------------------------------------------------------
// Pairing store
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingCode {
    pub code: String,
    pub expires_at: u64,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedDevice {
    pub device_id: String,
    pub token: String,
    pub label: Option<String>,
    pub paired_at: u64,
}

#[derive(Debug, Default)]
pub struct PairingStore {
    pending: Arc<RwLock<HashMap<String, PendingCode>>>,     // code → PendingCode
    devices: Arc<RwLock<HashMap<String, PairedDevice>>>,   // device_id → PairedDevice
    tokens: Arc<RwLock<HashMap<String, String>>>,          // token → device_id
    /// Code validity window (seconds).
    pub code_ttl_secs: u64,
}

impl PairingStore {
    pub fn new(code_ttl_secs: u64) -> Self {
        Self { code_ttl_secs, ..Default::default() }
    }

    /// Generate a new one-time pairing code.
    pub fn generate_code(&self, label: Option<&str>) -> PendingCode {
        let code = gen_code();
        let entry = PendingCode {
            code: code.clone(),
            expires_at: now_secs() + self.code_ttl_secs,
            label: label.map(str::to_string),
        };
        self.pending.write().unwrap().insert(code.clone(), entry.clone());
        info!("[Pairing] Generated code {} (expires in {}s)", code, self.code_ttl_secs);
        entry
    }

    /// Verify a submitted code and, if valid, issue a device token.
    pub fn verify_code(&self, code: &str, device_id: &str) -> Result<PairedDevice> {
        let mut pending = self.pending.write().unwrap();
        let entry = pending.remove(code).ok_or_else(|| anyhow::anyhow!("Invalid or expired code"))?;

        if now_secs() > entry.expires_at {
            bail!("Pairing code has expired");
        }

        let token = gen_token();
        let device = PairedDevice {
            device_id: device_id.to_string(),
            token: token.clone(),
            label: entry.label.clone(),
            paired_at: now_secs(),
        };

        self.devices.write().unwrap().insert(device_id.to_string(), device.clone());
        self.tokens.write().unwrap().insert(token, device_id.to_string());
        info!("[Pairing] Device '{}' paired successfully", device_id);
        Ok(device)
    }

    /// Validate a device token. Returns the device_id if valid.
    pub fn validate_token(&self, token: &str) -> Option<String> {
        self.tokens.read().unwrap().get(token).cloned()
    }

    /// Revoke a paired device.
    pub fn revoke(&self, device_id: &str) {
        let mut devices = self.devices.write().unwrap();
        let mut tokens = self.tokens.write().unwrap();
        if let Some(device) = devices.remove(device_id) {
            tokens.remove(&device.token);
            warn!("[Pairing] Revoked device '{}'", device_id);
        }
    }

    pub fn list_devices(&self) -> Vec<PairedDevice> {
        self.devices.read().unwrap().values().cloned().collect()
    }
}
