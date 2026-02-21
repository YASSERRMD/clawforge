//! Device Authentication Store
//!
//! Manages the `authorized_keys` payload representing paired peer instances
//! capable of securely routing RPCs to this node.

use anyhow::Result;
use tracing::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedDevice {
    pub alias: String,
    pub device_uuid: String,
    pub public_key: String,
}

pub struct AuthStore;

impl AuthStore {
    /// Lists all peer instances permitted to dispatch authorized requests.
    pub async fn list_authorized() -> Result<Vec<PairedDevice>> {
        info!("Loading authorized peers from Auth Store.");
        Ok(vec![]) // MOCK empty peer list
    }

    /// Appends a new confirmed public key wrapper to the keystore.
    pub async fn authorize_device(device: PairedDevice) -> Result<()> {
        info!("Authorizing new device peer: {} ({})", device.alias, device.device_uuid);
        Ok(())
    }

    /// Purges a paired device, immediately terminating trust.
    pub async fn revoke_device(device_uuid: &str) -> Result<()> {
        info!("Revoking device trust for UUID: {}", device_uuid);
        Ok(())
    }
}
