---
type: doc
name: architecture
description: System architecture, layers, patterns, and design decisions
category: architecture
generated: 2026-05-06
status: filled
scaffoldVersion: "2.0.0"
---

# Architecture Notes

Jcode is a modular Rust workspace centered on a terminal UI agent runtime. It is split into small crates for domain types, tool contracts, auth, background work, MCP/gateway integrations, desktop support, and shared core utilities. Python scripts and integration tests exercise the running app through sockets and subprocess boundaries.

## System Architecture Overview
The main user path starts in the TUI/CLI binary, loads configuration and auth state, constructs sessions and tool runtimes, streams model/tool events into UI state, persists history/memory, and exposes debug/self-dev control surfaces. Desktop and mobile/design artifacts sit beside the terminal core rather than replacing it.

## Architectural Layers
- **Application UI**: root `src/` renders and manages the CLI/TUI; `crates/jcode-desktop/` provides desktop wrapper support.
- **Agent/runtime contracts**: `crates/jcode-agent-runtime`, `jcode-*types` define portable data structures and behavior boundaries.
- **Integrations**: auth, MCP, gateway, web/search, browser, Gmail, and telemetry-related crates/scripts.
- **Automation and verification**: `scripts/` and `tests/` drive benchmarks, reloads, socket checks, and CI budgets.
- **Product artifacts**: `assets/`, `figma/`, `ios/`, and `packaging/`.

## Detected Design Patterns
| Pattern | Confidence | Locations | Description |
| --- | --- | --- | --- |
| Workspace modularity | High | `crates/*` | Domain crates keep contracts and implementations isolated. |
| Type-contract crates | High | `crates/jcode-*types` | Shared types reduce coupling across runtime, tools, and UI. |
| Socket-driven test harness | High | `scripts/test_*.py`, `tests/` | Integration tests control live sessions through debug commands. |
| Self-development loop | High | `selfdev` tool, `scripts/dev_cargo.sh` | Coordinated build/reload workflow for changing Jcode itself. |
| Budget checks | Medium | `scripts/check_*_budget.py` | Static policy checks guard code size, panic usage, and swallowed errors. |

## Entry Points
- `src/main.rs` for terminal execution and `src/bin/harness.rs` for the `jcode-harness` CLI.
- `crates/jcode-desktop/src/main.rs` for desktop execution.
- `telemetry-worker/src/worker.js` for telemetry ingestion.
- `scripts/test_reload.py`, `scripts/test_swarm.py`, and related scripts for black-box validation.

## Public API
| Symbol Area | Type | Location |
| --- | --- | --- |
| Rust workspace crates | Crate APIs | `crates/*/src/lib.rs` |
| Debug/test helpers | Python functions/classes | `scripts/`, `tests/` |
| Telemetry ingest functions | JavaScript functions | `telemetry-worker/src/worker.js` |

## Internal System Boundaries
Keep reusable contracts in type crates, UI state in application crates, and process/socket orchestration in scripts/tests. Avoid introducing circular dependencies between core crates and application-specific crates.

## External Service Dependencies
Jcode integrates with model providers, OAuth/auth services, MCP servers, browser/Gmail tooling, and optional telemetry infrastructure. Treat credentials and tokens as sensitive local configuration.

## Top Directories Snapshot
- `crates/` - largest Rust workspace area.
- `src/` - TUI application source.
- `scripts/` - automation and profiling.
- `tests/` - integration tests.
- `telemetry-worker/` - JavaScript telemetry endpoint.

## Related Resources
- `project-overview.md`
- `data-flow.md`
- `testing-strategy.md`
