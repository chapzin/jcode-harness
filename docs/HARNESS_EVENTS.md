# Harness Events

`harness-events` is the structured execution-event foundation for the jcode-harness cockpit. The first slice is intentionally local and in-process: it defines a typed event schema and a broadcast EventBus that future NDJSON, replay, SSE, broker, gRPC, and WebSocket adapters can consume.

## Current scope

Implemented in `src/harness_events.rs`:

- `HARNESS_EVENT_SCHEMA_VERSION = 1`.
- `HarnessEventLevel` with `trace`, `debug`, `info`, `warn`, and `error`.
- `HarnessEventKind` with initial run, skill, memory, tool, file, test, gate, and approval events.
- `HarnessEvent` as the serialized object.
- `HarnessEventDraft` for producer-side construction.
- `HarnessEventBus` for in-process pub/sub fan-out.

External transports are out of scope for this first slice. Consumers should subscribe to the in-process bus first, then later attach sinks such as NDJSON or SSE.

## Event object

Example:

```json
{
  "schema_version": 1,
  "event_id": "hevt_1778209200000_123",
  "run_id": "run_demo",
  "session_id": "session_demo",
  "parent_event_id": "hevt_parent",
  "timestamp": "2026-05-08T03:00:00Z",
  "sequence": 7,
  "level": "info",
  "kind": "tool_finished",
  "payload": {
    "tool": "cargo test",
    "status": "passed"
  }
}
```

Common fields:

- `schema_version`: event schema version. Increment only for incompatible changes.
- `event_id`: unique event identifier.
- `run_id`: stable run/correlation identifier.
- `session_id`: optional session that owns or observed the event.
- `parent_event_id`: optional parent for nested operations.
- `timestamp`: UTC timestamp.
- `sequence`: monotonic per-`run_id` sequence assigned by `HarnessEventBus`.
- `level`: severity/verbosity level.
- `kind`: typed event kind.
- `payload`: structured metadata. Keep it redacted and reference artifacts instead of embedding large or sensitive content.

## Minimal producer usage

```rust
use jcode::harness_events::{HarnessEventBus, HarnessEventDraft};

let bus = HarnessEventBus::global();
let started = bus.publish(HarnessEventDraft::run_started("run_123"));
let completed = bus.publish(HarnessEventDraft::run_completed("run_123"));
assert_eq!(started.sequence, 1);
assert_eq!(completed.sequence, 2);
```

## Design constraints

- Local-first: no broker or network service is required.
- Non-blocking fan-out: slow or absent subscribers must not fail event production.
- Versioned schema: later sinks should rely on `schema_version` and typed `kind`.
- Privacy-first payloads: raw prompts, secrets, large file contents, and unredacted tool output should not be placed in `payload`.

## Follow-up issues

- #18: NDJSON event log and CLI streaming sink.
- #19: replayable event store and audit timeline.
- #20: SSE endpoint for live dashboard progress.
- #21: redaction, retention, sampling, and privacy policy.
- #22: optional NATS JetStream and Redis Streams adapters.
- #23: gRPC distributed agent control plane prototype.
- #24: WebSocket interactive control channel.
- #25: benchmarks, load tests, and event overhead budgets.
- #26: full protocol guide, examples, and CI recipes.
