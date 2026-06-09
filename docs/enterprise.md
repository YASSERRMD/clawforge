# Use Case - Enterprise IT

**Scenario:** An enterprise IT operations team wants an agent that executes
approved remediation runbooks (restart services, run diagnostics) against
internal systems via a ServiceNow MCP server. This is critical-risk automation.

## 1. Register the agent

The platform team registers an **"IT Ops Runbook Agent"** in the Registry:
owner `it-ops`, department `Information Technology`, tools `["shell", "http.get"]`,
MCP servers `["servicenow-mcp"]`, `data_access_level = confidential`,
`risk_level = critical`.

## 2. Govern the building blocks

Several things need approval, each tracked by the Governance Engine:

```rust
governance.submit(/* kind: Agent  */)?;  // the agent itself
governance.submit(/* kind: Mcp,  servicenow-mcp */)?;
governance.submit(/* kind: Tool, shell */)?;
```

The `servicenow-mcp` server is registered in **MCP Governance**, reviewed
(`requires_governance_review()` is true - it has a `write` tool), approved, and
health-checked.

## 3. Policy & budget

A `SecurityPolicy` for this environment enables database writes and external
network for the runbook agent, but keeps `require_human_approval = true` so its
critical-risk actions still require a human gate. A per-agent `budget_limit`
caps daily spend.

## 4. Guarded execution

Before each remediation step the Security Gateway verifies the tool and MCP
server are allow-listed, the model matches, the action is within budget, and - because the agent is critical-risk under a mandated gate - flags it for human
approval. Allowed steps proceed; everything is scored and logged.

## 5. Observe the fleet

Every run emits Observability events. The IT dashboard shows task success rate,
tool failure rate, average latency, total cost, MCP call volume, and any blocked
executions - for this agent and across the whole fleet (`summary(None)`).

## Why a control plane

Without ClawForge this agent would run with ad-hoc credentials, no central
approval, no per-action checks, and no unified audit trail. With it, IT gets
**Kubernetes-style lifecycle, ServiceNow-style approvals, and Splunk-style
observability** over their agent fleet - in one place.
