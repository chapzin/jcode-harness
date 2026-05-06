---
type: doc
name: data-flow
description: How data moves through the system and external integrations
category: data-flow
generated: 2026-05-06
status: filled
scaffoldVersion: "2.0.0"
---

# Data Flow & Integrations

Jcode data flows from user input and scheduled/background events into session state, model/tool execution, streaming UI updates, persisted history/memory, and optional telemetry/debug channels.

## Module Dependencies
- **UI/application crates** depend on core utilities, config, auth, runtime contracts, and tool type crates.
- **Tool runtimes** depend on shared type crates and external integration clients.
- **Scripts/tests** depend on the built binaries and debug socket protocol.
- **Telemetry worker** accepts normalized event/session/turn payloads and stores them dynamically.

## Service Layer
- Session and agent runtime services live across `src/` and `crates/jcode-agent-runtime`.
- Config/auth services live in `crates/jcode-config-types`, `crates/jcode-auth-types`, and auth implementation crates.
- Background, batch, gateway, and MCP contracts live in corresponding `crates/jcode-*` crates.

## High-level Flow
1. User, schedule, swarm, or debug socket input enters the app.
2. The runtime resolves configuration, permissions, tools, and model/provider context.
3. Agent turns emit messages, tool calls, traces, and UI events.
4. State is rendered in terminal/desktop surfaces and persisted to local history or memory.
5. Tests and telemetry observe behavior through sockets, logs, or worker endpoints.

## Internal Movement
The repository favors typed in-process boundaries for Rust code and process/socket boundaries for integration testing. Long-running validation uses scripts that emit structured progress lines.

## External Integrations
- Model providers and auth/OAuth flows for agent execution.
- MCP servers and browser/Gmail/search tools for external actions.
- Telemetry worker for event/session/turn reporting.

## Observability & Failure Modes
Use debug socket inspection, script logs, benchmark outputs, and telemetry records. Common failures include stale binaries, provider auth issues, socket timeouts, and flaky long-running UI interactions.
