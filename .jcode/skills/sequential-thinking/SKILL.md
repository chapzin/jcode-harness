---
name: sequential-thinking
description: Guidelines for using the sequential thinking MCP tool on complex planning, debugging, architecture, verification, or multi-step analysis tasks that may require revision or branching.
allowed-tools: mcp__sequential-thinking__sequentialthinking
---

# Sequential Thinking

Use this skill when the task is complex enough to benefit from explicit multi-step analysis, hypothesis revision, branching, or a verifiable plan before implementation.

## Principles

1. **Use it for complexity, not ceremony**
   - Reach for sequential thinking when there is ambiguity, multi-file impact, architecture tradeoff, difficult debugging, or a need to revise hypotheses.
   - For trivial edits, avoid unnecessary reasoning overhead.

2. **Keep reasoning bounded and goal-driven**
   - Start with a small estimate of thoughts and adjust only when new evidence justifies it.
   - Convert the analysis into a concise plan with verifiable success criteria.
   - Stop once the next implementation step is clear.

3. **Revise openly, summarize safely**
   - Use revision or branching when evidence contradicts an earlier assumption.
   - Share concise conclusions, assumptions, tradeoffs, and verification evidence with the user.
   - Do not expose private chain-of-thought transcripts; provide useful summaries instead.

4. **Pair thinking with evidence**
   - Inspect source files, tests, docs, traces, or runtime output before locking in a plan.
   - After implementation, run focused validation and compare the result against the original success criteria.

5. **Respect tool and safety boundaries**
   - Sequential thinking is for analysis. It does not grant permission for destructive actions, credential use, network access, payments, deployment, or external side effects.

## Recommended workflow

```text
1. Identify the concrete goal and ambiguity.
2. Run a short sequential-thinking pass to choose the approach.
3. Translate conclusions into todos and validation commands.
4. Implement the smallest maintainable change.
5. Verify, then summarize decisions and evidence.
```
