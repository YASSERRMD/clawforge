//! CLI Sessions Subcommands
//!
//! Subcommands for interacting directly with running chat sessions.

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum SessionCommands {
    /// List all active sessions
    List,
    /// Send a message to a session
    Send {
        #[arg(short, long)]
        session_id: String,
        #[arg(short, long)]
        message: String,
    },
    /// Show chat history of a session
    History {
        #[arg(short, long)]
        session_id: String,
    },
    /// Forcibly kill a session
    Kill {
        #[arg(short, long)]
        session_id: String,
    },
}

pub async fn run(cmd: SessionCommands) -> Result<()> {
    // MOCK: interact with ClawForge REST API or local database
    match cmd {
        SessionCommands::List => {
            println!("Active Sessions:");
            println!("  - id: 5fb4-11ad, Agent: default, State: Waiting");
        }
        SessionCommands::Send { session_id, message } => {
            println!("Sending message to {}: '{}'", session_id, message);
            println!("Response: (mocked success)");
        }
        SessionCommands::History { session_id } => {
            println!("History for {}:", session_id);
            println!("  User: Hello");
            println!("  Assistant: Hi there! How can I help you?");
        }
        SessionCommands::Kill { session_id } => {
            println!("Killed session {}", session_id);
        }
    }
    Ok(())
}
