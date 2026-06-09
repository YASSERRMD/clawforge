# Limitations

An honest account of what the control plane does and does not do today, so
evaluators can judge it accurately.

## Scope

- **Library, not yet a service.** The control plane is a Rust crate
  (`clawforge-controlplane`) with SQLite-backed stores and a clean API. It is
  not yet exposed over HTTP, and there is no dedicated UI for it (the runtime's
  React dashboard is separate). See the [roadmap](roadmap.md).
- **Not auto-wired into the runtime.** `SecurityGateway::evaluate` and the
  observability event log are designed to sit in the execution path, but the
  live executor does not call them automatically yet.

## Security & identity

- **No built-in authentication / RBAC.** Governance `approve`/`reject` and
  status changes take an actor string but do not yet verify identity or
  enforce role-based access. Wire this to SSO/AD before production.
- **Digital signatures are a placeholder.** `AuditEvidence` carries a
  `signature` field and `is_signed()`, but no signing/verification is
  implemented.
- **No secret management.** By design, the control plane stores credential
  *references* only; it relies on an external vault/SSO for the actual secrets.

## Data & scale

- **Single-node SQLite.** Each store uses a local SQLite database - excellent
  for local-first and small deployments, but not a clustered/HA datastore.
- **No multi-tenancy.** There is no tenant isolation layer yet; an
  `organization` config field exists but is not enforced across stores.

## Compliance

- **Awareness, not certification.** The UAE PDPL mapping is an engineering aid.
  ClawForge provides the controls and evidence; legal sufficiency is determined
  by the entity and its regulator. See [uae-pdpl.md](uae-pdpl.md).
- **Retention is advisory.** `is_past_retention` reports when data is past due;
  it does not yet delete or archive anything automatically.

None of these are architectural dead-ends - they are the difference between a
solid, tested foundation and a fully operationalised platform, and each is
tracked on the [roadmap](roadmap.md).
