/// Matrix channel adapter for ClawForge.
///
/// Uses the Matrix Client-Server HTTP API (spec r0.6 / v3):
///  - Inbound: `GET /_matrix/client/v3/sync` long-poll loop
///  - Outbound: `PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message/{txnId}`
///
/// Required env vars:
///   MATRIX_HOMESERVER_URL — e.g. https://matrix.org
///   MATRIX_ACCESS_TOKEN   — user access token
///   MATRIX_USER_ID        — @bot:matrix.org (used to filter self-messages)
use crate::ChannelAdapter;
use anyhow::Result;
use async_trait::async_trait;
use clawforge_core::{AuditEventPayload, Event, EventKind, Message};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct MatrixConfig {
    pub homeserver_url: String,
    pub access_token: String,
    pub user_id: String,
}

// ---------------------------------------------------------------------------
// Matrix sync wire types (minimal subset)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug, Default)]
struct SyncResponse {
    next_batch: String,
    rooms: Option<SyncRooms>,
}

#[derive(Deserialize, Debug)]
struct SyncRooms {
    join: Option<std::collections::HashMap<String, JoinedRoom>>,
}

#[derive(Deserialize, Debug)]
struct JoinedRoom {
    timeline: Option<Timeline>,
}

#[derive(Deserialize, Debug)]
struct Timeline {
    events: Option<Vec<RoomEvent>>,
}

#[derive(Deserialize, Debug)]
struct RoomEvent {
    #[serde(rename = "type")]
    event_type: String,
    sender: Option<String>,
    event_id: Option<String>,
    content: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct SendMessageBody<'a> {
    msgtype: &'a str,
    body: &'a str,
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

pub struct MatrixAdapter {
    config: MatrixConfig,
    supervisor_tx: mpsc::Sender<Message>,
    http_client: Client,
}

impl MatrixAdapter {
    pub fn new(config: MatrixConfig, supervisor_tx: mpsc::Sender<Message>) -> Self {
        Self {
            config,
            supervisor_tx,
            http_client: Client::new(),
        }
    }

    fn sync_url(&self, since: Option<&str>) -> String {
        let base = format!(
            "{}_matrix/client/v3/sync?timeout=30000&access_token={}",
            self.config.homeserver_url.trim_end_matches('/'),
            self.config.access_token
        );
        if let Some(s) = since {
            format!("{}&since={}", base, s)
        } else {
            base
        }
    }

    fn send_url(&self, room_id: &str, txn_id: &str) -> String {
        format!(
            "{}_matrix/client/v3/rooms/{}/send/m.room.message/{}?access_token={}",
            self.config.homeserver_url.trim_end_matches('/'),
            urlencoding::encode(room_id),
            txn_id,
            self.config.access_token
        )
    }

    /// Long-poll /sync loop — posts inbound text events to the supervisor bus.
    async fn sync_loop(&self, supervisor_tx: mpsc::Sender<Message>) -> Result<()> {
        let mut since: Option<String> = None;

        // First call with no `since` to get initial next_batch (to skip history)
        match self.http_client.get(self.sync_url(None)).send().await {
            Ok(res) => {
                if let Ok(body) = res.json::<SyncResponse>().await {
                    since = Some(body.next_batch);
                }
            }
            Err(e) => {
                warn!("[Matrix] Initial sync failed: {}", e);
            }
        }

        info!("[Matrix] Starting sync loop (since: {:?})", since);

        loop {
            let url = self.sync_url(since.as_deref());
            match self.http_client.get(&url).send().await {
                Err(e) => {
                    error!("[Matrix] Sync request failed: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
                Ok(res) => {
                    if !res.status().is_success() {
                        error!("[Matrix] Sync returned {}", res.status());
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        continue;
                    }
                    let sync: SyncResponse = match res.json().await {
                        Ok(s) => s,
                        Err(e) => {
                            error!("[Matrix] Failed to parse sync response: {}", e);
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            continue;
                        }
                    };

                    since = Some(sync.next_batch.clone());

                    // Process timeline events for each joined room
                    if let Some(rooms) = sync.rooms {
                        if let Some(joined) = rooms.join {
                            for (room_id, room) in joined {
                                if let Some(timeline) = room.timeline {
                                    if let Some(events) = timeline.events {
                                        for ev in events {
                                            self.handle_room_event(
                                                &room_id,
                                                ev,
                                                &supervisor_tx,
                                            )
                                            .await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn handle_room_event(
        &self,
        room_id: &str,
        ev: RoomEvent,
        supervisor_tx: &mpsc::Sender<Message>,
    ) {
        // Only text messages
        if ev.event_type != "m.room.message" {
            return;
        }

        let sender = ev.sender.unwrap_or_default();

        // Ignore our own messages
        if sender == self.config.user_id {
            return;
        }

        let content = ev.content.unwrap_or_default();
        let msgtype = content
            .get("msgtype")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if msgtype != "m.text" {
            return;
        }

        let body = content
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if body.is_empty() {
            return;
        }

        info!("[Matrix] {} in {}: {}", sender, room_id, body);

        let event = Event::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            EventKind::RunStarted,
            serde_json::json!({
                "source": "matrix",
                "room_id": room_id,
                "sender": sender,
                "event_id": ev.event_id,
                "text": body,
            }),
        );

        let _ = supervisor_tx
            .send(Message::AuditEvent(AuditEventPayload { event }))
            .await;
    }
}

// ---------------------------------------------------------------------------
// ChannelAdapter impl
// ---------------------------------------------------------------------------

#[async_trait]
impl ChannelAdapter for MatrixAdapter {
    fn name(&self) -> &str { "matrix" }

    async fn start(&self, supervisor_tx: mpsc::Sender<Message>) -> Result<()> {
        info!("[Matrix] Starting sync loop");
        self.sync_loop(supervisor_tx).await
    }
}

impl MatrixAdapter {
    pub async fn send_message(&self, room_id: &str, text: &str) -> anyhow::Result<()> {
        let txn_id = Uuid::new_v4().to_string();
        let url = self.send_url(room_id, &txn_id);
        let body = SendMessageBody {
            msgtype: "m.text",
            body: text,
        };

        let res = self.http_client.put(&url).json(&body).send().await?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            error!("[Matrix] send failed to {}: {}", room_id, err);
            anyhow::bail!("Matrix send failed: {}", err);
        }
        info!("[Matrix] Sent message to room {}", room_id);
        Ok(())
    }
}
