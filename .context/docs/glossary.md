---
type: doc
name: glossary
description: Project terminology, type definitions, domain entities, and business rules
category: glossary
generated: 2026-05-06
status: filled
scaffoldVersion: "2.0.0"
---

# Glossary & Domain Concepts

## Type Definitions
Most exported Rust types live in `crates/jcode-*types` crates or `*/src/lib.rs`. Use `cargo doc --workspace --no-deps` for canonical signatures.

## Enumerations
Enums are distributed across Rust crates for tool statuses, config, background jobs, auth state, and UI/session state. Search with `agentgrep` for `enum <Name>`.

## Core Terms
- **Jcode**: The open source agentic coding environment.
- **Harness**: Local orchestration context for tools, governance, contracts, traces, and verification.
- **Self-dev**: Workflow for building and reloading Jcode while running inside Jcode.
- **Debug socket**: Control/inspection channel used by tests and visual debugging.
- **Swarm**: Multi-agent coordination layer for task assignment and reporting.
- **MCP**: Model Context Protocol integrations exposed as tools.
- **PREVC**: Plan, Review, Execute, Verify, Complete workflow convention used by ai-context tooling.

## Acronyms & Abbreviations
- **TUI**: Terminal user interface.
- **MCP**: Model Context Protocol.
- **CI**: Continuous integration.
- **OAuth**: Authorization protocol used for provider sign-in flows.

## Personas / Actors
- Developer using Jcode interactively.
- Agent implementing, testing, or reviewing changes.
- Maintainer managing releases, packaging, and quality gates.
