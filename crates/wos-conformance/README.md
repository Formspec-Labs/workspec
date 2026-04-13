# wos-conformance

Dynamic conformance test runner for WOS workflows. Executes event sequences against kernel documents and asserts on state transitions, provenance records, timer behavior, compensation ordering, and deontic enforcement. Covers 104 normative constraints that require runtime execution to verify.

## What it tests

Static linting (`wos-lint`) catches structural problems in WOS documents. This crate catches behavioral problems — constraints that only manifest when a workflow actually executes.

- **Determinism** — same document + same events produce same transitions across runs (K-011, K-033)
- **Provenance completeness** — every state mutation, timer event, task transition, and relationship change produces the correct provenance record (K-016, K-018, K-020, K-028, K-046)
- **Timer behavior** — timers survive restarts, cancel on region exit, reset on reentry, route to their creating region (K-025, K-038, K-043, K-044, K-045)
- **Compensation** — saga pattern executes in reverse order, pivot step excluded, nested scopes independent, completion event processed (K-027, K-039, K-040, K-041, K-042)
- **Deontic enforcement** — constraints evaluated in prescribed order, most restrictive action wins, null propagation by impact level (AI-009 through AI-017)
- **Autonomy caps** — effective autonomy is the minimum across all sources, calibration expiry enforced, demotion timing correct (AI-021, AI-022, AI-028, AC-001)
- **Confidence** — every output has a report, decay triggers multiply correctly, cumulative tracking pauses sessions at checkpoints (AI-034 through AI-038, AG-004)
- **Hold/resume** — timer starts on hold entry, cancels when resume trigger arrives (G-030, G-054)
- **DCR constraint zones** — excluding a pending activity raises error, zone satisfaction computed correctly (AG-002, AG-003)

See [`LINT-MATRIX.md`](../../LINT-MATRIX.md) for the complete constraint catalog.

## Fixture format

Each conformance test is a JSON fixture declaring documents, an event sequence, and expectations:

```json
{
  "id": "K-011-determinism",
  "rule": "K-011",
  "description": "Same document + events produce same transitions",
  "documents": {
    "kernel": "fixtures/kernel/purchase-order-approval.json"
  },
  "event_sequence": [
    { "event": "approve", "actor": "approver", "data": { "amount": 3000 } }
  ],
  "expected_transitions": [
    { "from": "submitted", "to": "approved", "event": "approve" }
  ],
  "expected_provenance": [
    { "type": "stateTransition", "actorId": "approver" }
  ]
}
```

**Fields:**
- `documents` — paths to WOS documents keyed by role (kernel, governance, ai, etc.)
- `event_sequence` — ordered events to feed into the workflow, with optional `actor`, `data`, and `delay` (ISO 8601 duration, for timer tests)
- `expected_transitions` — state transitions that must occur in order
- `expected_provenance` — provenance records that must be produced (partial match)
- `expected_errors` — diagnostic errors expected for negative tests

## Usage

```rust
use wos_conformance::run_fixture;

let fixture_json = std::fs::read_to_string("fixture.json").unwrap();
let result = run_fixture(&fixture_json, ".").unwrap();

assert!(result.passed, "failures: {:?}", result.failures);

// Inspect actual execution
for t in &result.transitions {
    println!("{} -> {} on '{}'", t.from, t.to, t.event);
}
for p in &result.provenance {
    println!("{:?}: {:?}", p.record_kind, p.event);
}
```

## Processor Reports

Batch 16 processor claims can also be emitted as a report artifact:

```bash
cargo run -p wos-conformance --bin wos-conformance-report -- \
  --manifest processor-manifest.json \
  --format text
```

The report uses `verify_processor_manifest()` under the hood, prints either
JSON or a readable text summary, and exits nonzero if any claimed meta-rule
fails verification.

## Engine

The conformance engine implements the deterministic evaluation algorithm from the Lifecycle Detail Companion (S2):

1. Collect transition candidates from active states
2. Evaluate guards in document order (first match wins, per Kernel S4.6)
3. Execute exit actions innermost first, transition actions, entry actions outermost first
4. Update configuration, emit provenance

Events matching no transition are recorded in provenance but do not change state (Kernel S4.9).

Guard evaluation uses `fel-core` — the same FEL runtime used by the Formspec engine, ensuring expression semantics are identical across all WOS and Formspec processors.

## Dependencies

- **`fel-core`** — FEL evaluator for guard expressions. Without this, guards are treated as always-true (useful for basic transition tests but insufficient for full conformance).
- **`wos-lint`** — document parsing and kind detection. The conformance engine reuses `wos-lint`'s document model rather than reimplementing WOS document detection.
- **`serde_json`** — fixture parsing and document traversal.

## Relationship to wos-lint

`wos-lint` checks documents before execution. `wos-conformance` checks behavior during execution. Together they cover all 187 normative constraints in the WOS Verification Matrix:

| Tool | Tier | Rules | What it checks |
| ---- | ---- | ----- | -------------- |
| JSON Schema | — | structural | Required fields, enum values, property names |
| `wos-lint` | T1 | 32 | Single-document structural constraints |
| `wos-lint --project` | T2 | 51 | Cross-document resolution + FEL AST analysis |
| `wos-conformance` | T3 | 104 | Runtime behavioral guarantees via event-driven fixtures |
