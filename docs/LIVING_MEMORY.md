# Jcode Living Memory

Memory v2 introduces an optional `llm-wiki` backend based on Andrej Karpathy's LLM Wiki pattern.

## Status

This is implemented as a progressive backend, not a replacement for existing memory.

Backends:

- `legacy`: current graph/embedding/JSON memory. This remains the default.
- `wiki`: Markdown Living Memory only.
- `hybrid`: injects wiki context first and keeps legacy retrieval as fallback.
- `off`: disables memory injection/retrieval.

Configure with either `~/.jcode/config.toml`:

```toml
[memory]
backend = "legacy"      # legacy | wiki | hybrid | off
wiki_scope = "global-cache" # global-cache | repo-local
```

or environment variables:

```bash
JCODE_MEMORY_BACKEND=wiki
JCODE_MEMORY_BACKEND=hybrid
JCODE_MEMORY_BACKEND=off
JCODE_MEMORY_WIKI_SCOPE=repo-local
```

## Layout

Default global layout:

```text
~/.jcode/memory_wiki/
  schema.md
  index.md
  log.md
  overview.md
  raw/
    sessions/
    imports/
    artifacts/
    manual/
  wiki/
    user/
      preferences.md
      corrections.md
      procedures.md
      style.md
    projects/
    entities/
    concepts/
    sources/
  cache/
    search_index.json
    page_manifest.json
  trash/
```

Repo-local layout is opt-in and uses `.jcode/memory_wiki/`.

## CLI

```bash
jcode memory wiki init
jcode memory wiki status
jcode memory wiki doctor
jcode memory wiki search "preference"
jcode memory wiki schema
jcode memory wiki schema --full
```

## Safety rules

- Raw sources are intended to be append-only.
- Wiki pages are Markdown with YAML frontmatter.
- Important claims should cite raw sources.
- Do not store secrets, tokens, passwords, `.env` contents, private keys, credentials, or sensitive personal data.
- The wiki works offline and does not require embeddings.
