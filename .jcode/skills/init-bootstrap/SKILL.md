---
name: init-bootstrap
description: Guidelines for running and reviewing jcode-harness project initialization. Use when /init, context scaffolding, swarm init analysis, MCP plans, skills plans, side panels, or onboarding bootstrap files are involved.
allowed-tools: mcp__ai-context__context, mcp__ai-context__workflow-init, todo, swarm, side_panel
---

# Init Bootstrap

Use this skill when a task involves project initialization, `/init`, `.context` scaffolding, `.jcode` bootstrap files, side-panel status, MCP planning, or first-pass repository onboarding.

## Principles

1. **Bootstrap before synthesis**
   - Check whether `.context` and `.jcode` scaffolding already exist before writing new bootstrap content.
   - Treat deterministic scaffold files as phase 0. Use them as inputs for later repository analysis rather than as final truth.

2. **Keep `/init` swarm-aware**
   - Preserve the split between deterministic scaffold generation and the queued LLM/swarm analysis turn.
   - Required discovery roles are architecture, QA/testing, documentation/onboarding, and tooling/MCP/security.
   - Do not synthesize final recommendations until discovery agents have reported or are explicitly marked blocked.

3. **Review MCP before enabling it**
   - Do not download, install, or enable MCP servers as part of init without explicit permission and documented scopes.
   - Prefer local/offline capabilities first. Keep network, credential, browser, database, telemetry, and deployment surfaces review-gated.

4. **Update the durable bootstrap files together**
   - Keep `AGENTS.md`, `.jcode/SKILLS_PLAN.md`, `.jcode/MCP_PLAN.md`, `.jcode/init/SWARM_ANALYSIS_PLAN.md`, `.jcode/init/SWARM_ANALYSIS_REPORT.md`, and side-panel status consistent when init guidance changes.
   - Mark unknowns and human questions honestly instead of inventing repository facts.

5. **Verify bootstrap changes**
   - For code changes, prefer `cargo test -p jcode project_init --lib` and relevant harness init e2e tests.
   - For docs/scaffold updates, verify referenced paths exist and commands are supported by repository evidence.

## Recommended workflow

```text
1. Check existing `.context`/`.jcode` state and current git status.
2. If scaffolding is missing, initialize it with the local context/init path.
3. Run or preserve the `/init` swarm analysis plan with a barrier before synthesis.
4. Update skills/MCP/side-panel docs as a single coherent bootstrap slice.
5. Run focused validation and record any remaining blockers.
```

## Boundaries

- Do not persist secrets, provider tokens, cookies, `.env` values, private keys, or deployment credentials.
- Do not enable MCP servers automatically.
- Do not treat generated context as more authoritative than current source files, tests, and explicit user instructions.
