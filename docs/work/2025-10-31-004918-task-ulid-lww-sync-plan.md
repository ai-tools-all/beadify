## ULID + LWW Sync Plan

### Goal
Provide a deterministic, distributed event log flow that combines human-friendly issue IDs with globally sortable event identifiers so every replica materializes the same state.

### Event Envelope
- `event_id`: ULID generated at append time (uuidv7-style, monotonic per writer).
- `ts`: ISO-8601 wall clock for observability (non-authoritative).
- `op`, `id`, `actor`, `data`: unchanged from current spec.

### ULID Generation Rules
1. Every append uses `MonotonicULIDGenerator`, seeded from the highest `event_id` seen during the last sync.
2. Generator persists its last 128-bit value locally; on each new event it increments the counter portion when the timestamp is equal, ensuring strict monotonicity.
3. CLI refuses to emit events if it has never synced (no baseline `event_id`).

### Append Workflow
1. `beads sync` must run (implicitly or explicitly) before any mutating command.
2. After sync, the client holds `last_processed_offset` and `last_event_id`.
3. Mutating command constructs the event payload, requests a ULID from the generator, writes the NDJSON line to `events.jsonl`, records the starting byte offset, and updates `_meta` with the new offset and `last_event_id`.

### Merge & Conflict Resolution
- Git merge driver concatenates BASE/LOCAL/REMOTE segments, emits the first occurrence of each line, and maintains append order.
- Post-merge, `beads sync --full` (or normal sync) reads the file sequentially and sorts only by `event_id` when materializing; lower ULIDs take effect first, higher ULIDs overwrite fields (“Last Write Wins”).
- Concurrent updates to the same fields resolve deterministically because ULIDs totally order events.

### Incremental Sync Logic
1. Read `_meta.last_processed_offset` and seek in `events.jsonl`.
2. Stream each new line, parse the ULID, and apply events immediately.
3. Update `_meta` with the new offset and highest ULID processed.
4. If a new ULID is <= `last_event_id`, queue a warning and fall back to replaying from the previous checkpoint; this indicates a generator bug or unsynced client.

### Client Guardrails
- Mutating commands enforce a “fresh before write” invariant by running `beads sync` when the local `last_event_id` is older than the file’s tail.
- Offline mode is supported, but before reconnecting the client must run `beads sync`; if its generator’s timestamp lags behind the log tail, the monotonic counter still yields greater ULIDs than any previously observed event.

### Failure Handling
- **Generator regression:** Detect via ULID comparison; abort the write and force a full sync.
- **Merge anomalies:** After `git merge`, run `beads sync --verify` that scans sequentially ensuring ULIDs are strictly increasing; duplicates or regressions surface as actionable errors.
- **Partial writes:** Append uses `fsync` after write; sync ignores trailing malformed lines until repaired.

### Open Follow-Ups
- Decide whether to persist `_meta` in a single row or separate keys (`last_processed_offset`, `last_event_id`).
- Document CLI UX for forced syncs and error recovery.
- Evaluate using Hybrid Logical Clocks instead of ULID if wall-clock monotonicity cannot be trusted.
