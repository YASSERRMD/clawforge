# Security Disclaimer

ClawForge is software for **governing** AI agents. It helps an organisation
reduce risk, but it does not, by itself, make any deployment secure or
compliant. Read this before relying on it.

## Treat agent input as untrusted

The underlying runtime connects to real messaging surfaces (WhatsApp, Telegram,
Discord, Slack, …). Inbound messages, tool outputs, and MCP responses are
**untrusted input** and may attempt prompt injection. The Security Gateway gates
*capabilities* (which tool/MCP/model/data an action may use) — it does not, on
its own, sanitise content. Keep untrusted execution sandboxed (the runtime
supports Docker isolation).

## The control plane is an aid, not a guarantee

- It enforces the policies **you configure**. A permissive `SecurityPolicy`
  permits permissive behaviour.
- Approval actions currently trust the supplied actor identity — **add
  authentication/RBAC** (SSO/AD) before production. See [limitations](limitations.md).
- Digital signatures over audit evidence are a **placeholder**; do not treat
  unsigned evidence as cryptographically attested.
- Compliance reporting is **awareness-level**; it is not legal certification.

## Secrets

Never put secrets in the control plane. Integrations store a `CredentialRef`
(vault/env/SSO pointer) only. Keep real secrets in a proper vault and restrict
access to it.

## Responsible use

This project assists **authorised** operation and governance of AI agents in
environments you control. Do not use it to target systems you are not authorised
to operate, to evade legitimate oversight, or to process personal data without a
lawful basis.

## Reporting issues

Found a security issue? Open a private report to the maintainer rather than a
public issue, and avoid including sensitive data in reproduction steps.
