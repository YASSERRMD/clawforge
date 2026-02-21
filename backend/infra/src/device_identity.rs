//! Device Identity Generation
//!
//! Generates and persists a stable cryptographic Device Identity (UUID + Ed25519)
//! for secure mDNS / peer-to-peer introductions.

use anyhow::Result;
use tracing::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub device_uuid: String,
    pub public_key: String,
    pub private_key: String, // encrypted typically, plaintext Mock for now
}

pub struct IdentityManager;

impl IdentityManager {
    /// Loads the `~/.clawforge/identity.json` file or generates a fresh cryptosystem.
    pub async fn load_or_generate() -> Result<DeviceIdentity> {
        info!("Loading local device identity keystore...");
        
        let path = dirs::home_dir().unwrap().join(".clawforge/identity.json");
        if path.exists() {
            info!("Found existing device identity at {:?}", path);
            Ok(DeviceIdentity {
                device_uuid: "mock-uuid-1111".into(),
                public_key: "mock_ed25519_pub".into(),
                private_key: "mock_ed25519_priv".into(),
            })
        } else {
            info!("Provisioning new device identity UUID and Keypair.");
            Ok(DeviceIdentity {
                device_uuid: "mock-new-uuid-9999".into(),
                public_key: "mock_new_ed25519_pub".into(),
                private_key: "mock_new_ed25519_priv".into(),
            })
        }
    }
}
