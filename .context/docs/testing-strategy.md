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
# Rust focused check
cargo check -p <crate>

# Rust focused tests
cargo test -p <crate> <filter>

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

## Troubleshooting
Stale binaries and socket timing are common causes of false failures. Rebuild with the self-dev profile and prefer scripts that emit progress/checkpoint lines for long tasks.
