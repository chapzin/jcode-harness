# Side Panel Preferences

Resolved defaults for this project:

- [x] Current goal
- [x] Active todos
- [x] Test commands
- [x] Failing checks
- [x] Open risks
- [x] Architecture notes
- [x] MCP status
- [x] Memory/wiki status when relevant

Default focus: `.jcode/side_panel/status.md`.

## Review triggers

Update the side panel when any of the following changes:

- A new implementation slice changes `jcode-harness` CLI, JSON, NDJSON, skills, smoke, `/init`, or clean-code behavior.
- A live provider, browser, MCP, telemetry, deployment, or database workflow is enabled.
- A release gate is added, removed, skipped, or marked non-mandatory.
- A generated context file is found stale or contradicted by repository source.
