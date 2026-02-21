//! WhatsApp Groups Routing
//!
//! Resolves WhatsApp Group JIDs into discrete session spaces for the LLM.

use anyhow::Result;
use tracing::info;

pub struct WaGroups;

impl WaGroups {
    /// Reconciles a Participant JID and a Chat JID to determine if the message
    /// belongs to a group, tracking replies and @mentions.
    pub fn resolve_session_id(remote_jid: &str, participant: Option<&str>) -> String {
        info!("Resolving WA Session space for {} (participant: {:?})", remote_jid, participant);
        if remote_jid.ends_with("@g.us") {
            // Group Chat: Treat the whole group as one session context.
            format!("wa-group-{}", remote_jid)
        } else {
            // Direct Message
            format!("wa-{}", remote_jid)
        }
    }

    /// Fetches all participants in a WA Group to build a Mention Map for the LLM.
    pub async fn fetch_participants(group_jid: &str) -> Result<Vec<String>> {
        info!("Syncing participants list for WA Group: {}", group_jid);
        // MOCK: Query Baileys Node proxy.
        Ok(vec!["p1@s.whatsapp.net".into(), "p2@s.whatsapp.net".into()])
    }
}
