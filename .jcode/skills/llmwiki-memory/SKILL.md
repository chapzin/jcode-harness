---
name: llmwiki-memory
description: Guidelines for using the local LLM wiki as durable project memory. Use when researching prior decisions, synchronizing session transcripts, preserving provenance, or turning repo/session knowledge into reusable context.
allowed-tools: mcp__llmwiki__wiki_query, mcp__llmwiki__wiki_search, mcp__llmwiki__wiki_read_page, mcp__llmwiki__wiki_sync, mcp__llmwiki__wiki_export, mcp__llmwiki__wiki_lint
---

# LLM Wiki Memory

Use this skill when the task depends on project memory, prior decisions, session transcripts, durable context, or wiki-backed research.

## Principles

1. **Query before assuming**
   - Search the wiki for prior decisions before inventing new policy.
   - Prefer `wiki_query` for synthesized answers and `wiki_search` when exact wording matters.

2. **Preserve provenance**
   - When a decision comes from the wiki, cite the source page or raw session path.
   - Distinguish verified repository facts from remembered context and hypotheses.

3. **Sync deliberately**
   - Use `wiki_sync` when recent sessions should become durable memory.
   - Do not sync secrets, credentials, private keys, `.env` values, or provider tokens into wiki memory.

4. **Keep memory operational**
   - Convert repeated decisions into concise project notes, checklists, or skill guidance.
   - Keep generated wiki/context content short enough to be useful to future agents.

5. **Lint and export before relying on broad context**
   - Use `wiki_lint` to detect broken links, stale summaries, orphan pages, and contradictions.
   - Use `wiki_export` only when a compact or complete wiki snapshot is needed for handoff.

## Recommended workflow

```text
1. Ask: does this task depend on previous project decisions or session history?
2. If yes, query/search the LLM wiki.
3. Read specific pages only when needed.
4. Apply Karpathy-style surgical changes using the retrieved facts.
5. Record any new durable decision in project docs or wiki memory with provenance.
```

## Boundaries

- The LLM wiki is memory, not ground truth. Verify code claims against the repository.
- Do not store secrets or sensitive local environment values.
- Do not let stale wiki content override current source files, tests, or explicit user instructions.
