---
type: doc
name: development-workflow
description: Day-to-day engineering processes, branching, and contribution guidelines
category: workflow
generated: 2026-05-06
status: filled
scaffoldVersion: "2.0.0"
---

# Development Workflow

Prefer small, reviewable changes with focused verification. When changing Jcode itself, use the self-development build and reload loop rather than ad hoc release builds.

## Branching & Releases
- Work happens on feature branches such as `feature/embedded-skills-harness`.
- Commit logically scoped changes as you go.
- Follow `RELEASING.md` for release-specific packaging and tagging.

## Local Development
```bash
# Inspect workspace
cargo metadata --no-deps

# Focused crate check
cargo check -p <crate>

# Preferred self-dev binary build fallback
scripts/dev_cargo.sh build --profile selfdev -p jcode --bin jcode

# Python test example
python3 tests/test_selfdev_reload.py
```

## Code Review Expectations
Reviews should confirm crate boundaries, error handling, permission safety, test coverage, UI regressions, and documentation updates. For UI changes, validate with debug socket testers or snapshots when available.

## Onboarding Tasks
Start by reading `AGENTS.md`, `README.md`, `CONTRIBUTING.md`, this `.context/docs` set, and the crate nearest your change.
