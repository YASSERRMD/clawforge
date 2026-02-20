/// IRC adapter — connects to an IRC server using the IRC protocol (TCP).
/// Runs a read loop forwarding PRIVMSG events to the supervisor.
use anyhow::Result;
use async_trait::async_trait;
use axum::Router;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc,
};
use tracing::{error, info};
use uuid::Uuid;

use clawforge_core::{AuditEventPayload, Event, EventKind, Message};

use crate::ChannelAdapter;

pub struct IrcConfig {
    pub server: String,
    pub port: u16,
    pub nick: String,
    pub channels: Vec<String>,
    pub password: Option<String>,
}

pub struct IrcAdapter {
    config: IrcConfig,
    supervisor_tx: mpsc::Sender<Message>,
}

impl IrcAdapter {
    pub fn new(config: IrcConfig, supervisor_tx: mpsc::Sender<Message>) -> Self {
        Self { config, supervisor_tx }
    }
}

#[async_trait]
impl ChannelAdapter for IrcAdapter {
    fn name(&self) -> &str { "irc" }

    fn build_router(&self) -> Router { Router::new() }

    async fn start(&self, supervisor_tx: mpsc::Sender<Message>) -> Result<()> {
        let addr = format!("{}:{}", self.config.server, self.config.port);
        info!("[IRC] Connecting to {}", addr);

        let stream = TcpStream::connect(&addr).await?;
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        // Authenticate and join channels
        if let Some(pass) = &self.config.password {
            writer.write_all(format!("PASS {}\r\n", pass).as_bytes()).await?;
        }
        writer.write_all(format!("NICK {}\r\n", self.config.nick).as_bytes()).await?;
        writer.write_all(format!("USER {} 0 * :ClawForge Bot\r\n", self.config.nick).as_bytes()).await?;
        for ch in &self.config.channels {
            writer.write_all(format!("JOIN {}\r\n", ch).as_bytes()).await?;
        }

        info!("[IRC] Connected to {} as {}", addr, self.config.nick);

        while let Ok(Some(line)) = lines.next_line().await {
            // Respond to PING
            if line.starts_with("PING ") {
                let pong = format!("PONG {}\r\n", &line[5..]);
                let _ = writer.write_all(pong.as_bytes()).await;
                continue;
            }

            // Parse PRIVMSG: :nick!user@host PRIVMSG #channel :text
            if let Some(msg) = parse_privmsg(&line) {
                info!("[IRC] <{}> {}: {}", msg.channel, msg.nick, msg.text);
                let event = Event::new(
                    Uuid::new_v4(), Uuid::new_v4(), EventKind::RunStarted,
                    serde_json::json!({ "source": "irc", "nick": msg.nick, "channel": msg.channel, "text": msg.text }),
                );
                let _ = supervisor_tx.send(Message::AuditEvent(AuditEventPayload { event })).await;
            }
        }

        Ok(())
    }
}

struct PrivMsg { nick: String, channel: String, text: String }

fn parse_privmsg(line: &str) -> Option<PrivMsg> {
    // :nick!user@host PRIVMSG #channel :text
    if !line.contains(" PRIVMSG ") { return None; }
    let nick = line.trim_start_matches(':').split('!').next()?.to_string();
    let mut parts = line.splitn(4, ' ');
    parts.next(); // :nick!...
    parts.next(); // PRIVMSG
    let channel = parts.next()?.to_string();
    let text = parts.next()?.trim_start_matches(':').to_string();
    Some(PrivMsg { nick, channel, text })
}
