# WOS TODO

**Last audited:** 2026-04-13
**Counts:** 18 specs, 18 schemas, 39 document fixtures + 95 conformance fixtures (0 T3 red, 95 green), 5 crates, 196 lint rules (186 tested, 10 untested)

[ADR-0058](../thoughts/adr/0058-wos-core-gap-analysis.md) (gap analysis) |
[ADR-0057](../thoughts/adr/0057-wos-core-implementation-boundary.md) (core vs. implementation boundary) |
[Implementation Plan](../thoughts/plans/2026-04-10-wos-core-extraction.md) (core extraction - COMPLETE) |
[Implementation Plan](../thoughts/plans/2026-04-13-wos-runtime-crate.md) (runtime crate - COMPLETE) |
[LINT-MATRIX](LINT-MATRIX.md) |
[Runtime Companion](specs/companions/runtime.md) |
[Feature Matrix](WOS-FEATURE-MATRIX.md) |
[Implementation Status](WOS-IMPLEMENTATION-STATUS.md)

The reference implementation is now operational. Current focus has shifted from spec extraction to engine bindings and production readiness.

```text
Phase 1          Phase 2          Phase 3          Phase 4          Phase 5
Settle specs ──→ Settle schemas ──→ Extract core ──→ Write fixtures ──→ Build capabilities
(prose)          (structure)       (typed models)   (red)              (green)
                                       │
                                       └──→ Phase 6: Adapt wos-lint ──→ Phase 7: Documentation
                                                                           │
                                                                           └──→ Phase 13: Engine Bindings
```

---

## Phase 1: Settle the specs

- [x] **Kernel spec (S4.2, S4.10, S9.2)** — concurrency, cascade depth, async actions.
- [x] **Governance spec (S6.2)** — source authority ranking.
- [x] **Runtime companion (S5.3, S10, S12, S14)** — parallel provenance, convergence cap, EventQueue interface.
- [x] **Formspec integration gaps** — version pinning, changelog migration, semantic contracts.
- [x] **Housekeeping** — reconcile LINT-MATRIX rule count (196 total).

---

## Phase 2: Settle the schemas

- [x] **Kernel Schema** — `evaluationMode`, `maxRelationshipEventDepth`.
- [x] **Governance Schema** — `scope`, `sourceAuthority`, `ruleId`.
- [x] **Case Instance Schema** — `pendingEvents`, `governanceState`, `volumeCounters`.

---

## Phase 3: Extract wos-core

- [x] **3.1 Typed deserialization** — Kernel, Governance, and AI fixtures round-trip successfully.
- [x] **3.2 Move evaluation logic** — `Evaluator` implements deterministic algorithm from S2.
- [x] **3.3 Add trait interfaces** — 9 host interfaces defined in `traits/mod.rs`.
- [x] **3.4 New modules** — `instance.rs` (serialization) and `explain.rs` (explanation assembly).
- [x] **3.5 Adapt wos-conformance** — Thin harness delegating to `wos-core::Evaluator`.

---

## Phase 4: Write T3 conformance fixtures

- [x] **Batches 1–15** — 88 T3 fixtures covering timers, deontic, autonomy, confidence, and compensation.
- [x] **Batch 16** — Processor conformance meta-rules (AI-001, G-051, etc.).

---

## Phase 5: Build engine capabilities (green)

- [x] Timer region scoping and tolerance validation.
- [x] `deontic.rs` — FEL-based constraint evaluation.
- [x] `autonomy.rs` — escalation/demotion and cap computation.
- [x] `confidence.rs` — decay and cumulative tracking.
- [x] `event_handler.rs` — unified governance/runtime dispatcher.
- [x] `eval_mode.rs` — continuous evaluation mode support.
- [x] `explain.rs` — authority-ranked explanation assembly.

---

## Phase 6: Adapt wos-lint to typed models

- [x] Migrated T1 and T2 rules to use `KernelDocument` and `KernelCollections`.
- [x] Replaced manual tag/event collection with typed state-tree walks.
- [x] Implemented sub-delegation depth check (G-027) using typed models.

---

## Phase 7: Documentation

- [x] **`wos-spec/README.md`** — inventory and architecture updated.
- [x] **`context.md`** — added WOS/Formspec relationship section.
- [x] **`wos-core/README.md`** — documented traits and conformance guidance.
- [x] **`WOS-IMPLEMENTATION-STATUS.md`** — added technical status ledger.

---

## Phase 8: Lint test coverage

- [x] **T1-TESTS** — 12 tests for sidecar gaps (G-058, G-059, G-062, G-065).
- [x] **T1-K009** — duplicate actor id detection.
- [x] **CM-001** — correspondence entry template id uniqueness.
- [x] **T2-GAPS** — business-day SLA (G-060) and template resolution (G-063).

---

## Phase 9: Conformance profiles

- [x] **PROFILE-GOV** — Governance Basic/Complete aggregate tests.
- [x] **PROFILE-AI** — Agent Registration/Confidence aggregate tests.

---

## Phase 10: SMT verifiable subset

- [x] **AG010-FINITE** — AST-only finite domain equality analysis.
- [x] **AG010-DECL** — added `finiteDomainDeclarations` to schema and linter.
- [x] **AG010-FILTER** — verified FEL parser rejects filter expressions.

---

## Phase 11: Formspec coprocessor protocol

- [x] **FEL-QUANTIFIERS** — `every`/`some` built-ins added to Formspec core.
- [x] **COPROCESSOR** — Runtime Companion S15 (Formspec coprocessor) landed.
- [x] **BINDING** — `wos-formspec-binding` adapter implements S15 prefill/mapping.

---

## Phase 12: Security and conformance documentation

- [x] **SECURITY-PROFILE** — conformance guidance for Runtime S13 (Isolation).
- [x] **ARCH-AI004** — behavioral verification strategy for AI-004/AI-050.

---

## Phase 13: Engine Bindings

Adapting the reference evaluator to established engines.

- [ ] **Camunda 8 Worker** — Delegate BPMN task execution to WOS governance.
- [ ] **Temporal Workflow** — Map WOS evaluation steps to deterministic replay.
- [ ] **AWS Step Functions** — Bridge ASL states to WOS transitions.

---

## Future specs (trigger-gated)

| Spec | Description | Trigger |
|------|-------------|---------|
| Batch Operations | Parallel case instantiation, bulk state transitions | Deployment processes >100 cases/minute |
| Federation Profile | Cross-org trust, signed provenance | Second organization adopts WOS |
| Learning Profile | Retraining governance | AI agents run long enough to need retraining |

---

## Deferred

- [ ] Full lifecycle soundness verification (linear time logic proofs).
- [ ] Merkle tree tamper evidence (cryptographic hash-chaining).
- [ ] JSON Patch support for fine-grained provenance.
- [ ] FEEL-to-FEL migration guide.
