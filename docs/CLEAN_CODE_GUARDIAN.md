# Clean Code Guardian

Clean Code Guardian is jcode's offline-first quality policy for implementation, review, refactoring, debugging, and generated code. It uses an original operational synthesis of clean-code engineering practices. It does not vendor or reproduce copyrighted book content.

## What it provides

- Built-in skill: `clean-code-guardian`.
- Built-in rule pack: `.jcode/quality/clean-code-rules.yaml`.
- Static quality gate that runs without an LLM.
- Human and JSON reports for local development and CI.
- Agent routing so coding, review, bugfix, test, refactor, PR, and diff tasks automatically receive the skill.

## Commands

```bash
jcode clean-code check
jcode clean-code check src tests --fail-on warning
jcode clean-code check --json
jcode clean-code rules

jcode-harness clean-code check --cwd /path/to/project
jcode-harness clean-code check --json --fail-on error
jcode-harness clean-code rules
```

`--fail-on` accepts `info`, `warning`, or `error`. The default is `error`.

## Current offline checks

The first gate is intentionally simple and deterministic:

- `small-focused-functions`: warns above 40 function-like lines and errors above 80.
- `manageable-file-size`: warns above 500 lines and errors above 1000.
- `no-silent-error-swallowing`: errors on common ignored-error patterns like `except: pass`, empty `catch`, `Err(_) => {}`, `let _ =`, and `.ok();`.
- `readable-names`: currently emits info for very long lines as a readability smell.

The YAML also includes checklist and LLM-oriented rules for future enriched review, but the basic gate does not require network access or a model.

## Agent behavior

For coding-related goals, the skill router now selects:

- `karpathy-guidelines` for disciplined engineering flow;
- `clean-code-guardian` for readability, focused changes, explicit error handling, and validation;
- `optimization` only for performance-oriented goals.

The agent should use Clean Code as a compass, not dogma. Project constraints, compatibility, security, and performance can justify trade-offs, but the trade-off should be explained.

## Copyright boundary

Do not copy chapters, examples, proprietary lists, PDFs, or extended excerpts from any copyrighted Clean Code source into this repository. Keep project content as original operational guidance. Users may point to private notes externally if they own them, but those notes should not be vendored here.

## CI example

```bash
jcode-harness clean-code check --fail-on error --json > clean-code-report.json
```
