# WOS TODO

**Last audited:** 2026-04-15
**Counts:** 18 specs, 18 schemas, 41 document fixtures + 146 conformance fixtures (0 T3 red, 146 green), 5 crates, 197 lint rules (197 tested, 0 untested)

**Links:** [ADR-0058](../thoughts/adr/0058-wos-core-gap-analysis.md) (gap analysis) · [ADR-0057](../thoughts/adr/0057-wos-core-implementation-boundary.md) (core vs implementation) · [Core extraction plan](../thoughts/plans/2026-04-10-wos-core-extraction.md) (complete) · [Runtime plan](../thoughts/plans/2026-04-13-wos-runtime-crate.md) (complete) · [§1 plan](thoughts/plans/2026-04-14-wos-spec-section-1-implementation.md) (complete) · [LINT-MATRIX](LINT-MATRIX.md) · [Runtime Companion](specs/companions/runtime.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md)

**Sequencing logic:** (1) Land zero-dependency foundation work first — provenance export and ontology field identity — before deeper layers lock in. (2) Ship engine adapters to validate the runtime against real commercial workflow engines. (3) Build audit and narrative products on the now-stable provenance export surface. (4) Regulatory companion specs follow once ontology is landed. (5) Interoperability and speculative research last.

---

## 1 — Reference implementation blockers

> §1 closed 2026-04-14 — see Completed.

---

## 2 — Foundational (zero external dependencies)

Highest leverage-per-effort: both items unlock downstream tiers and have no external blocker.

- [ ] **Provenance export** — Serialize internal provenance to W3C PROV-O, OCEL 2.0, IEEE 1849 XES. 30+ record kinds already stable; export path missing. Ships industry-standard audit-tool interop without waiting on engine bindings.
- [ ] **Ontology field identity** — Implement ontology-driven semantic field identity (`ontology-spec.md`). Grounds AI integration, cross-document alignment, and the regulatory specs in §5. Cheaper to land now than to retrofit after regulatory specs are drafted.

---

## 3 — Engine adapters

Validate the runtime against real commercial workflow engines. Shakes out bugs in the reference implementation under production-shape workloads.

- [ ] **Camunda 8 Worker** — Delegate BPMN task execution under WOS governance. Most common BPMN target; broadest external fixture diversity.
- [ ] **Temporal Workflow** — Map WOS evaluation steps to deterministic replay. Natural fit with WOS evaluator determinism.
- [ ] **AWS Step Functions** — Bridge ASL states to WOS transitions. Broadest commercial reach; narrowest semantic fit.

---

## 4 — Audit and evidence products

Build on the stable provenance export surface from §2. Each item depends on export semantics being locked.

- [ ] **Dual-readability narrative** — Machine-readable + human prose from the same provenance; specify and implement generation. The governance/regulatory story lives here.
- [ ] **Merkle provenance chains** — Cryptographic hash-chaining for tamper-evident logs. Requires stable export format.
- [ ] **Simulation trace format** — Standardize replay of simulation runs for validation and tooling.

---

## 5 — Regulatory alignment

External-deadline-driven; benefits from ontology (§2) landing first so field identity is stable before regulatory text cites it.

- [ ] **EU AI Act alignment** — Art. 13–14 alignment spec: draft → 1.0.0.
- [ ] **OMB M-24-10 compliance** — Compliance support spec: draft → 1.0.0.

---

## 6 — Interoperability and speculative research

Pick up when §§2–5 stabilize. Ordered cheap-to-expensive.

- [ ] **Claim check pattern** — Evidence by content hash + URI; not specified in WOS. Conceptually small; valuable once large-evidence use cases appear.
- [ ] **Role-based field visibility** — Specify at WOS vs keep Formspec-only. Decision first, spec second (if WOS wins the boundary).
- [ ] **SCXML interoperability** — Bidirectional WOS ↔ SCXML mapping (currently informative only).
- [ ] **Statutory deadline chains** — Interdependent government deadlines and automated legal consequences; not specified. Largest item in this tier.

---

## Future specs (trigger-gated)

| Spec | Description | Trigger |
|------|-------------|---------|
| Batch Operations | Parallel case instantiation, bulk state transitions | Sustained deployments above 100 cases/minute |
| Federation Profile | Cross-org trust, signed provenance | Second organization adopts WOS |
| Learning Profile | Retraining governance | Long-lived AI agents need retraining policy |

---

## Parked

- [ ] Full lifecycle soundness verification (e.g. linear-time logic).
- [ ] JSON Patch for fine-grained provenance.
- [ ] FEEL-to-FEL migration guide.

---

## Completed

**Specs and schemas**

- [x] Kernel spec (S4.2, S4.10, S9.2) — concurrency, cascade depth, async actions.
- [x] Governance spec (S6.2) — source authority ranking.
- [x] Runtime companion (S5.3, S10, S12, S14) — parallel provenance, convergence cap, EventQueue interface.
- [x] Formspec integration gaps — version pinning, changelog migration, semantic contracts.
- [x] LINT-MATRIX rule count reconciled (197 total; I-001 added in NB.2).
- [x] Kernel schema — `evaluationMode`, `maxRelationshipEventDepth`.
- [x] Governance schema — `scope`, `sourceAuthority`, `ruleId`.
- [x] Case Instance schema — `pendingEvents`, `governanceState`, `volumeCounters`.

**wos-core and runtime capabilities**

- [x] Typed deserialization — Kernel, Governance, AI fixtures round-trip.
- [x] Evaluator — deterministic algorithm from S2.
- [x] Host traits — nine interfaces in `traits/mod.rs`.
- [x] `instance.rs`, `explain.rs`.
- [x] Conformance harness wired to runtime (`WosRuntime` / evaluator path as landed).
- [x] T3 fixtures batches 1–17 (102) and batch 16 processor meta-rules.
- [x] Inline conformance documents — `run_fixture` and fixture parse checks support `documents.* = "inline"`.
- [x] Timer region scoping and tolerance validation.
- [x] `deontic.rs`, `autonomy.rs`, `confidence.rs`, `event_handler.rs`, `eval_mode.rs`, `explain.rs` behavior.

**wos-lint**

- [x] T1/T2 on typed models (`KernelDocument`, `KernelCollections`).
- [x] Typed state-tree walks (replaced manual tag/event collection).
- [x] G-027 sub-delegation depth via typed models.
- [x] T1-TESTS (G-058, G-059, G-062, G-065), T1-K009, CM-001, T2-GAPS (G-060, G-063).
- [x] LINT-COVERAGE — 197 of 197 rules covered (see LINT-MATRIX.md; I-001 added in NB.2).

**Conformance harness hygiene**

- [x] **CONF-META-MOVE** — Move `observe_proxy_behavior` / `observe_assist_governance_proxy` into `wos-core/src/proxy.rs`.
- [x] **CONF-AI050-DIFF** — `differential_check_passed` computed from actual severity + violation-id comparison instead of hard-coded `true`.
- [x] **CONF-AI004-EVIDENCE** — `observe_delegated_formspec_evaluation` sets `full_response_envelope_validated` from `validation_result.valid`.
- [x] **CONF-PROFILE-DEDUP** — `tests/profile_conformance.rs` now delegates to `run_profile_against_fixtures` in `meta.rs`.
- [x] **CONF-RUNTIME-POLICY** — Move deontic, autonomy, confidence, event-handler, and DCR fixture policy into `wos_runtime::ReferenceCompanionPolicy`; conformance only selects/configures it.
- [x] **CONF-RUNTIME-PROVENANCE** — Emit compensation, lifecycle/case separation, and history-cleared provenance from `wos-runtime` / `wos-core`; conformance asserts observed provenance instead of synthesizing it.
- [x] **CONF-EVENT-IDENTITY** — Runtime drain results report the processed event token; fixture draining no longer stops on event name alone.
- [x] **CONF-IDEMPOTENCY-SCOPE** — Scope reference companion idempotency tracking per instance.
- [x] **CONF-STORE-API** — Remove `InMemoryStore` from the conformance public API; engine uses `wos_runtime::InMemoryStore`.
- [x] **CONF-STUB-TESTS** — Document inline stub tests as harness verification, not spec behavior.
- [x] **CONF-BINDING-DOC** — Document `ConformanceBinding`: intentionally permissive, `compute_case_mutation` returns `None`.

**Documentation**

- [x] `wos-spec/README.md`, root `context.md` WOS section, `wos-core/README.md`, `WOS-IMPLEMENTATION-STATUS.md`.

**Conformance profiles**

- [x] Governance Basic/Complete aggregate tests.
- [x] Agent Registration / Confidence Framework aggregate tests.

**SMT / static analysis**

- [x] AG010 finite-domain AST analysis, `finiteDomainDeclarations` in schema/linter, FEL filter rejection.

**Formspec coprocessor**

- [x] FEL `every`/`some` in Formspec core.
- [x] Runtime Companion S15 interface and reference in-memory runtime path.
- [x] `wos-formspec-binding` — adapter surface plus prefill, validation, and mapping tests.
- [x] S15.3 pin re-validation on replay paths — `wos-formspec-binding::FormspecBinding::revalidate_submission` recomputes pin equality fresh on every replay/audit/review call.

**Coprocessor version discipline (S15)**

- [x] S15.1 — register `FormspecBinding` alongside `ConformanceBinding`; real binding path exercised in conformance (61132c1).
- [x] S15.2 — author S15 validation fixtures through real `wos-formspec-binding` path; all 6 fixtures green (b0f3306).
- [x] S15.3 — delete `ConformanceBinding`; pin re-validation enforced on replay paths (0283740 + 0a3c369). `StubValidator` retained for service-invocation contract validation (`contract_outcomes` fixture field), which is a separate code path from the task-binding adapter.

**Kernel/runtime semantics (KS)**

- [x] KS.1 — DeepHistory + ShallowHistory state semantics with conformance fixtures (D1 depth-1, D2 depth-2 + parallel-exit, D3 depth-3); `wos-core` capture/restore (c78848c).
- [x] KS.2 — Milestone firing with pinned ordering (data write durable → `MilestoneFired` → reactive transitions evaluated); 5 conformance fixtures K-M-001 through K-M-005 (521bd54).

**Business calendar (BC)**

- [x] BC.1 — Business Calendar SLA runtime integration: lazy deadline evaluation at check time, `calendarVersion` snapshot, `DidNotConverge` error on convergence failure; 4 fixtures G-S10-001 through G-S10-004 green (c93052f).

**Integration Profile binding kinds (NB)**

- [x] NB.1 — typed `IntegrationBindingKind` enum + `IntegrationBindingHandler` trait; replaced stringly-typed dispatch (f017910).
- [x] NB.2 — outputBinding RFC 9535 profile pinned (wildcard + slice; filter/recursive-descent rejected); lint rule I-001; spec §3.3.1 (e6e916d).
- [x] NB.3 — CloudEvents bindings (`event-emit`, `event-consume`, `callback`) with subject correlation `{instanceId}:{bindingId}:{invocationId}`; full envelope captured in provenance; 6 fixtures INT-EMIT/CONSUME/CALLBACK-001–003 (75c8b21).
- [x] NB.4 — Arazzo, tool, and policy-engine bindings; `PolicyDecision` normalized to `{decision, reasons, obligations}`; 7 fixtures INT-ARAZZO/TOOL/POLICY-001–004 (d79c02b).

**Security / architecture docs**

- [x] Runtime S13 isolation conformance guidance.
- [x] AI-004 / AI-050 behavioral verification strategy (ARCH-AI004).
