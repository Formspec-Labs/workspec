# WOS TODO

**Last audited:** 2026-04-14
**Counts:** 18 specs, 18 schemas, 41 document fixtures + 102 conformance fixtures (0 T3 red, 102 green), 5 crates, 196 lint rules (196 tested, 0 untested)

**Links:** [ADR-0058](../thoughts/adr/0058-wos-core-gap-analysis.md) (gap analysis) · [ADR-0057](../thoughts/adr/0057-wos-core-implementation-boundary.md) (core vs implementation) · [Core extraction plan](../thoughts/plans/2026-04-10-wos-core-extraction.md) (complete) · [Runtime plan](../thoughts/plans/2026-04-13-wos-runtime-crate.md) (complete) · [LINT-MATRIX](LINT-MATRIX.md) · [Runtime Companion](specs/companions/runtime.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md)

**Sequencing logic:** (1) Measure honestly before optimizing—verification and conformance first. (2) Finish what the specs already promise (runtime consumption of existing schema/fixtures). (3) Tighten the Formspec coprocessor contract in runtime. (4) Complete hierarchical state semantics so external engines bind to stable behavior. (5) Ship adapters. (6) Build audit and narrative products on top of stable provenance. (7) Ontology then regulatory companion specs. (8) Interop and speculative work last.

---

## 1 — Reference implementation blockers

Finish these before binding WOS to an external execution engine or calling a demo implementation representative.

**Sub-sequencing within §1:** (a) Binding-backed S15 + version-pin — exercises the existing path and unblocks honest claims. (b) History + Milestone — settles kernel semantics before engines encode assumptions. (c) Business Calendar runtime integration — schema done, smallest remaining item. (d) CloudEvents / Arazzo / tool / policy-engine bindings — largest chunk, benefits from (a) landing first so the provenance shape is pinned before new binding kinds extend it.

**Runtime schema consumption** — Finish execution paths for artifacts that already exist. This unblocks honest “supported in reference runtime” claims before engine bindings.

- [ ] **Integration Profile Processor** — Initial `invokeService` request-response binding consumption is implemented in `wos-runtime` (typed profile model, FEL input mapping, request/response contract validation, idempotency expression, output binding, provenance). Remaining: CloudEvents 1.0 envelopes, Arazzo-specific execution metadata, and policy-engine decision mapping.
- [ ] **Integration Profile binding coverage** — Finish the remaining binding kinds that the profile already names:
  - `event-emit`, `event-consume`, `callback` — CloudEvents 1.0 envelope handling. Correlate callbacks via `subject = {instanceId}:{bindingId}:{invocationId}`. Capture the full envelope in provenance (headers, `id`, `time`, `source`, `specversion`), not only `data`. Reject events missing `id` or `source` at ingress; no silent default-filling.
  - `arazzo-sequence` — model each step as its own `invokeService` invocation with step-level provenance, not one monolithic record. Cross-step state lives in WOS (not the Arazzo runner) so pause/resume across the sequence survives restart.
  - `tool` — reuse the `invokeService` request/response contract and version-pinning discipline; no parallel contract. Tool calls feed the same deontic / autonomy / confidence pipeline as other service invocations.
  - `policy-engine` — normalize every engine's output to `{decision: allow|deny|indeterminate, reasons: [...], obligations: [...]}` at the binding boundary before it enters provenance. Pin `indeterminate` semantics now so downstream audit tools do not learn engine-specific shapes.
- [ ] **Integration Profile output binding** — Pin an explicit **RFC 9535 profile** for `outputBinding`: member access, index, wildcard, slices only. Exclude filter expressions (`[?()]`) and recursive descent (`..`) until a concrete binding justifies them. Lint at definition load; grow the profile backwards-compatibly when needed. Rationale: predictability and static analysability over power; full JSONPath would introduce a second expression language inside binding documents and complicate replay/provenance.
- [ ] **Business Calendar SLA** — Consume `wos-business-calendar` for Governance S10.3 SLA deadlines; schema done; **runtime integration** pending. Compute deadlines **lazily at check time**, not eagerly at case creation, so calendar updates (holidays, business-day changes) shift future deadlines. Snapshot `calendarVersion` at each evaluation; do not cache computed deadlines across calendar-version changes.

**Coprocessor version discipline** — Validate that the Formspec path matches pinned-definition semantics.

- [ ] **Binding-backed S15 conformance** — Run task draft, submit, validation, pin-check, and mapping fixtures through `wos-runtime` + `wos-formspec-binding`, not the permissive `ConformanceBinding` / `StubValidator` path. Once the real path is green, **delete `ConformanceBinding` and `StubValidator` entirely** — no dual-path fallback. If a fixture cannot pass the real path, fix the fixture or the binding, do not route around it.
- [ ] **Version-pinned response validation** — Confirm runtime behavior matches definition URL + version pinning in spec. Pin URL is **mandatory** in every S15 request; runtime fails loudly on absence. No "default to latest" fallback. Assert pin equality on re-validation paths (review, audit replay), not only at initial submit.

**Kernel/runtime semantics** — Settle these before external engines encode assumptions about resumption and checkpoints.

- [ ] **History state semantics** — DeepHistory (full snapshot) vs ShallowHistory (exit point) for hierarchical resumption; schema field exists, behavior does not. Implement **DeepHistory first** — safer default; ShallowHistory is a performance optimization for deep hierarchies that isn't needed yet. Write conformance fixtures **before** implementation; at minimum one fixture per nesting depth (1, 2, 3) covering normal re-entry, re-entry after parallel-region exit, and re-entry after a transition crosses the history boundary.
- [ ] **Milestone firing** — Data-driven checkpoints independent of workflow state; schema + lint K-013 exist; add **conformance** coverage. Pin ordering: **data write durable → milestone emitted → reactive transitions evaluated**. Keeps provenance narratable ("X changed, milestone Y fired, which enabled transition Z") and prevents milestone and transition from appearing in the same logical instant.

---

## 2 — Engine bindings

Snap commercial/workflow engines onto the reference runtime once §1 is trustworthy.

- [ ] **Camunda 8 Worker** — Delegate BPMN task execution under WOS governance.
- [ ] **Temporal Workflow** — Map WOS evaluation steps to deterministic replay.
- [ ] **AWS Step Functions** — Bridge ASL states to WOS transitions.

---

## 3 — Auditability and evidence products

Formats and integrity layers assume a stable provenance stream from the runtime.

- [ ] **Provenance export** — Serialize internal provenance to W3C PROV-O, OCEL 2.0, IEEE 1849 XES (30+ record kinds exist; export path missing).
- [ ] **Dual-readability narrative** — Machine-readable + human prose from the same provenance; specify and implement generation.
- [ ] **Merkle provenance chains** — Cryptographic hash-chaining for tamper-evident logs (builds on stable export/stream semantics).
- [ ] **Simulation trace format** — Standardize replay of simulation runs for validation and tooling.

---

## 4 — Ontology and regulatory companion specs

Stable field identity supports AI and documentation; companion specs follow when semantics are implementable.

- [ ] **Ontology field identity** — Implement ontology-driven semantic field identity (`ontology-spec.md`); grounds AI and cross-document alignment.
- [ ] **EU AI Act alignment** — Art. 13–14 alignment spec: draft → 1.0.0.
- [ ] **OMB M-24-10 compliance** — Compliance support spec: draft → 1.0.0.

---

## 5 — Interoperability and long-horizon research

- [ ] **SCXML interoperability** — Bidirectional WOS ↔ SCXML mapping (currently informative only).
- [ ] **Statutory deadline chains** — Interdependent government deadlines and automated legal consequences; not specified.
- [ ] **Role-based field visibility** — Specify at WOS vs keep Formspec-only.
- [ ] **Claim check pattern** — Evidence by content hash + URI; not specified in WOS.

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
- [x] LINT-MATRIX rule count reconciled (196 total).
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
- [x] LINT-COVERAGE — 196 of 196 rules covered (see LINT-MATRIX.md).

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

**Security / architecture docs**

- [x] Runtime S13 isolation conformance guidance.
- [x] AI-004 / AI-050 behavioral verification strategy (ARCH-AI004).
