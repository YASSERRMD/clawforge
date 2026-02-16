
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tracing::{info, Level};
use uuid::Uuid;

use clawforge_core::{
    types::Role, ActionType, AgentSpec, Capabilities, ClawBus, Component, FailurePolicy, JobTrigger, LlmPolicy, Message,
    TriggerSpec, WorkflowStep,
};
use clawforge_executor::Executor;
use clawforge_planner::providers::ProviderRegistry;
use clawforge_planner::LlmPlanner;
use clawforge_scheduler::Scheduler;
use clawforge_supervisor::store::EventStore;
use clawforge_supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting GitHub PR Reviewer Demo");

    // 2. Setup configuration (mock env vars for demo)
    std::env::set_var("OPENROUTER_API_KEY", "sk-mock-key");
    std::env::set_var("OLLAMA_URL", "http://localhost:11434");

    // 3. Define the PR Reviewer Agent
    let pr_reviewer = AgentSpec {
        id: Uuid::new_v4(),
        name: "pr-reviewer".to_string(),
        description: "Checks for new PRs and reviews them".to_string(),
        trigger: TriggerSpec::Interval { seconds: 10 }, // Check every 10s for demo
        capabilities: Capabilities {
            can_read_files: false,
            can_write_files: false,
            can_execute_commands: false,
            can_make_http_requests: true,
            allowed_domains: vec!["api.github.com".to_string()],
            max_tokens_per_run: Some(1000),
            max_cost_per_run_usd: Some(0.10),
        },
        llm_policy: LlmPolicy {
            providers: vec!["openrouter".to_string(), "ollama".to_string()],
            model: "openai/gpt-4o-mini".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            system_prompt: "You are a senior Rust engineer reviewing code.".to_string(),
        },
        role: Role::Executor,
        memory_config: None,
        workflow: vec![
            WorkflowStep {
                name: "fetch_pr".to_string(),
                action: ActionType::ShellCommand {
                    command: "git".to_string(),
                    args: vec!["fetch".to_string(), "origin".to_string(), "pull/123/head:pr-123".to_string()],
                },
                on_failure: FailurePolicy::Stop,
            },
        ],
        allowed_tools: vec![],
    };

    info!(agent_id = %pr_reviewer.id, "Defined PR Reviewer agent");

    // 4. Initialize components
    let event_store = EventStore::in_memory()?;
    let supervisor = Arc::new(Supervisor::new(event_store));

    let mut bus = ClawBus::new();

    // Mock provider registry for demo
    let mut registry = ProviderRegistry::new();
    // In a real run we'd register actua providers. For this example we'll let it fail/warn if no API key
    // or use Ollama if available.

    let registry = Arc::new(registry);

    let planner = LlmPlanner::new(
        registry,
        bus.executor_tx.clone(),
        bus.supervisor_tx.clone(),
        None,
    );

    let executor = Executor::new(bus.supervisor_tx.clone());

    let scheduler = Scheduler::new(
        vec![pr_reviewer.clone()],
        bus.planner_tx.clone(),
        bus.supervisor_tx.clone(),
    );

    // 5. Start components
    let scheduler_rx = bus.take_scheduler_rx().unwrap();
    let planner_rx = bus.take_planner_rx().unwrap();
    let executor_rx = bus.take_executor_rx().unwrap();
    let supervisor_rx = bus.take_supervisor_rx().unwrap();

    let supervisor_ref = Arc::clone(&supervisor);
    tokio::spawn(async move {
        supervisor_ref.start(supervisor_rx).await.unwrap();
    });

    tokio::spawn(async move {
        executor.start(executor_rx).await.unwrap();
    });

    tokio::spawn(async move {
        planner.start(planner_rx).await.unwrap();
    });

    // 6. Run Scheduler (main driver)
    // We'll run it in a separate task so we can manually trigger it too
    let scheduler_handle = tokio::spawn(async move {
        scheduler.start(scheduler_rx).await.unwrap();
    });

    // 7. Simulate manual trigger to start immediate review
    info!("Manually triggering PR review...");
    bus.scheduler_tx
        .send(Message::ScheduleJob(JobTrigger {
            run_id: Uuid::new_v4(),
            agent_id: pr_reviewer.id,
            trigger_reason: "manual-demo".to_string(),
        }))
        .await?;

    // 8. Let it run for 5 seconds then exit
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // 9. Check results
    let runs = supervisor.get_recent_runs(10)?;
    info!("Demo finished. Recent runs: {}", serde_json::to_string_pretty(&runs)?);

    Ok(())
}
