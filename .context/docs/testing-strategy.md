---
type: doc
name: testing-strategy
description: Test frameworks, patterns, coverage requirements, and quality gates
category: testing
generated: 2026-05-06
status: filled
scaffoldVersion: "2.0.0"
---

# Testing Strategy

Quality is maintained with focused Rust tests, Python black-box integration tests, budget scripts, benchmark scripts, and manual/debug-socket validation for interactive UI behavior.

## Test Types
- **Rust unit/integration**: Cargo tests near crates and modules. Use focused `cargo test -p <crate> <filter>`.
- **Python integration**: `tests/test_*.py` and `scripts/test_*.py` drive binaries, sockets, reloads, swarm, and injection behavior.
- **JavaScript worker tests**: validate `telemetry-worker` behavior when touching telemetry ingestion.
- **Benchmarks/budgets**: `scripts/bench_*.py`, `scripts/check_*_budget.py`, memory and startup profilers.

## Running Tests
```bash
# Formatting and focused Rust check
cargo fmt --check
cargo check -p jcode

# /init swarm bootstrap coverage
cargo test -p jcode project_init --lib -- --nocapture
cargo test -p jcode test_init_command --lib -- --nocapture

# Embedded skills and clean-code gates
cargo test -p jcode skill::tests --lib
cargo test -p jcode clean_code --lib

# Harness CLI e2e coverage
cargo test --test e2e harness_cli -- --nocapture

# Harness JSON smoke checks
cargo run -q -p jcode --bin jcode-harness -- skills list --json | python3 -m json.tool >/dev/null
cargo run -q -p jcode --bin jcode-harness -- skills doctor --json | python3 -m json.tool >/dev/null

# Self-dev reload regression
python3 tests/test_selfdev_reload.py

# Budget examples
python3 scripts/check_panic_budget.py
python3 scripts/check_swallowed_error_budget.py
```

## Quality Gates
- Run the narrowest reliable check that covers the changed code.
- For self-dev changes, build through `selfdev build` when available.
- For UI changes, use debug socket tester sessions and inspect rendered output.
- Do not claim completion without recording skipped or failing verification.
- Current swarm analysis noted that CI covers many checks but may compile more lib/bin tests than it runs. A local default serial run of `python3 scripts/test_ci_suites.py lib-bins` on 2026-05-07 hit the 600s Jcode background supervision limit while tests were still passing, so keep it as a future split/optimization issue rather than a mandatory gate.
- `telemetry-worker` currently has Wrangler dev/deploy/migration scripts but no package-local test/lint script in `package.json`.

## Troubleshooting
Stale binaries and socket timing are common causes of false failures. Rebuild with the self-dev profile and prefer scripts that emit progress/checkpoint lines for long tasks.
