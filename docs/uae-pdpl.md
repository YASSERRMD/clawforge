# UAE PDPL — Awareness Note

> **Disclaimer:** This note is an engineering aid, not legal advice. It
> summarises how ClawForge's compliance pack *maps to* commonly-cited UAE
> Personal Data Protection Law (Federal Decree-Law No. 45 of 2021) concepts.
> Confirm obligations with your legal/DPO function.

## Why PDPL matters here

ClawForge is built for UAE government entities and municipalities, where agents
routinely touch resident data. The PDPL governs how personal data of
individuals in the UAE is processed, and the compliance pack is designed so an
organisation can demonstrate the relevant controls.

## How ClawForge concepts map to PDPL themes

| PDPL theme | ClawForge mechanism |
|------------|---------------------|
| Lawful, purposed processing | Agent registry records purpose (`description`), owner, and department |
| Data classification | `PiiClassification` (`non_pii` / `pii` / `sensitive_pii`) on `CompliancePolicy` |
| Data minimisation & access control | Security Gateway `DataAccessLevel` checks + agent clearance |
| Consent / approval for sensitive processing | `ApprovalChain` (e.g. data-owner → DPO → CISO) |
| Storage limitation / retention | `data_retention_days` + `is_past_retention()` |
| Cross-border transfer controls | `ExportControl` (`unrestricted` / `restricted` / `prohibited`) |
| Accountability & record-keeping | Audit trails (governance, gateway, integrations) + `AuditEvidence` |
| Breach / investigation handling | `investigation_mode` legal hold + evidence collection |
| Demonstrating compliance | `ComplianceReport` & `DepartmentComplianceSummary` |

## Sensitive personal data

For `sensitive_pii` (e.g. health, biometric, religious data), the pack expects:

- a **completed approval chain** before processing,
- a **defined retention period** (not indefinite),
- **collected audit evidence**, ideally digitally signed.

The `ComplianceReport::generate` function flags any of these that are missing.

## What this is *not*

ClawForge does not, by itself, make a deployment "PDPL compliant". It provides
the **controls, classification, and evidence** an organisation needs to operate
agents responsibly and to produce compliance reports for review. Legal
sufficiency is determined by the entity and its regulator.
