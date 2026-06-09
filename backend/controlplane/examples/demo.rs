//! End-to-end control-plane demo.
//!
//! Run with:
//!
//! ```bash
//! cargo run -p clawforge-controlplane --example demo
//! ```
//!
//! Walks a single agent through the whole control plane: publish from the
//! marketplace, register an MCP server, govern the agent through approval,
//! check an action at the security gateway, record observability, and produce
//! a compliance report — all in-memory.

use clawforge_controlplane::compliance::{
    ApprovalChain, CompliancePolicy, ComplianceReport, PiiClassification,
};
use clawforge_controlplane::constants::{DataAccessLevel, LifecycleStatus, RiskLevel};
use clawforge_controlplane::gateway::{ActionRequest, SecurityGateway, SecurityPolicy};
use clawforge_controlplane::governance::{ApprovalKind, GovernanceEngine, NewApprovalRequest};
use clawforge_controlplane::marketplace::Marketplace;
use clawforge_controlplane::mcp::{McpRegistry, McpTool, NewMcpServer, TransportType};
use clawforge_controlplane::observability::{NewExecutionEvent, ObservabilityStore};
use clawforge_controlplane::registry::AgentRegistry;

fn main() -> anyhow::Result<()> {
    println!("== ClawForge Control Plane demo ==\n");

    // Stores (in-memory for the demo).
    let registry = AgentRegistry::in_memory()?;
    let governance = GovernanceEngine::in_memory()?;
    let mcp = McpRegistry::in_memory()?;
    let marketplace = Marketplace::in_memory()?;
    let obs = ObservabilityStore::in_memory()?;

    // 1. Marketplace → install a verified template into the registry.
    let listings = clawforge_controlplane::marketplace::seed::seed(&marketplace)?;
    let listing = &listings[0];
    println!("1. Marketplace listing '{}' trusted={}", listing.name, listing.is_trusted());
    let agent = marketplace.install(&listing.id, &registry, "Permit Bot A", "team-a", "Licensing")?;
    println!("   installed agent {} (status {:?})", agent.name, agent.status);

    // 2. MCP governance → register + approve the server the agent needs.
    let server = mcp.register(NewMcpServer {
        name: "records-mcp".into(),
        description: "Resident records".into(),
        owner: "data-platform".into(),
        endpoint: "https://mcp.internal/records".into(),
        transport: TransportType::Http,
        tools_exposed: vec![McpTool {
            name: "lookup".into(),
            description: "read records".into(),
            permissions: vec!["read".into()],
        }],
        permissions_required: vec!["read".into()],
        risk_level: RiskLevel::High,
    })?;
    mcp.approve(&server.id)?;
    println!("2. MCP server '{}' approved", server.name);

    // 3. Governance → approve the agent, then move it to Active.
    let req = governance.submit(NewApprovalRequest {
        kind: ApprovalKind::Agent,
        subject_id: agent.id.clone(),
        subject_name: agent.name.clone(),
        requested_by: "team-a".into(),
        department: "Licensing".into(),
        risk_level: agent.risk_level,
        justification: "Permit triage".into(),
    })?;
    governance.approve(&req.id, "ciso", "meets data-access policy")?;
    registry.set_status(&agent.id, LifecycleStatus::PendingApproval)?;
    let agent = registry.set_status(&agent.id, LifecycleStatus::Active)?;
    println!("3. Governance approved; agent now {:?}", agent.status);

    // 4. Security gateway → check an action before execution.
    let gateway = SecurityGateway::new(SecurityPolicy::permissive());
    let mut action = ActionRequest::for_agent(agent.clone());
    action.tool = Some("search".into());
    action.mcp_server = Some("records-mcp".into());
    action.data_access_level = DataAccessLevel::Internal;
    action.estimated_cost = 0.02;
    let decision = gateway.evaluate(&action);
    println!("4. Gateway decision: {}", decision.summary());

    // 5. Observability → record the execution.
    obs.log_event(NewExecutionEvent::task(&agent.id, decision.allowed, 120, 0.02))?;
    let metrics = obs.summary(Some(&agent.id))?;
    println!(
        "5. Observability: {} task(s), success rate {:.0}%",
        metrics.task_count,
        metrics.success_rate() * 100.0
    );

    // 6. Compliance → classify and report.
    let mut policy = CompliancePolicy::pdpl(&agent.id);
    policy.pii_classification = PiiClassification::Pii;
    policy.data_retention_days = 365;
    let chain = ApprovalChain::from_roles(&agent.id, &["data-owner", "dpo"]);
    let report = ComplianceReport::generate(&policy, &[], Some(&chain));
    println!(
        "6. Compliance: framework {}, compliant={} ({} finding(s))",
        report.framework,
        report.is_compliant(),
        report.findings.len()
    );
    for f in &report.findings {
        println!("     - {f}");
    }

    println!("\n== demo complete ==");
    Ok(())
}
