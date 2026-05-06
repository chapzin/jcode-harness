---
type: doc
name: tooling
description: Scripts, IDE settings, automation, and developer productivity tips
category: tooling
generated: 2026-05-06
status: filled
scaffoldVersion: "2.0.0"
---

# Tooling & Productivity Guide

## Required Tooling
- Rust toolchain and Cargo for workspace builds.
- Python 3 for tests, benchmarks, profiling, and CI helper scripts.
- Node/JavaScript tooling when editing `telemetry-worker` or Figma plugin code.
- Jcode `selfdev` and `debug_socket` tools for self-development and UI validation.

## Recommended Automation
- Use `selfdev build` for coordinated self-dev builds.
- Use `scripts/dev_cargo.sh` as the fallback cargo wrapper.
- Use `agentgrep` for source discovery and `cargo metadata` for crate topology.
- Use budget scripts before expanding panic/error/code-size surfaces.

## IDE / Editor Setup
Enable Rust Analyzer, Python linting, and format-on-save where practical. Keep generated artifacts and `target/` out of broad searches.

## Productivity Tips
Prefer focused crate checks over full workspace builds. For long-running tests, run them in the background with progress output. Use debug socket commands to inspect live state instead of relying only on screenshots.
