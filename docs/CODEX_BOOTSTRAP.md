# Codex Bootstrap

Use this file to continue the embedded-skills harness work with Codex or another CLI agent.

## Repository state

- Work branch: `feature/embedded-skills-harness`.
- Upstream remote should point at `https://github.com/1jehuang/jcode.git`.
- Product scope: keep work aligned with the Jcode + local LLM wiki + `forrestchang/andrej-karpathy-skills` proposal. Prefer increments that make embedded skills, durable wiki memory, and harness automation cooperate without adding network/runtime marketplace dependencies.
- Built-in skills live in `src/skill_pack.rs` and are compiled with `include_str!`.
- Built-ins currently include `karpathy-guidelines`, `optimization`, `clean-code-guardian`, `llmwiki-memory`, `init-bootstrap`, and `sequential-thinking`.
- Vendored Karpathy files live under `third_party/andrej-karpathy-skills/`.
- `llmwiki-memory` lives under `.jcode/skills/llmwiki-memory/SKILL.md` and describes safe use of the local LLM wiki MCP tools for durable project memory, provenance, transcript sync, and secret boundaries.
- `init-bootstrap` and `sequential-thinking` live under `.jcode/skills/` and describe `/init`/scaffold upkeep plus bounded use of the local sequential-thinking MCP helper.

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

The harness should only need a configured provider/model to operate. It must not require network access at runtime to load embedded skills. Do not add dependencies on Claude Code, Cursor, Codex CLI, Node, plugin marketplaces, or remote MCP servers for skill loading.

LLM wiki usage is a memory/provenance layer, not a source-code truth layer. Agents should verify wiki claims against the repository and must not sync secrets, credentials, private keys, `.env` values, provider tokens, deployment secrets, or database credentials into wiki memory.

## Implementation guardrails

Keep diffs small, preserve upstream jcode behavior, do not remove existing providers or commands, and keep `jcode run`, `jcode serve`, and `jcode connect` compatible.
