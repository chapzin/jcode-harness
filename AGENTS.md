# Repository Guidelines

## Development Workflow

- **Commit as you go** - Make small, focused commits after completing each feature or fix.
- If the git state is not clean, or there are other agents working in the codebase in parallel, do your best to still commit only your work.
- **Push when done** - Push all commits to remote when finishing a task or session, unless the user or environment says not to.
- **Use fast iteration by default** - Prefer `cargo check`, targeted tests, and dev/selfdev builds while iterating.
- **Rebuild when done** - When you are done making source changes, build the affected binary or use the self-dev build flow.
- **Bump version for releases** - Update version in `Cargo.toml` when making releases. When cutting a new release, inspect all changes since the last release and choose patch/minor/etc.
- **Remote builds available** - Use `scripts/remote_build.sh` to offload heavy cargo work to another machine. If a build is terminated, check local resources and prefer remote build if needed.

## Repository Map

- `Cargo.toml` defines the root Rust 2024 workspace and the root `jcode` package.
- `src/lib.rs` is the root library entry point.
- `src/main.rs` builds the primary `jcode` CLI/TUI binary.
- `src/bin/harness.rs` builds the automation-facing `jcode-harness` binary.
- `src/project_init.rs` owns `/init` scaffolding and swarm bootstrap behavior.
- `src/skill.rs` and `src/skill_pack.rs` own embedded skills behavior and built-in skill registry.
- `src/server/` owns server, socket, and swarm/session orchestration behavior.
- `crates/` contains smaller Rust crates for shared types, provider/auth contracts, storage, TUI components, side-panel, mobile/desktop, tools, and update support.
- `docs/` contains product, architecture, release-gate, skills, clean-code, and bootstrap documentation.
- `scripts/` contains developer automation, CI helpers, quality budgets, profiling, release, and install scripts.
- `tests/` contains Rust e2e tests and Python black-box/debug-socket tests.
- `telemetry-worker/` contains Cloudflare Wrangler telemetry worker scripts and migrations.
- `.jcode/` contains project-local harness init plans, MCP plans, skills plans, and side-panel status.
- `.context/` contains generated AI context. Treat it as helpful but verify against `Cargo.toml` and the current tree before relying on it.

## Architecture Boundaries

- Keep stable DTOs and serde contracts in focused `jcode-*-types` crates when they are dependency-light.
- Keep runtime behavior needing storage, config, logging, server, providers, process spawning, Tokio tasks, or TUI state in the root crate unless a full domain boundary can move cleanly.
- Do not make type crates depend on root/runtime-heavy crates. Use `python3 scripts/check_dependency_boundaries.py` after changing type-crate dependencies.
- Prefer compatibility re-exports during crate-boundary migrations, then remove old paths only after downstream call sites are updated intentionally.
- See `docs/CRATE_OWNERSHIP_BOUNDARIES.md` before moving types or adding internal crate dependencies.

## Embedded Skills Harness Fork

- Built-in skills must remain usable without runtime network access, Node, Claude Code, Cursor, Codex CLI, or plugin marketplaces.
- Preserve vendored attribution under `third_party/andrej-karpathy-skills/` and `NOTICE.md` when updating `karpathy-guidelines`.
- Prefer `include_str!` and small registry changes over broad abstractions.
- Keep `clean-code-guardian` as an original operational synthesis. Do not vendor copyrighted Clean Code book text, examples, chapters, PDFs, or proprietary lists.
- Use `jcode clean-code check` or `jcode-harness clean-code check` for offline quality-gate validation when touching code.
- Keep `jcode run`, `jcode serve`, `jcode connect`, and existing providers compatible.
- Use `docs/SKILLS_HARNESS.md`, `docs/CLEAN_CODE_GUARDIAN.md`, `docs/CODEX_BOOTSTRAP.md`, `docs/JCODE_HARNESS_INIT_SWARM.md`, and `docs/JCODE_HARNESS_RELEASE_GATES.md` as operating docs for this fork.

## Validation Commands

Use the narrowest command that covers the touched code. Evidence-backed release/init candidates include:

```bash
cargo fmt --check
cargo check -p jcode
cargo test -p jcode project_init --lib -- --nocapture
cargo test -p jcode test_init_command --lib -- --nocapture
cargo test -p jcode skill::tests --lib
cargo test -p jcode clean_code --lib
cargo test --test e2e harness_cli -- --nocapture
cargo run -q -p jcode --bin jcode-harness -- skills list --json | python3 -m json.tool >/dev/null
cargo run -q -p jcode --bin jcode-harness -- skills doctor --json | python3 -m json.tool >/dev/null
```

For `jcode-harness` automation contracts, also validate JSON/NDJSON smoke paths when relevant:

```bash
cargo run -q -p jcode --bin jcode-harness -- run "review this diff" --json --mock-response ok | python3 -m json.tool >/dev/null
cargo run -q -p jcode --bin jcode-harness -- run "review this diff" --ndjson --mock-response ok | while read -r line; do printf '%s\n' "$line" | python3 -m json.tool >/dev/null; done
```

## Self-Development and Debugging

- Prefer coordinated builds with the Jcode `selfdev build` tool when available.
- Fallback build command: `scripts/dev_cargo.sh build --profile selfdev -p jcode --bin jcode`.
- For UI changes, use debug socket testers and frame/state inspection instead of relying only on screenshots.
- Logs are written to `~/.jcode/logs/` as daily files such as `jcode-YYYY-MM-DD.log`.
- If a long command is necessary, prefer scripts that emit progress/checkpoint output.

## Security and Secrets

- Never commit or persist tokens, API keys, cookies, private keys, `.env` values, local session files, provider credentials, deployment secrets, or database credentials.
- Redact credentials from logs, screenshots, telemetry, fixtures, generated docs, side-panel pages, and memory.
- MCP setup is review-first. `.jcode/mcp.json` currently has no active servers. Do not add network, GitHub, browser, database, or telemetry MCP servers without explicit scope and credential review.
- Treat provider/auth, telemetry, release, browser automation, and email/Gmail tooling as sensitive integration surfaces.
- Do not perform destructive or externally visible operations, such as deleting databases, sending email, deploying, or publishing releases, without explicit user confirmation.

## Install Notes

- `~/.local/bin/jcode` is the launcher symlink used from `PATH`.
- `~/.jcode/builds/current/jcode` is the active local/source-build channel; self-dev builds and `scripts/install_release.sh` point the launcher here.
- `~/.jcode/builds/stable/jcode` is the stable release channel; `scripts/install.sh` installs this and points the launcher here.
- `~/.jcode/builds/versions/<version>/jcode` stores immutable binaries.
- `~/.jcode/builds/canary/jcode` still exists for canary/testing flows, but it is not the primary self-dev install path.
- On Windows, the equivalents are `%LOCALAPPDATA%\\jcode\\bin\\jcode.exe` for the launcher, `%LOCALAPPDATA%\\jcode\\builds\\stable\\jcode.exe` for stable, and `%LOCALAPPDATA%\\jcode\\builds\\versions\\<version>\\jcode.exe` for immutable installs; `scripts/install.ps1` currently installs the stable channel.
- Ensure `~/.local/bin` is **before** `~/.cargo/bin` in `PATH`.

## AI Context References

- Init swarm synthesis: `.jcode/init/SWARM_ANALYSIS_REPORT.md`.
- Documentation index: `.context/docs/README.md`.
- Agent playbooks: `.context/agents/README.md`.
- Generated `.context` files can become stale. Verify claims against `Cargo.toml`, `src/`, `crates/`, and current docs before acting.
