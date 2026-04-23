# Expected Trace Goldens

Each `*.json` file in this directory is a committed conformance-trace golden.
The runtime regenerates a live trace from the matching T3 fixture and
`assert_trace_matches` diffs that live trace against the golden here
(see `crates/wos-conformance/tests/trace_parity.rs`).

## Per-fixture notes

- **ai-auto-001-escalation-expiry-revocation.json** — Escalation expiry
  revokes elevated autonomy. The run exercises the deontic / autonomy
  policy engine, so the trace carries a populated `policiesApplied`
  block plus an `autonomyDemotion` provenance record.

- **ai-auto-002-drift-alert-demotion.json** — Drift-alert demotion path.
  The runtime reroutes the event through `escalated` so the kernel
  lands on `humanTriage`; on this path the drift-alert policy fires as
  a runtime-side demotion *before* any policy-application recording, so
  the golden carries no `policiesApplied` block. The trace is short on
  purpose — adding a fake `policiesApplied` here would desync from
  what the runtime actually emits. See the `run_t3_fixture` harness
  and `trace_parity_ai_auto_002_drift_alert_demotion` for the
  round-trip check that keeps this golden honest.
