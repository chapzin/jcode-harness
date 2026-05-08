# Jcode Harness Init Questions

These project-local answers customize the harness for this repository. Update this file whenever release gates, credential boundaries, or side-panel defaults change.

## Project

1. What is the main goal of this project?
   - Maintain and evolve Jcode as an open source agentic coding environment while this branch advances the standalone `jcode-harness` product direction: offline embedded skills, deterministic `/init` scaffolding, scriptable `run`/`smoke`/`skills`/`clean-code` commands, and release-grade JSON contracts.
2. What commands must pass before work is considered done?
   - Use the narrowest reliable command set for the touched surface.
   - For normal Rust/runtime changes: `cargo fmt --check`, focused `cargo test ...`, `cargo check -p jcode`, and `selfdev build` when changing the active Jcode binary.
   - For harness CLI changes: relevant commands from `docs/JCODE_HARNESS_RELEASE_GATES.md`, especially `cargo test --test e2e harness_cli -- --nocapture` plus focused e2e filters such as `harness_init_json`, `harness_smoke`, `harness_live_provider` default-skip, and `clean_code_check_json`.
   - For docs/context-only changes: `git diff --check`, relevant grep/schema checks, and JSON parsing for `.codex-harness/**/*.json` when governance files changed.
3. What files/directories are forbidden to edit?
   - Never edit `.git/`, `target/`, local build caches, local credentials, provider config files, session stores, database files, deployment state, or secrets.
   - Do not vendor copyrighted Clean Code book text, proprietary examples, PDFs, or chapter material.
   - Treat generated `.context/` as advisory context. It can be corrected when stale, but claims must be verified against `Cargo.toml`, `src/`, `crates/`, and current docs.
4. What data is sensitive and must never enter memory?
   - Tokens, API keys, cookies, OAuth/session files, provider credentials, `.env` values, private keys, deployment secrets, database credentials, telemetry secrets, browser profiles, email contents, and private customer/user data.

## Side panel/status

1. What should always appear in the side panel?
   - Current goal and active slice.
   - Todo/progress status.
   - Validation commands and latest result.
   - Open risks, especially provider/auth/network, MCP, release, telemetry, and swarm lifecycle boundaries.
   - Architecture notes for touched areas.
   - MCP and memory/wiki status when relevant.
2. Should side panel pages be linked files under `.jcode/side_panel/`?
   - Yes. Keep status and question pages as reviewable markdown files under `.jcode/side_panel/`.
3. Which page should be focused by default?
   - `.jcode/side_panel/status.md`.

## MCP

1. Which external systems should MCP access?
   - None by default. Native Jcode tools cover normal repository reads, search, edits, shell validation, and self-development.
   - Candidate MCP categories require explicit review: browser/Playwright for UI QA, GitHub/GitLab for issues and releases, docs/search for network research, and database/telemetry for diagnostics.
2. Which MCP servers require credentials?
   - GitHub/GitLab, provider-backed docs/search, browser profiles with logged-in state, database/telemetry, email/Gmail, deployment, and any remote network service.
3. Which MCP servers are allowed in CI?
   - None by default. CI-safe MCP usage must be local/offline, deterministic, credential-free, and separately documented.
4. Should network MCP servers be disabled by default?
   - Yes. Enable only after scope, credential, quota, and output-capture review.

## Skills

1. Which built-in skills should be active by default?
   - Skills should be task-routed, not globally injected. Prefer `rust`, `karpathy-guidelines`, `optimization`, `clean-code-guardian`, and `llmwiki-memory` when their triggers match the task.
2. Which project-specific skills should be added?
   - Keep embedded skills offline and minimal. Add project-specific skills only when they encode repeatable repository workflows or release gates that cannot be captured clearly in `AGENTS.md` or docs.
