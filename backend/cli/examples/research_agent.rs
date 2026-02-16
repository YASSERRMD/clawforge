use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tracing::{info, Level};
use uuid::Uuid;

use clawforge_core::{
    AgentSpec, Capabilities, ClawBus, Component, EventKind, JobTrigger, LlmPolicy, Message, TriggerSpec,
    message::{MemoryQueryResponse, MemorySearchResult},
    types::{MemoryConfig, Role},
};
use clawforge_executor::Executor;
use clawforge_memory::{InMemoryVectorStore, MemoryStore, VectorEntry, MemoryQuery};
use clawforge_planner::providers::ProviderRegistry;
use clawforge_planner::LlmPlanner;
use clawforge_scheduler::Scheduler;
use clawforge_supervisor::store::EventStore;
use clawforge_supervisor::Supervisor;

/// A simple component that acts as the "Memory Service" for the demo.
struct MemoryService {
    store: Arc<InMemoryVectorStore>,
}

#[async_trait::async_trait]
impl Component for MemoryService {
    fn name(&self) -> &str {
        "memory_service"
    }

    async fn start(&self, mut rx: tokio::sync::mpsc::Receiver<Message>) -> Result<()> {
        info!("Memory Service started");
        while let Some(msg) = rx.recv().await {
            match msg {
                Message::MemoryQuery(req) => {
                    info!(run_id = %req.run_id, "Handling memory query");
                    
                    let query = MemoryQuery {
                        vector: req.query_vector,
                        min_score: req.min_score,
                        limit: req.limit,
                    };

                    match self.store.search(query).await {
                        Ok(results) => {
                            info!(run_id = %req.run_id, count = results.len(), "Memory search successful");
                            // Convert to core MemorySearchResult
                            let core_results: Vec<MemorySearchResult> = results.into_iter().map(|r| {
                                MemorySearchResult {
                                    content: r.entry.content,
                                    score: r.score,
                                    metadata: r.entry.metadata,
                                }
                            }).collect();

                            let response = MemoryQueryResponse {
                                run_id: req.run_id,
                                results: core_results,
                            };
                            
                            // We need to send this back to the planner.
                            // Currently ClawBus doesn't have a direct "reply" mechanism easily accessible here 
                            // unless we pass the bus or a sender channel.
                            // For this demo, let's assume we have a planner_tx injected.
                            // But Component trait doesn't have it.
                            // We'll modify MemoryService to hold planner_tx.
                        }
                        Err(e) => {
                            // in real system send error response
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

// Simplified version for demo: just spawn a task that listens to memory_rx and sends to planner_tx
async fn run_memory_service(
    store: Arc<InMemoryVectorStore>,
    mut rx: tokio::sync::mpsc::Receiver<Message>,
    planner_tx: tokio::sync::mpsc::Sender<Message>,
) {
    info!("Memory Service started");
    while let Some(msg) = rx.recv().await {
        if let Message::MemoryQuery(req) = msg {
            info!(run_id = %req.run_id, "Handling memory query");
             let query = MemoryQuery {
                vector: req.query_vector,
                min_score: req.min_score,
                limit: req.limit,
            };

            if let Ok(results) = store.search(query).await {
                info!(run_id = %req.run_id, count = results.len(), "Found memories");
                 let core_results: Vec<MemorySearchResult> = results.into_iter().map(|r| {
                    MemorySearchResult {
                        content: r.entry.content,
                        score: r.score,
                        metadata: r.entry.metadata,
                    }
                }).collect();

                let response = Message::MemoryResponse(MemoryQueryResponse {
                    run_id: req.run_id,
                    results: core_results,
                });
                
                let _ = planner_tx.send(response).await;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    info!("Starting Research Agent Demo");

    // 1. Setup Memory
    let memory_store = Arc::new(InMemoryVectorStore::new());
    
    // Pre-populate memory with some "facts" relevant to the task
    let fact1 = VectorEntry {
        id: Uuid::new_v4(),
        content: "Rust 1.75 stabilized async traits in traits.".to_string(),
        vector: vec![0.0; 1536], // Mock vector
        metadata: serde_json::json!({"source": "manual_entry"}),
    };
    memory_store.upsert(fact1).await?;

    // 2. Define Research Agent
    let researcher = AgentSpec {
        id: Uuid::new_v4(),
        name: "researcher".to_string(),
        description: "Research agent with memory".to_string(),
        trigger: TriggerSpec::Manual,
        capabilities: Capabilities::default(),
        llm_policy: LlmPolicy {
            providers: vec!["mock".to_string()],
            model: "mock".to_string(),
            max_tokens: 100,
            temperature: 0.0,
            system_prompt: "You are a research agent.".to_string(),
        },
        role: Role::Planner,
        memory_config: Some(MemoryConfig {
            collection_name: "facts".to_string(),
            embedding_model: "mock".to_string(),
        }),
        workflow: vec![],
    };

    // 3. Wiring
    let mut bus = ClawBus::new();
    // Add memory channel to bus (manually since ClawBus update is next step)
    let (memory_tx, memory_rx) = tokio::sync::mpsc::channel(100);
    
    let event_store = EventStore::in_memory()?;
    let supervisor = Arc::new(Supervisor::new(event_store));
    
    let mut registry = Arc::new(ProviderRegistry::new());
    // Use unsafe get_mut or just recreate since we haven't shared it yet
    let mut registry = ProviderRegistry::new();
    registry.register("mock", Arc::new(clawforge_planner::providers::mock::MockProvider::new("mock")));
    let registry = Arc::new(registry);
    
    let planner = LlmPlanner::new(
        registry,
        bus.executor_tx.clone(),
        bus.supervisor_tx.clone(),
        Some(memory_tx.clone()),
    );
    
    let executor = Executor::new(bus.supervisor_tx.clone());

    let scheduler = Scheduler::new(
        vec![researcher.clone()],
        bus.planner_tx.clone(),
        bus.supervisor_tx.clone(),
    );

    // 4. Start components
    let scheduler_rx = bus.take_scheduler_rx().unwrap();
    let planner_rx = bus.take_planner_rx().unwrap();
    let executor_rx = bus.take_executor_rx().unwrap();
    let supervisor_rx = bus.take_supervisor_rx().unwrap();

    let sup_ref = supervisor.clone();
    tokio::spawn(async move { sup_ref.start(supervisor_rx).await.unwrap() });
    tokio::spawn(async move { executor.start(executor_rx).await.unwrap() });
    tokio::spawn(async move { planner.start(planner_rx).await.unwrap() });
    
    // Start Memory Service
    let planner_tx_clone = bus.planner_tx.clone();
    tokio::spawn(async move {
        run_memory_service(memory_store, memory_rx, planner_tx_clone).await;
    });

    let scheduler_handle = tokio::spawn(async move {
        scheduler.start(scheduler_rx).await.unwrap();
    });

    // 5. Trigger Run
    info!("Triggering research agent...");
    bus.scheduler_tx.send(Message::ScheduleJob(JobTrigger {
        run_id: Uuid::new_v4(),
        agent_id: researcher.id,
        trigger_reason: "manual-demo".to_string(),
    })).await?;

    tokio::time::sleep(Duration::from_secs(3)).await;

    // 6. Verify Log
    let runs = supervisor.get_recent_runs(10)?;
    info!("Recent runs: {}", serde_json::to_string_pretty(&runs)?);

    Ok(())
}
