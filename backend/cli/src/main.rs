mod api;
mod config;

use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

use clawforge_core::ClawBus;
use clawforge_executor::Executor;
use clawforge_planner::providers::ProviderRegistry;
use clawforge_planner::LlmPlanner;
use clawforge_scheduler::Scheduler;
use clawforge_supervisor::Supervisor;
use clawforge_supervisor::store::EventStore;

use api::AppState;
use config::Config;

#[derive(Parser)]
#[command(name = "clawforge")]
#[command(about = "ClawForge — Blazing-fast AI agent runtime")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the ClawForge runtime server
    Serve {
        /// Port to bind the HTTP server to
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Show current runtime status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env();

    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level)),
        )
        .json()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port } => {
            let config = Config {
                port: port.unwrap_or(config.port),
                ..config
            };
            run_server(config).await?;
        }
        Commands::Status => {
            println!("ClawForge status: checking...");
            let client = reqwest::Client::new();
            match client
                .get(format!("http://localhost:{}/api/health", config.port))
                .send()
                .await
            {
                Ok(resp) => {
                    let body: serde_json::Value = resp.json().await?;
                    println!("{}", serde_json::to_string_pretty(&body)?);
                }
                Err(_) => {
                    println!("ClawForge is not running on port {}", config.port);
                }
            }
        }
    }

    Ok(())
}

async fn run_server(config: Config) -> Result<()> {
    info!(
        port = config.port,
        bind = %config.bind_address,
        db = %config.db_path,
        "Starting ClawForge runtime"
    );

    // Initialize event store
    let event_store = EventStore::open(&config.db_path)?;
    let supervisor = Arc::new(Supervisor::new(event_store));

    // Initialize broadcast channel for real-time events
    let (broadcast_tx, _) = broadcast::channel(100);
    supervisor.set_broadcast_tx(broadcast_tx.clone()).await;

    // Initialize channel bus
    let mut bus = ClawBus::new();

    // Initialize provider registry
    let mut registry = ProviderRegistry::new();

    if let Some(api_key) = &config.openrouter_api_key {
        use clawforge_planner::providers::openrouter::OpenRouterProvider;
        registry.register("openrouter", Arc::new(OpenRouterProvider::new(api_key)));
        info!("Registered OpenRouter provider");
    }

    if let Some(url) = &config.ollama_url {
        use clawforge_planner::providers::ollama::OllamaProvider;
        registry.register("ollama", Arc::new(OllamaProvider::new().with_base_url(url)));
        info!(url = %url, "Registered Ollama provider");
    }

    let registry = Arc::new(registry);

    // Wire up components
    let planner = LlmPlanner::new(
        registry,
        bus.executor_tx.clone(),
        bus.supervisor_tx.clone(),
        None, // Memory disabled in main CLI for now
    );

    let executor = Executor::new(bus.supervisor_tx.clone());

    let scheduler = Scheduler::new(
        vec![], // No agents registered yet — Phase 2 adds dynamic registration
        bus.planner_tx.clone(),
        bus.supervisor_tx.clone(),
    );

    // Take receivers and start component tasks
    let scheduler_rx = bus.take_scheduler_rx().expect("scheduler rx already taken");
    let planner_rx = bus.take_planner_rx().expect("planner rx already taken");
    let executor_rx = bus.take_executor_rx().expect("executor rx already taken");
    let supervisor_rx = bus.take_supervisor_rx().expect("supervisor rx already taken");

    // Spawn component tasks
    let supervisor_ref = Arc::clone(&supervisor);
    tokio::spawn(async move {
        if let Err(e) = clawforge_core::Component::start(&*supervisor_ref, supervisor_rx).await {
            error!(error = %e, "Supervisor task failed");
        }
    });

    tokio::spawn(async move {
        if let Err(e) = clawforge_core::Component::start(&scheduler, scheduler_rx).await {
            error!(error = %e, "Scheduler task failed");
        }
    });

    tokio::spawn(async move {
        if let Err(e) = clawforge_core::Component::start(&planner, planner_rx).await {
            error!(error = %e, "Planner task failed");
        }
    });

    tokio::spawn(async move {
        if let Err(e) = clawforge_core::Component::start(&executor, executor_rx).await {
            error!(error = %e, "Executor task failed");
        }
    });

    info!("All components started");

    // Initialize endpoints
    let mut bb_router = None;
    if let (Some(url), Some(password)) = (&config.bluebubbles_server_url, &config.bluebubbles_password) {
        use clawforge_channels::bluebubbles::{BlueBubblesAdapter, BlueBubblesConfig};
        use clawforge_channels::ChannelAdapter;
        
        let bb_config = BlueBubblesConfig {
            server_url: url.clone(),
            password: password.clone(),
            webhook_path: config.bluebubbles_webhook_path.clone(),
        };
        
        let bb_adapter = BlueBubblesAdapter::new(bb_config, bus.supervisor_tx.clone());
        bb_router = Some(bb_adapter.build_router());
        
        let supervisor_tx_bb = bus.supervisor_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = bb_adapter.start(supervisor_tx_bb).await {
                error!("BlueBubbles adapter start failed: {}", e);
            }
        });
        info!("Registered BlueBubbles channel adapter");
    }

    // Start HTTP API
    let app_state = Arc::new(AppState {
        supervisor: Arc::clone(&supervisor),
        broadcast_tx,
        scheduler_tx: bus.scheduler_tx.clone(),
        supervisor_tx: bus.supervisor_tx.clone(),
    });

    let app = api::build_router(app_state, bb_router).layer(CorsLayer::permissive());
    let addr = format!("{}:{}", config.bind_address, config.port);

    info!(addr = %addr, "HTTP API listening");

    let listener = TcpListener::bind(&addr).await?;

    // Phase 12: Optional Tailscale funnel/serve automation
    // We spawn this so it runs after the server is up
    if std::env::var("CLAWFORGE_ENABLE_TAILSCALE").is_ok() {
        let port = config.port;
        tokio::spawn(async move {
            info!("Configuring Tailscale serve for port {}", port);
            let _ = std::process::Command::new("tailscale")
                .arg("serve")
                .arg(format!("--bg"))
                .arg(format!("localhost:{}", port))
                .output();
        });
    }

    axum::serve(listener, app).await?;

    Ok(())
}
