# Project Status Panel

## Current goal

Scoped autonomous increment completed for proposal: Jcode + local LLM wiki + `forrestchang/andrej-karpathy-skills`.

## Completed this slice

- Added built-in `llmwiki-memory` skill at `.jcode/skills/llmwiki-memory/SKILL.md`.
- Embedded it through `src/skill_pack.rs` with `include_str!`, preserving offline skill loading.
- Updated `src/skill_router.rs` to route LLM wiki/project-memory/provenance/transcript/context-history tasks to `llmwiki-memory`.
- Fixed a router false positive where the old `"pr"` trigger matched inside words like `prior`/`project`.
- Fixed the deprecated-provider test warning with a scoped `#[allow(deprecated)]` only on the legacy compatibility assertion.
- Updated `/init` Skills Plan generation so bootstrap includes `llmwiki-memory` and wiki secret-safety notes.
- Updated bootstrap/docs/release gates/status/README to keep future work inside the Jcode + LLM wiki + Karpathy skills scope.

## Validation passed

```bash
cargo fmt --check
cargo test -p jcode test_provider_choice_arg_values --lib -- --nocapture
cargo test -p jcode skill_router --lib
cargo test -p jcode skill::tests --lib
cargo test -p jcode project_init --lib -- --nocapture
cargo test --test e2e harness_cli -- --nocapture
cargo run -q -p jcode --bin jcode-harness -- skills show llmwiki-memory --json | python3 -m json.tool >/dev/null
cargo run -q -p jcode --bin jcode-harness -- skills doctor --json | python3 -m json.tool >/dev/null
selfdev build target=auto
```

Selfdev build passed. `selfdev reload` was attempted but the reload tool returned `Could not find jcode repository directory`; `selfdev status` still sees the build channel and recent build.

## Next recommended slices

1. Add permission-reviewed bridge points from `llmwiki-memory` to concrete wiki commands without making remote MCP/network dependencies mandatory.
2. Add JSON schema docs/examples for `llmwiki-memory` in `docs/JCODE_HARNESS_JSON_SCHEMAS.md`.
3. Add release-note template for upstream divergence and harness-specific behavior.
