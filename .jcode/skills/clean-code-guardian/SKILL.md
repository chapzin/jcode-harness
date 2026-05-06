---
name: clean-code-guardian
description: Apply an original, operational Clean Code inspired quality policy while writing, reviewing, refactoring, and fixing code.
allowed-tools: read, grep, glob, bash, write, edit, multiedit, patch, apply_patch, todo
---

# Clean Code Guardian

Use this skill when implementing, reviewing, refactoring, testing, debugging, or generating code. It is an original operational synthesis inspired by widely known clean-code engineering practices. It is not a copy of any book.

## Operating principles

- Prefer code that is easy for the next maintainer to read, test, and safely change.
- Use intention-revealing names for modules, types, functions, variables, tests, and errors.
- Keep functions and methods small enough to understand at a glance, with one clear purpose.
- Keep each changed unit focused on one responsibility.
- Reduce duplication when it is real and local, but avoid speculative abstractions.
- Prefer straightforward control flow over cleverness.
- Treat errors explicitly. Do not silently swallow failures.
- Prefer self-explanatory code over comments that explain confusing code.
- Keep comments for intent, constraints, invariants, public contracts, and surprising trade-offs.
- Keep tests clear, behavior-focused, and close to the changed behavior.
- Limit side effects and make dependencies visible.
- Preserve the existing project style unless there is a clear reason to improve touched code.
- Improve only the code you touch unless the task explicitly authorizes broader refactoring.
- Run relevant checks and report remaining risk.

## Review checklist

Before declaring completion, check:

1. Names reveal purpose and avoid unnecessary abbreviation.
2. Changed functions have one primary reason to change.
3. New branches and error paths are covered or explicitly justified.
4. No error is ignored without a documented reason.
5. No broad rewrite was done for a narrow task.
6. Tests or validation match the behavior changed.
7. Public behavior, compatibility, performance, and security trade-offs are noted.

## Boundaries

Clean Code is a compass, not dogma. Existing constraints, performance requirements, security requirements, compatibility, and project conventions may justify trade-offs. Explain controversial decisions instead of hiding them.
