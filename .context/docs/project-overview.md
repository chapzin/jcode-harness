---
type: doc
name: project-overview
description: High-level overview of the project, its purpose, and key components
category: overview
generated: 2026-05-06
status: filled
scaffoldVersion: "2.0.0"
---

# Project Overview

Jcode is an open source agentic coding environment focused on terminal-first development, multi-agent coordination, tool use, memory, self-development, and desktop/mobile companion experiences. The repository contains the Rust TUI and supporting crates, automation scripts, integration tests, telemetry ingestion, packaging, and design assets.

> **Detailed Analysis**: For complete generated symbol counts and dependency graphs, see `codebase-map.json` when present.

## Quick Facts
- Root: `/home/chapzin/jcode-harness`
- Primary languages: Rust for the application and crates, Python for test/benchmark automation, JavaScript for telemetry and Figma tooling.
- Main binaries/crates: `crates/jcode`, `crates/jcode-desktop`, MCP/tooling crates, gateway/config/auth/background type crates.
- Current branch: `feature/embedded-skills-harness`.

## Entry Points
- `src/main.rs` and `crates/jcode/src/main.rs` for the TUI/CLI application.
- `crates/jcode-desktop/src/main.rs` for the desktop wrapper.
- `telemetry-worker/src/worker.js` for telemetry ingestion.
- `scripts/` for developer automation and profiling.

## Key Exports
The generated semantic scan found many exported Python automation helpers and Rust crate APIs. Use `cargo metadata`, `cargo doc --workspace --no-deps`, and targeted `agentgrep` searches for complete crate-level APIs.

## File Structure & Code Organization
- `crates/` - Modular Rust crates for runtime, tool types, config, auth, desktop, MCP support, memory, patching, and UI capabilities.
- `src/` - Application modules for the main Jcode TUI.
- `scripts/` - Benchmarks, debug socket tests, CI checks, profiling, and release helpers.
- `tests/` - Integration and regression tests that drive Jcode through sockets and subprocesses.
- `telemetry-worker/` - Event/session/turn ingestion service.
- `packaging/`, `ios/`, `assets/` - release and product artifacts.

## Technology Stack Summary
Jcode is primarily a Rust workspace using Cargo. Python scripts provide automation and black-box tests. JavaScript is used for the telemetry worker and Figma plugin. Developer workflows rely on self-dev builds, debug sockets, and focused Cargo/Python checks.

## Getting Started Checklist
1. Review `AGENTS.md`, `README.md`, and `CONTRIBUTING.md`.
2. Inspect crates with `cargo metadata --no-deps` or `cargo check -p <crate>`.
3. Use `scripts/dev_cargo.sh build --profile selfdev -p jcode --bin jcode` or the `selfdev` tool for local self-dev builds.
4. Run focused tests near the touched crate or script.
5. For UI changes, validate with debug socket tester sessions.
