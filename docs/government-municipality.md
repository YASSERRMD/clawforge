# Use Case - Government Municipality

**Scenario:** A municipality wants an AI agent to triage building-permit
applications. The agent must read resident records (regulated PII) and is
therefore high-risk.

## 1. Register / install

The Licensing team installs the verified **"Permit Intake Assistant"** template
from the Marketplace. It lands in the Registry as `Draft`, owned by
`licensing-platform`, with `data_access_level = internal` and
`risk_level = high`.

## 2. Classify for PDPL

A compliance policy is attached:

```rust
let mut policy = CompliancePolicy::pdpl(&agent.id);
policy.pii_classification = PiiClassification::Pii;
policy.data_retention_days = 365;          // statutory retention
policy.export_control = ExportControl::Prohibited;  // data stays in-country
```

## 3. Multi-party approval

Because the agent touches resident PII, governance requires a chain:

```rust
let mut chain = ApprovalChain::from_roles(&agent.id, &["data-owner", "dpo", "ciso"]);
chain.approve_next("data.owner@municipality", now);
chain.approve_next("dpo@municipality", now);
chain.approve_next("ciso@municipality", now);   // chain complete
```

A matching `governance.submit(...)` / `approve(...)` records the decision and
history; the agent then moves `Draft → PendingApproval → Active`.

## 4. Guarded execution

When the agent tries to look up a resident record, the Security Gateway checks:
agent is `Active`, the `records-mcp` server is allow-listed, the data access is
within clearance, PII access is permitted by policy, and budget remains. A
high-risk action under a mandated approval gate is held for human sign-off; any
denial is written to the blocked-execution log.

## 5. Evidence & reporting

Each sensitive access is captured as `AuditEvidence`. At review time:

```rust
let report = ComplianceReport::generate(&policy, &evidence, Some(&chain));
assert!(report.is_compliant());  // PII + retention + completed chain + evidence
let dept = DepartmentComplianceSummary::summarize("Licensing", &[report]);
```

The municipality can now demonstrate, with reproducible evidence, *who approved
the agent, what it accessed, and that retention and export rules were enforced*
- the core of a PDPL accountability story.
