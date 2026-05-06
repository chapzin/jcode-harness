# Codex Bootstrap

Use this file to continue the embedded-skills harness work with Codex or another CLI agent.

## Repository state

- Work branch: `feature/embedded-skills-harness`.
- Upstream remote should point at `https://github.com/1jehuang/jcode.git`.
- Built-in skills live in `src/skill_pack.rs` and are compiled with `include_str!`.
- Vendored Karpathy files live under `third_party/andrej-karpathy-skills/`.

## Recommended continuation flow

```bash
git status --short --branch
cargo fmt
cargo check -p jcode --bin jcode
cargo check -p jcode --bin jcode-harness
cargo test -p jcode skill --lib
cargo run -p jcode --bin jcode -- skills list
cargo run -p jcode --bin jcode-harness -- smoke
```

## Runtime assumptions

The harness should only need a configured provider/model to operate. It must not require network access at runtime to load embedded skills. Do not add dependencies on Claude Code, Cursor, Codex CLI, Node, or plugin marketplaces for skill loading.

## Implementation guardrails

Keep diffs small, preserve upstream jcode behavior, do not remove existing providers or commands, and keep `jcode run`, `jcode serve`, and `jcode connect` compatible.
