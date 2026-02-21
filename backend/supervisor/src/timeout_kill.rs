//! Timeout Killer
//!
//! Escalates signals on a process if it takes too long.
//! E.g., SIGTERM -> wait -> SIGKILL.

use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::kill_tree::KillTree;

pub struct TimeoutKill;

impl TimeoutKill {
    /// Wait for `timeout`, then send SIGTERM. If process is still alive after `grace_period`, send SIGKILL tree.
    pub async fn execute_with_timeout(
        pid: u32,
        timeout: Duration,
        grace_period: Duration,
    ) -> Result<()> {
        info!("Starting timeout monitor for PID {} (timeout: {:?})", pid, timeout);
        
        sleep(timeout).await;
        
        // MOCK: Check if alive
        let is_alive = true; // Assume still running
        
        if is_alive {
            warn!("Process {} exceeded timeout. Sending SIGTERM...", pid);
            // MOCK: libc::kill(pid, SIGTERM)
            
            sleep(grace_period).await;
            
            // Check again
            let still_alive = true;
            if still_alive {
                warn!("Process {} ignored SIGTERM. Escalating to SIGKILL tree.", pid);
                KillTree::kill_tree(pid).await?;
            }
        }

        Ok(())
    }
}
