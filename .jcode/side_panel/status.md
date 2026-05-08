# Project Status Panel

## Current goal

Continue the `feature/embedded-skills-harness` branch with current `/init` context, release gates, and side-panel scaffolds aligned to the implemented `jcode-harness` surfaces.

## Current branch state

- Branch: `feature/embedded-skills-harness`.
- Last pushed implementation/doc slices before this context refresh:
  - `78987bbf` Fix selfdev reload repo discovery.
  - `30456878` Add opt-in live provider smoke.
  - `b8f81d27` Add CI-friendly harness smoke e2e.
  - `cd4ec520` Document init and clean-code JSON schemas.
  - `db46a4dc` Sync release gates and product plan.
- Offline `jcode-harness init --yes --no-memory-wiki --json` reported no files written and detected Rust.

## Stable harness surfaces

- Offline embedded skills and deterministic source precedence.
- `jcode-harness skills` list/show/sync/doctor/match/llmwiki-bridge JSON surfaces.
- `jcode-harness run` JSON, NDJSON, dry-run, mock-response, and opt-in live-provider smoke coverage.
- `jcode-harness smoke` deterministic offline tool cases.
- `/init` static scaffolding plus queued swarm bootstrap contract.
- `clean-code check/rules` offline quality gate with documented JSON output.

## Validation snapshot to prefer

```bash
cargo fmt --check
cargo check -p jcode
cargo test -p jcode project_init --lib -- --nocapture
cargo test -p jcode test_init_command --lib -- --nocapture
cargo test -p jcode skill::tests --lib
cargo test -p jcode clean_code --lib
cargo test --test e2e harness_cli -- --nocapture
cargo test --test e2e harness_init_json -- --nocapture
cargo test --test e2e harness_smoke -- --nocapture
cargo test --test e2e clean_code_check_json -- --nocapture
cargo test --test e2e harness_live_provider -- --nocapture
```

For runtime changes, finish with `selfdev build` and reload when appropriate. For docs/context-only changes, use `git diff --check`, targeted grep/schema checks, and JSON parsing of `.codex-harness/**/*.json` when governance files changed.

## Current risks

- Live provider and live `/init` swarm smokes must remain opt-in until credentials, quota, isolation, and UI/provider automation are reviewed.
- MCP remains disabled by default. Do not add network or credentialed servers without explicit scope review.
- Generated `.context/` is advisory and must be verified against `Cargo.toml`, `src/`, `crates/`, and current docs.
- Telemetry/deployment workflows involve external services and secrets; do not deploy or migrate databases without explicit confirmation.

## Next recommended slices

1. Keep schema docs, release gates, release-note template, and status snapshots aligned with any new stable harness fields.
2. Expand Clean Code Guardian fixtures only alongside documented severity-policy changes.
3. Keep `python3 scripts/test_ci_suites.py lib-bins` as a future split/optimization issue: local default serial measurement hit the 600s supervisor limit while tests were still passing, so it is not ready as a mandatory gate.
4. Add telemetry-worker test/lint scripts before treating telemetry deploy workflows as fully gated.
