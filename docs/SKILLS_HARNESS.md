# Skills Harness

jcode skills are markdown instruction packs stored as `SKILL.md` files with YAML frontmatter. They can guide coding, review, refactoring, debugging, optimization, or other repeatable workflows.

## Skill sources and priority

Skills are loaded deterministically. Later sources override earlier sources with the same `name`:

1. built-in skills embedded in the binary;
2. project compatibility skills from `./.claude/skills`;
3. global jcode skills from `~/.jcode/skills`;
4. project-local jcode skills from `./.jcode/skills`.

This means a project-local `.jcode/skills/<name>/SKILL.md` can override a built-in skill without changing the binary.

## Built-in skills

This fork embeds:

- `karpathy-guidelines`, vendored from `forrestchang/andrej-karpathy-skills`;
- `optimization`, from this repository's existing `.jcode/skills/optimization` skill.

The built-ins are compiled with `include_str!`, so runtime skill loading does not require internet access, Node, Claude Code, Cursor, Codex CLI, or a plugin marketplace.

## CLI

```bash
jcode skills list
jcode skills show karpathy-guidelines
jcode skills sync
jcode skills sync --force
jcode skills doctor

jcode-harness skills list
jcode-harness skills show karpathy-guidelines
jcode-harness skills sync
jcode-harness skills doctor
```

`sync` copies built-in skills to `~/.jcode/skills` and does not overwrite existing files unless `--force` is used.

`skills doctor` reports loaded skills, built-in availability, invalid frontmatter found while scanning standard paths, duplicate names across origins, and the final effective path for each loaded skill.

## Harness run

```bash
jcode-harness
jcode-harness smoke
jcode-harness run "fix this Rust bug" --provider openai-compatible --model gpt-4.1
jcode-harness run "optimize memory usage" --skills always --dry-run
jcode-harness run "review this diff" --skill karpathy-guidelines --max-turns 3 --json
```

`jcode-harness` with no subcommand starts the regular interactive jcode experience.

`jcode-harness run` uses the same provider initialization, tool registry, and `Agent` runtime as `jcode run`, while remaining script-friendly. It prepends selected skill context before starting the agent loop.

## Skill router

The router is intentionally simple and deterministic:

- coding, bug, test, refactor, review, implement, fix, PR, or diff tasks select `karpathy-guidelines`;
- performance, latency, memory, throughput, CPU, RAM, or efficiency tasks select `optimization`;
- explicit `--skill <name>` always includes that skill;
- `--skills off` disables automatic routing while preserving explicit `--skill` values;
- `--skills always` includes built-in coding and optimization skills.

The router does not inject every skill by default.
