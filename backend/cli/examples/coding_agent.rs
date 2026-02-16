use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tracing::{info, Level};
use uuid::Uuid;

use clawforge_core::{
    AgentSpec, Capabilities, ClawBus, Component, EventKind, JobTrigger, LlmPolicy, Message, TriggerSpec,
    types::Role,
};
use clawforge_executor::Executor;
use clawforge_planner::providers::{ProviderRegistry, mock::MockProvider};
use clawforge_planner::LlmPlanner;
use clawforge_scheduler::Scheduler;
use clawforge_supervisor::store::EventStore;
use clawforge_supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Setup Logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    info!("Starting Coding Agent Demo");

    // 2. Define Coding Agent
    let coding_agent = AgentSpec {
        id: Uuid::new_v4(),
        name: "python-coder".to_string(),
        description: "Agent that writes python code".to_string(),
        trigger: TriggerSpec::Manual,
        capabilities: Capabilities {
            can_read_files: true,
            can_write_files: true,
            can_execute_commands: true,
            can_make_http_requests: false,
            allowed_domains: vec![],
            max_tokens_per_run: None,
            max_cost_per_run_usd: None,
        },
        llm_policy: LlmPolicy {
            providers: vec!["mock".to_string()],
            model: "mock".to_string(),
            max_tokens: 100,
            temperature: 0.0,
            system_prompt: "You are a coding agent.".to_string(),
        },
        role: Role::Planner,
        memory_config: None,
        workflow: vec![],
        allowed_tools: vec!["file_write".to_string()],
    };

    // 3. Wiring
    let mut bus = ClawBus::new();
    
    let event_store = EventStore::in_memory()?;
    let supervisor = Arc::new(Supervisor::new(event_store));
    
    // Setup Mock Provider with a SPECIFIC tool call response
    let mut registry = ProviderRegistry::new();
    let mock_response = "Action: file_write({\"path\": \"hello_demo.py\", \"content\": \"print('Hello from ClawForge Tool!')\"})";
    
    let mock_provider = MockProvider::new("mock").with_response(mock_response);
    registry.register("mock", Arc::new(mock_provider));
    let registry = Arc::new(registry);
    
    // Planner
    let planner = LlmPlanner::new(
        registry,
        bus.executor_tx.clone(),
        bus.supervisor_tx.clone(),
        None, // No memory needed for this demo
    );
    
    // Executor
    let executor = Executor::new(bus.supervisor_tx.clone());

    // Scheduler
    let scheduler = Scheduler::new(
        vec![coding_agent.clone()],
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
    
    let _scheduler_handle = tokio::spawn(async move {
        scheduler.start(scheduler_rx).await.unwrap();
    });

    // 5. Trigger Run
    info!("Triggering coding agent...");
    bus.scheduler_tx.send(Message::ScheduleJob(JobTrigger {
        run_id: Uuid::new_v4(),
        agent_id: coding_agent.id,
        trigger_reason: "manual-demo".to_string(),
    })).await?;

    // 6. Wait for execution
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 7. Verify Log
    let runs = supervisor.get_recent_runs(5)?;
    info!("Runs: {}", serde_json::to_string_pretty(&runs)?);

    // 8. checks
    if std::path::Path::new("hello_demo.py").exists() {
        info!("SUCCESS: hello_demo.py was created!");
        let content = std::fs::read_to_string("hello_demo.py")?;
        info!("File content: {}", content);
        // Cleanup
        std::fs::remove_file("hello_demo.py")?;
    } else {
        info!("FAILURE: hello_demo.py was NOT created.");
    }

    Ok(())
}
