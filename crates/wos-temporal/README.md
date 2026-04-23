# wos-temporal

**Status:** Skeleton-only. No implementation ships today. This crate anchors the target architecture for the first Temporal.io engine adapter so the trait surface in `wos-core` / `wos-runtime` stays adapter-ready.

**Reference architecture:** [`thoughts/examples/temporal-reference-implementation.md`](../../thoughts/examples/temporal-reference-implementation.md) — 15-section design doc specifying workflow shape, activities, stores, coprocessor handoff, and deployment topology. Read that first.

**Tracking plan:** [`thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md`](../../thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md) — Track H4 owns this crate's skeleton posture; Phase 5 Adapter Readiness (prior plan `2026-04-13-wos-runtime-crate.md`) owns the first real implementation.

---

## Why this crate exists as a skeleton

Two purposes:

1. **Locks the trait surface.** The reference doc names the exact set of `wos-core` traits a Temporal adapter needs. Tracks G and H in the plan land those traits. If any of them drift out of adapter-usable shape, the skeleton is the early-warning signal — either CI fails or the skeleton stops type-checking against the reference structure.

2. **Gives the commercial-demand trigger a starting point.** When a customer asks "does WOS run on Temporal?", the answer is a concrete crate + a design doc, not a blank page. Shortens the distance between "first paying customer who needs this" and "shipping it" from months to weeks.

## Why it's not a real impl

Plan §7 positioning: engine adapters are held until a first-commercial-deployment-demands trigger fires. Building a full Temporal worker now (~4 weeks) without a consumer paying for it is speculative engineering. The reference doc + this skeleton preserve the option without spending the capital.

---

## Intended topology

```text
 SaaS API / Client                     Temporal Server
       │                                      │
       │  start_workflow, signal, query       │  event history
       ▼                                      ▼
┌────────────────────────────────────────────────────────┐
│                 wos-temporal (this crate)              │
│  workflow.rs    coprocessor.rs                         │
│  activities/*   store/*     queries.rs    signals.rs   │
└────────────┬───────────────────────────────────────────┘
             │ traits from wos-core
             ▼
        wos-runtime  ─────►  wos-core  (pure eval, no I/O)
```

`wos-temporal` owns: the Temporal workflow function (one WOS case = one workflow instance), activity implementations that wrap wos-core traits, signal / query handlers, and Postgres-backed stores for tasks + provenance (separate from Temporal's event history).

`wos-temporal` does NOT own: governance evaluation (wos-core), runtime orchestration (wos-runtime), or Formspec semantics (wos-formspec-binding). Those stay upstream.

---

## Traits this crate will implement when real

From `wos-core::traits` (after Track G/H land):

| Trait | Temporal impl |
|---|---|
| `InstanceStore` | `TemporalInstanceStore` — event-sourced via Temporal history; uses additive `append_event` / `replay_events` from H3 |
| `EventQueue` | Maps to Temporal signals |
| `TimerService` (H1) | `TemporalTimerService` — `schedule` → `ctx.timer_with_signal(...)`, `cancel` → `ctx.cancel_timer(...)`. ~20 lines. |
| `ExternalService` | `TemporalActivityService` — wraps external calls as Temporal activities with built-in retry/idempotency |
| `TaskStore` (G8) | `PostgresTaskStore` — separate from Temporal history per reference doc §9 |
| `NotificationService` (G9) | `TemporalNotificationActivity` — notifications as durable activities |
| `VisibilityService` (G11) | Maps to `temporal_client.query_workflow(...)` |
| `ProvenanceSigner` (widened by G2) | Consumer choice — local Ed25519 or real Formspec Respondent Ledger; transport is a Temporal activity |
| `Coprocessor` (G7) | `FormspecTemporalCoprocessor` — validates submissions + maps to case file at workflow-start time |

Traits that don't need a Temporal-specific impl (defaults work): `DocumentResolver`, `ContractValidator`, `ReportRenderer`, `Clock`, `TaskPresenter`, `ActionExecutor`, `AccessControl`, `IdentityResolver`, `DirectoryService`, `PolicyEngine`.

## Skeleton contents (when H4 lands)

Per Track H4 in the plan, the skeleton depth is a documented open question ([`thoughts/reviews/2026-04-22-di-review-open-questions.md`](../../thoughts/reviews/2026-04-22-di-review-open-questions.md#q10-wos-temporal-skeleton-h4-depth) Q10). Three options: (A) zero-code stubs, (B) compile-valid `unimplemented!()` shells, (C) minimal runnable in-process mock. Plan currently targets (B).

```text
wos-temporal/
├── Cargo.toml                    # publish = false; temporal-sdk feature-gated
├── README.md                     # this file
└── src/
    ├── lib.rs                    # module declarations + doc comments
    ├── workflow.rs               # Temporal workflow: WOS case lifecycle
    ├── coprocessor.rs            # Formspec → WOS handoff (wraps wos-formspec-binding::Coprocessor)
    ├── signals.rs                # WosSignal enum (1:1 with WOS events)
    ├── queries.rs                # WosQuery enum (maps to VisibilityService)
    ├── activities/
    │   ├── mod.rs
    │   ├── human_task.rs         # create_human_task, complete_task, claim_task
    │   ├── service.rs            # invoke_external_service
    │   ├── notification.rs       # send_notification
    │   ├── provenance.rs         # append_provenance
    │   ├── contract.rs           # validate_contract
    │   └── governance.rs         # evaluate_governance pipeline (reference §6)
    └── store/
        ├── mod.rs
        ├── provenance.rs         # PostgresProvenanceStore
        └── task.rs               # PostgresTaskStore
```

## Implementation triggers

The skeleton becomes a real impl when any of these fire:

- **Commercial trigger.** A paying customer asks to run WOS on Temporal. This is plan §7's explicit demand-signal trigger.
- **Reference-implementation trigger.** The project commits to shipping a Temporal-backed reference deployment (e.g., for a public benchmark or demo). Plan Q1 option B.
- **Compliance trigger.** A regulatory or procurement process requires demonstrating multi-engine portability; the skeleton alone isn't sufficient evidence.

Until one of these fires, this crate stays at whatever skeleton depth H4 ships (see Q10 recommendation).

## Open architectural questions

Tracked in [`thoughts/reviews/2026-04-22-di-review-open-questions.md`](../../thoughts/reviews/2026-04-22-di-review-open-questions.md):

- **Q1.** Should Phase 5 of the prior build-order plan be reinstated as an active commitment, or should the "held until demand" posture win?
- **Q2.** Does spec kernel §9 G4 (timer state in `CaseInstance`) need amendment to allow H1's externalized-timer shape that Temporal requires?
- **Q3.** Chain-redundancy posture: is WOS's `previous_hash` chain defense-in-depth or transitional when the Respondent Ledger is wired?
- **Q4.** Export-to-Temporal framing (IDEA_SCRATCH §23) — separate artifact or retire?
- **Q9.** `AgentInvocationService` as separate trait vs subsumed under `ExternalService`?
- **Q10.** Skeleton depth (this crate): zero-code, compile-valid stubs, or minimal runnable?
- **Q16.** Should a parallel `wos-lambda` skeleton ship too?

None of these block the skeleton itself. They gate how much the skeleton does and what sibling crates exist.

---

## Quick links

- Reference architecture (READ FIRST): [`thoughts/examples/temporal-reference-implementation.md`](../../thoughts/examples/temporal-reference-implementation.md)
- Plan Track H (structural corrections): [`thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md`](../../thoughts/plans/2026-04-18-wos-remainder-di-seam-framing.md)
- Open questions: [`thoughts/reviews/2026-04-22-di-review-open-questions.md`](../../thoughts/reviews/2026-04-22-di-review-open-questions.md)
- ADR naming expected adapter crates: [`thoughts/archive/adr/0057-wos-core-implementation-boundary.md`](../../thoughts/archive/adr/0057-wos-core-implementation-boundary.md)
- Prior plan's Phase 5 Adapter Readiness: [`thoughts/plans/2026-04-13-wos-runtime-crate.md`](../../thoughts/plans/2026-04-13-wos-runtime-crate.md)
- Spec durable-execution guarantees G1–G5: [`specs/kernel/spec.md`](../../specs/kernel/spec.md) §9, [`specs/companions/runtime.md`](../../specs/companions/runtime.md) §6
