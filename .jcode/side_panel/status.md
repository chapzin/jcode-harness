# Project Status Panel

## Current goal

Post-init hardening completed for `/home/chapzin/jcode-harness`: onboarding/context docs corrected and validation suite passed.

## Swarm status

- architect: reported, completed
- qa: reported, completed
- documenter: reported, completed
- tooling-security: reported, ready
- Barrier: all required discovery reports received before synthesis
- Synthesis: `.jcode/init/SWARM_ANALYSIS_REPORT.md`

## Documentation updates completed

- Expanded `AGENTS.md` with repository map, validation commands, architecture boundaries, self-dev/debug guidance, and security/MCP policy.
- Corrected stale `.context` references to TypeScript and `crates/jcode` as the main binary.
- Updated `.context/docs/README.md`, `project-overview.md`, `architecture.md`, and `testing-strategy.md`.

## Validation passed

Last run: 2026-05-06T21:14Z

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

Observed warning only: deprecated `ProviderChoice::ClaudeSubprocess` in `src/cli/provider_init_tests.rs`.

## Remaining recommended work

- Decide whether to make `python3 scripts/test_ci_suites.py lib-bins` or `cargo test --lib --bins` a CI gate.
- Add telemetry-worker test/lint scripts if telemetry deployment should be release-gated.
- Review release/install script signature verification and Homebrew SSH host-key behavior before release hardening.
- Keep MCP disabled until explicit credential and scope review.
