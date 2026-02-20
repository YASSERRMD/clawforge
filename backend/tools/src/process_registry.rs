/// Process registry — track running background shell processes per session.
///
/// Mirrors `src/agents/bash-process-registry.ts` from OpenClaw.
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Process entry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub pid: u32,
    pub session_id: String,
    pub command: String,
    pub started_at: Instant,
    pub label: Option<String>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct ProcessRegistry {
    processes: Arc<RwLock<HashMap<u32, ProcessEntry>>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, entry: ProcessEntry) {
        let pid = entry.pid;
        info!("[ProcessRegistry] Registered process {} ({:?}) for session {}",
            pid, entry.label, entry.session_id);
        self.processes.write().unwrap().insert(pid, entry);
    }

    pub fn remove(&self, pid: u32) {
        self.processes.write().unwrap().remove(&pid);
        info!("[ProcessRegistry] Removed process {}", pid);
    }

    pub fn list_for_session(&self, session_id: &str) -> Vec<ProcessEntry> {
        self.processes
            .read()
            .unwrap()
            .values()
            .filter(|e| e.session_id == session_id)
            .cloned()
            .collect()
    }

    pub fn kill(&self, pid: u32) {
        #[cfg(unix)]
        {
            unsafe { libc_kill(pid as i32, 15); } // SIGTERM
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output();
        }
        self.remove(pid);
        warn!("[ProcessRegistry] Killed process {}", pid);
    }

    pub fn kill_session(&self, session_id: &str) {
        let pids: Vec<u32> = self.processes
            .read()
            .unwrap()
            .values()
            .filter(|e| e.session_id == session_id)
            .map(|e| e.pid)
            .collect();
        for pid in pids { self.kill(pid); }
    }
}

#[cfg(unix)]
extern "C" {
    fn kill(pid: libc_pid_t, sig: libc_c_int) -> libc_c_int;
}

#[cfg(unix)]
type libc_pid_t = i32;
#[cfg(unix)]
type libc_c_int = i32;

#[cfg(unix)]
unsafe fn libc_kill(pid: i32, sig: i32) {
    unsafe { kill(pid, sig); }
}
