# Documentation Index

Welcome to the repository knowledge base. Start with the project overview, then dive into specific guides as needed.

> Note: these `.context` files are generated AI context. They should stay project-specific, but always verify details against `Cargo.toml`, `src/`, `crates/`, and current docs before making code changes.

## Core Guides
- [Project Overview](./project-overview.md)
- [Architecture Notes](./architecture.md)
- [Development Workflow](./development-workflow.md)
- [Testing Strategy](./testing-strategy.md)
- [Glossary & Domain Concepts](./glossary.md)
- [Data Flow & Integrations](./data-flow.md)
- [Security & Compliance Notes](./security.md)
- [Tooling & Productivity Guide](./tooling.md)

## Repository Snapshot
- `AGENTS.md` - Repository-specific agent guidelines and validation/security policy.
- `Cargo.toml` - Root Rust 2024 workspace and root `jcode` package manifest.
- `Cargo.lock` - Cargo dependency lockfile.
- `src/` - Root Rust library, CLI/TUI binary, harness binary, server, skills, init, and runtime modules.
- `crates/` - Modular Rust crates for shared types, providers/auth, storage, TUI, side-panel, tools, update, mobile, and desktop support.
- `docs/` - Product, architecture, skills, clean-code, release-gate, bootstrap, and operating documentation.
- `scripts/` - Developer automation, CI helpers, budget checks, profiling, release, and install scripts.
- `tests/` - Rust e2e tests and Python black-box/debug-socket tests.
- `telemetry-worker/` - Cloudflare Wrangler telemetry worker and D1 migrations.
- `third_party/` - Vendored third-party content and attribution-sensitive assets.
- `.jcode/` - Project-local init reports, MCP plan, skills plan, and side-panel status.
- `.context/` - Generated AI context and agent playbooks.
- `assets/`, `packaging/`, `ios/`, `figma/`, `mockups/` - Product assets, packaging, mobile/design files.

## Important Project-Specific References
- Init swarm synthesis: `.jcode/init/SWARM_ANALYSIS_REPORT.md`
- Harness release gates: `docs/JCODE_HARNESS_RELEASE_GATES.md`
- Embedded skills: `docs/SKILLS_HARNESS.md`
- Clean-code gate: `docs/CLEAN_CODE_GUARDIAN.md`
- Bootstrap continuation: `docs/CODEX_BOOTSTRAP.md`
- Crate boundaries: `docs/CRATE_OWNERSHIP_BOUNDARIES.md`
- Server architecture: `docs/SERVER_ARCHITECTURE.md`
- Swarm architecture: `docs/SWARM_ARCHITECTURE.md`

## Document Map
| Guide | File | Primary Inputs |
| --- | --- | --- |
| Project Overview | `project-overview.md` | `Cargo.toml`, README, docs, init swarm findings |
| Architecture Notes | `architecture.md` | Crate boundaries, server/swarm docs, dependency structure |
| Development Workflow | `development-workflow.md` | `AGENTS.md`, scripts, CI config, selfdev flow |
| Testing Strategy | `testing-strategy.md` | Release gates, CI workflows, scripts, e2e tests |
| Glossary & Domain Concepts | `glossary.md` | Product docs and repo terminology |
| Data Flow & Integrations | `data-flow.md` | Provider/auth, MCP, telemetry, tools, storage |
| Security & Compliance Notes | `security.md` | Auth model, secrets management, MCP/release boundaries |
| Tooling & Productivity Guide | `tooling.md` | Cargo, selfdev, debug socket, scripts, side panel |
