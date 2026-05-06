---
type: doc
name: security
description: Security policies, authentication, secrets management, and compliance requirements
category: security
generated: 2026-05-06
status: filled
scaffoldVersion: "2.0.0"
---

# Security & Compliance Notes

Security work should preserve local user trust: do not leak tokens, do not broaden tool permissions silently, and avoid destructive operations without explicit confirmation.

## Authentication & Authorization
Auth-related contracts and implementations live in `crates/jcode-auth-types`, `crates/jcode-azure-auth`, and provider-specific runtime code. OAuth flows are documented in `OAUTH.md`. Treat provider credentials and session tokens as sensitive.

## Secrets & Sensitive Data
- Never commit tokens, API keys, cookies, or local session files.
- Redact credentials from logs, screenshots, telemetry, and test fixtures.
- Keep secret storage in OS/user config locations or documented secure stores.
- Prefer explicit permission boundaries for tools that write files, use networks, send emails, or control browsers.

## Compliance & Policies
- Follow repository policy files, `AGENTS.md`, and harness governance when present.
- Telemetry changes must preserve user privacy and documented event semantics in `TELEMETRY.md`.

## Incident Response
If a secret is exposed, stop propagation, remove it from commits/logs where possible, rotate it externally, and document the remediation.
