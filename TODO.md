# WOS TODO

**Last audited:** 2026-04-10
**Counts:** 18 specs, 18 schemas, 26 fixtures (2 T3 red), 3 crates, 189 lint rules (85 tested)

[ADR-0058](../thoughts/adr/0058-wos-core-gap-analysis.md) (gap analysis) |
[ADR-0057](../thoughts/adr/0057-wos-core-implementation-boundary.md) (core vs. implementation boundary) |
[Implementation Plan](../thoughts/reviews/2026-04-09-wos-core-companion-review.md) (phases, success criteria, content recovery) |
[LINT-MATRIX](LINT-MATRIX.md) |
[Runtime Companion](specs/companions/runtime.md) |
[Feature Matrix](WOS-FEATURE-MATRIX.md) (competitive comparison, implementation status, audit corrections)

The order below minimizes tech debt. Each phase depends on the one before it. Doing Phase 3 before Phase 2 means building on raw JSON that gets rewritten. Doing Phase 4 before Phase 3 means writing fixtures against an API that changes.

```text
Phase 1          Phase 2          Phase 3          Phase 4          Phase 5
Settle specs ──→ Settle schemas ──→ Extract core ──→ Write fixtures ──→ Build capabilities
(prose)          (structure)       (typed models)   (red)              (green)
                                       │
                                       └──→ Phase 6: Adapt wos-lint ──→ Phase 7: Documentation
```

---

## Phase 1: Settle the specs

Normative prose changes. No code, no schemas. Get the words right before anything depends on them.

### Kernel spec (`specs/kernel/spec.md`)

- [x] **S4.2** — add concurrency statement: "Events MUST be processed serially per instance. Concurrent event delivery MUST be serialized. Multiple actors MAY append to case state concurrently (S5.4), but lifecycle transitions MUST be serialized." _(backport from Runtime S4.1)_
- [x] **S4.10** — add `$related.stateChanged`, `$related.resolved`, `$related.holdReleased` to kernel-generated event catalog _(backport from Runtime S14)_. Add `maxRelationshipEventDepth` (default 3) with cascade prevention _(novel addition — Runtime S14 defines the three `$related.*` events but not the depth cap; author as new content)_.
- [x] **S9.2** — add async action semantics: "Actions within a single state execute sequentially in document order. Actions across parallel regions MAY execute concurrently; provenance MUST record actual execution order." _(backport from Runtime S5)_

### Governance spec (`specs/governance/workflow-governance.md`)

- [x] **S6.2** — add `sourceAuthority` enum (`statute`, `regulation`, `policy`, `guideline`) to `RuleReference`. Named `sourceAuthority` to avoid collision with Delegation's `authority` field.

### Runtime companion (`specs/companions/runtime.md`)

- [x] **S5.3** — resolve parallel provenance ordering: mandate document-order-of-regions, or state implementation-defined. _(already done — implementation-defined with must-record constraint)_
- [x] **S10** — clarify convergence cap: transitions fired during the cycle are committed (they already emitted provenance). State that timer-driven mutations trigger re-evaluation in continuous mode.
- [x] **S12** — add `EventQueue` host interface (S12.8). `listByState`/`listByDefinition` marked OPTIONAL. Provenance log storage clarified as separate append-only store with cursor.
- [x] **S14** — add `maxRelationshipEventDepth` cascade prevention (S14.5).

### Formspec integration gaps

- [x] Add Respondent Ledger SHOULD-to-MUST elevation reference in governance spec S3.2. _(plan Section 3.2)_
- [x] Add VP-01/VP-02 version pinning citations in kernel S9.6. _(plan Section 3.2)_
- [x] Add Changelog S4 reference in Runtime S11 for version migration. _(plan Section 3.2)_
- [x] Add Ontology S3 reference in AI Integration S3.5 for semantic agent contracts. _(plan Section 3.2)_

### Housekeeping

- [x] Reconcile LINT-MATRIX rule count: corrected to 181 total (30 T1 + 50 T2 + 101 T3). Header, tier box, summary table, and gap counts now agree.

---

## Phase 2: Settle the schemas

JSON Schema updates. These define the structure that typed models deserialize into. Get them right before writing Rust structs.

### `schemas/wos-kernel.schema.json`

- [x] Add `evaluationMode` property (enum: `event-driven`, `continuous`, default: `event-driven`). Also added `maxRelationshipEventDepth` (integer, default: 3).

### `schemas/wos-workflow-governance.schema.json`

- [x] Add `scope` property (FEL string) to `ReviewProtocolBinding`, `DueProcess`, `HoldPolicy`. _(backport from Runtime S8.2)_
- [x] Add `sourceAuthority` enum to `RuleReference` $def (`statute`, `regulation`, `policy`, `guideline`). Also added `ruleId`, `description`, `citation` fields.

### `schemas/wos-case-instance.schema.json`

- [x] Add `pendingEvents` array with `PendingEvent` $def.
- [x] Add `governanceState` object with `GovernanceState`, `ActiveDelegation`, `ActiveHold` $defs.
- [x] Add `volumeCounters` object with `VolumeCounters`, `VolumeCounter` $defs.

---

## Phase 3: Extract wos-core

Move the domain model and evaluation algorithm from `wos-conformance` into `wos-core` using typed models. After this phase, no `serde_json::Value` walking in the evaluation hot path.

### 3.1 Typed deserialization

- [x] Verify `KernelDocument` deserializes all 4 valid kernel fixtures (benefits-adjudication, case-relationship-appeal, medicaid-redetermination, purchase-order-approval). Note: benefits-correspondence-metadata is a sidecar, purchase-order-provenance is a test artifact, invalid-documents is a test harness — none are kernel documents.
- [x] Add unit tests for typed model deserialization — 8 tests in `tests/kernel_deser.rs` (4 fixture round-trips + 1 negative test + 3 inline coverage for Phase 2 fields), 2 in `tests/governance_deser.rs`, 1 in `tests/ai_deser.rs`. Fixed `Milestone.id` (was required, now keyed by map like fixtures). Spec S4.13 updated to match.
- [x] Type remaining `Value` fields: `contracts` is now `HashMap<String, ContractReference>`, `execution` is now `Option<ExecutionConfig>` (with `compensable: bool`). Added `evaluationMode`, `maxRelationshipEventDepth`, `title`, `description`, `schema`, `extensions`. Added `extensions` to `State`, `Actor`, `Action`, `ExecutionConfig`. `Action.value` and `Action.data` remain as `serde_json::Value` (intentionally dynamic — these carry arbitrary payloads). Schema: added `ref` to `ContractReference` required array.
- [x] Add typed models for governance (`GovernanceDocument`) and AI (`AIIntegrationDocument`) in `model/governance.rs` and `model/ai.rs`. Full type hierarchies: Governance has 24 types (DueProcess, Pipeline, Delegation, HoldPolicy, etc.), AI has 28 types (AgentDeclaration, DeonticConstraints, FallbackLevel, etc.). Both round-trip against fixtures.

### 3.2 Move evaluation logic

- [x] Implement full typed evaluation algorithm in `wos-core/eval.rs`: `process_event`, `try_fire_transition`, `fire_transition`, `enter_state`, guard evaluation via FEL, action execution (setData/startTimer/cancelTimer), parallel region routing, $join generation. Uses typed `State`/`Transition`/`Action` — no `serde_json::Value` walking. Includes `IndexedState` flat index, `ObservedTransition`, and `parse_iso_duration_to_ms`.
- [x] Consolidate timer logic: conformance engine now delegates to `wos_core::Evaluator`, which uses `wos_core::Timers` and `wos_core::timer::Timer` internally. Conformance's private `Timer` struct (with `timer_id` field name mismatch) is deleted. The `fel-core` direct dependency was also removed from `wos-conformance/Cargo.toml`.
- [x] Consolidate provenance: wos-core now has all 10 `ProvenanceKind` variants (added OnEntry, OnExit, ActionExecuted, InvalidDuration) and all constructor methods. Conformance now re-exports from wos-core. Field name: `record_kind` (matches conformance convention).
- [x] Add `EvalContext::from_case_state()` constructor. Also enriched `to_fel_environment()` to produce `caseFile` as both FEL object and dotted paths, plus `event` and `instance` namespaces — matching conformance engine's `build_fel_context` behavior.

### 3.3 Add trait interfaces

- [x] `traits/mod.rs` — 9 traits: `InstanceStore`, `DocumentResolver`, `ContractValidator`, `ExternalService`, `AccessControl`, `ProvenanceSigner`, `ReportRenderer`, `EventQueue`, `ActionExecutor`. Plus `ValidationResult` struct.
- [x] `DefaultRuntime` struct — bundles in-memory `InstanceStore`, `AccessControl` (permissive), and `EventQueue` implementations.
- [x] Verify `Evaluator::process_event` uses `&mut self` (ADR-0057 requires type-system-enforced serialization). _(confirmed in wos-core/eval.rs)_

### 3.4 New modules

- [x] `instance.rs` — `CaseInstance` struct with `InstanceStatus`, `TimerState`, `PendingEvent`, `GovernanceState`, `ActiveDelegation`, `ActiveHold`, `VolumeCounters`, `CompensationLog`.
- [x] `explain.rs` — `Explanation`, `ReasoningRecord`, `CounterfactualRecord` types + `assemble_explanation()` stub. Full assembly deferred to Phase 5 when fixtures exercise explanation output.

### 3.5 Adapt wos-conformance

- [x] Add `wos-core` to `wos-conformance/Cargo.toml`. _(done — wos-core now in deps)_
- [x] Make `wos-conformance` a thin harness calling `wos-core::Evaluator`. _(engine.rs rewritten: deserializes kernel into `KernelDocument`, creates `Evaluator`, delegates all lifecycle evaluation. No `serde_json::Value` walking in the engine. Removed `fel-core` and `wos-lint` direct dependencies.)_
- [x] Remove duplicate `ProvenanceRecord`/`ProvenanceKind` from conformance — now re-exports from wos-core. Timer consolidated (conformance's private `Timer` struct deleted).
- [x] Implement `StubValidator`, `StubService`, `InMemoryStore`. _(stubs.rs: `InMemoryStore` implements `InstanceStore`, `StubValidator` implements `ContractValidator` with configurable pass/fail, `StubService` implements `ExternalService` with configurable response. 6 tests.)_
- [x] Add `contract_outcomes` to fixture format for Formspec coprocessor testing. _(fixture.rs: `contract_outcomes: HashMap<String, ContractOutcome>` with `ContractOutcome { valid, errors }`. Serde-defaulted so existing fixtures are unaffected.)_
- [x] Verify all 151+ existing tests still pass. _(208 total: 151 original + 20 deser/eval + 8 duration parsing + 15 conformance-engine + 8 sidecar deser + 6 stub tests = all pass.)_

---

## Phase 4: Write T3 conformance fixtures (red)

Write failing fixtures for every untested T3 rule. Each fixture declares expected behavior that the engine can't yet produce. This is the spec-as-executable-tests step.

### Fixture strategy

- One fixture per rule (or small tightly-coupled group).
- Synthetic minimal kernels, not extensions of benefits-adjudication.
- Naming: `{rule-id}-{short-description}.json` (fixture), with matching kernel in `fixtures/kernel/`.
- Test pattern: `#[ignore = "Phase 5: requires {module} -- {rule-id}"]`.
- `cargo test` skips red tests; `cargo test -- --ignored` shows what remains.

### Batch table (priority order)

| # | Batch | Rules | Rule IDs | Engine dep |
|---|-------|-------|----------|------------|
| 1 | Cancel-siblings/fail-fast | 2 | K-044, K-045 | `eval.rs` |
| 2 | Hold/resume lifecycle | 2 | G-030, G-054 | `eval.rs` |
| 3 | Deontic enforcement | 20 | AI-009, AI-010, AI-011, AI-012, AI-013, AI-014, AI-015, AI-016, AI-017, AI-027, AI-051, AI-054, AI-055 | `deontic.rs` |
| 4 | Autonomy caps | 15 | AI-005, AI-019, AI-021, AI-022, AI-025, AI-028, AI-029, AI-030, AC-001, AC-002, AG-005, AG-006, AG-007 | `autonomy.rs` |
| 5 | Confidence framework | 12 | AI-034, AI-035, AI-036, AI-037, AI-038, AG-004, AG-016 | `confidence.rs` |
| 6 | Due process runtime | 9 | G-002, G-006, G-007, G-010, G-016, G-017, G-018, AI-045 | `due_process.rs` |
| 7 | Pipeline execution | 8 | G-012, G-013, G-019, G-020, G-021, G-032, G-049 | `pipeline.rs` |
| 8 | Compensation | 7 | K-027, K-039, K-040, K-041, K-042 | `compensation.rs` |
| 9 | Delegation runtime | 5 | G-025, G-026 + existing G-rules with runtime delegation semantics | `delegation.rs` |
| 10 | Agent provenance + fallback | 17 | AI-006, AI-008, AI-033, AI-044, AI-047, AI-052, AI-053, AG-009, AI-057, AI-032, AI-039, AI-040 | `agent.rs`, `fallback.rs` |
| 11 | Crash recovery / durability | 7 | K-023, K-024, K-026, K-028, K-031, K-032, K-035 | `durability.rs` |
| 12 | DCR marking state | 5 | AG-001, AG-002, AG-003, AG-015, AG-016 | `dcr.rs` |
| 13 | Provenance completeness | 3 | K-018, AI-029, AI-048 | `provenance.rs` |
| 14 | Verification reports | 4 | VR-001, VR-002, AG-002, AG-015 | `verification.rs` |
| 15 | Processor conformance | 6 | AI-001, AI-002, AI-004, G-051, G-052, AI-050 | meta-rules, deferred |

### Batch corrections applied

- **Removed from T3 batches** (T2-xdoc lint rules, not runtime fixtures): G-024 (already lint-tested), G-027, G-053 (need T2 lint tests, separate agent). K-037 (already T2 lint). AG-010 (T2-ast, deferred -- needs FEL AST analysis).
- **Moved AI-044** (drift detection) from Confidence to Agent Provenance batch.
- **Moved AI-045** (independentFirst oversight ordering) from Confidence to Due Process batch.
- **Added orphaned rules to existing batches:** Deontic (+8: AI-010 through AI-016, AI-051), Compensation (+2: K-040, K-041), Confidence (+3: AI-035, AI-036, AI-037), Delegation (+2: G-025, G-026).
- **Created new batches:** Fallback chain (AI-032, AI-039, AI-040), Hold/resume lifecycle (G-030, G-054), Crash recovery/durability (7 rules), Provenance completeness (3 rules), Processor conformance (6 meta-rules, deferred), Verification reports (4 rules).

### Batch progress

- [x] **Batch 1: Cancel-siblings/fail-fast** -- 2 fixtures written (K-044, K-045), 2 ignored tests added. Both fail as expected (red). Kernel documents: `parallel-timer-scoping.json`, `timer-tolerance-violation.json`.
- [ ] Batch 2: Hold/resume lifecycle
- [ ] Batch 3: Deontic enforcement
- [ ] Batch 4: Autonomy caps
- [ ] Batch 5: Confidence framework
- [ ] Batch 6: Due process runtime
- [ ] Batch 7: Pipeline execution
- [ ] Batch 8: Compensation
- [ ] Batch 9: Delegation runtime
- [ ] Batch 10: Agent provenance + fallback
- [ ] Batch 11: Crash recovery / durability
- [ ] Batch 12: DCR marking state
- [ ] Batch 13: Provenance completeness
- [ ] Batch 14: Verification reports
- [ ] Batch 15: Processor conformance (deferred -- meta-rules)

### T2 rule tests (pre-Phase-4)

- [x] Write lint tests for 7 implemented-but-untested T2 rules: G-003, G-008, G-023, G-024, G-036, AI-026, AI-031. _(22 new tests in tier2_rules.rs: 3 per rule with clean, flagged, and skip-condition variants.)_
- [x] Implement + test K-010 (action assignTo must reference declared actor) and K-037 (fail-fast parallel must have error-tagged final state). _(New lint rules in tier2.rs + 7 tests.)_
- [x] Add `BusinessCalendar` and `NotificationTemplate` document kinds to wos-lint's `DocumentKind` enum and MARKERS list. _(Required for G-023 to detect business calendar sidecars.)_
- [ ] Remaining 3 unimplemented T2 rules: AG-010 (SMT subset restrictions), AG-012 (quantifier finite domains), AI-023 (agent-free completion path). All require complex analysis; deferred.
- [ ] T2 lint tests for G-027 and G-053 (removed from T3 Delegation batch -- these are cross-document lint rules, not runtime fixtures).

---

## Phase 5: Build engine capabilities (green)

Implement each capability in `wos-core` to make its fixture batch pass. Each is a new module. Order matches Phase 4 batch priority.

- [ ] Timer region scoping + tolerance validation in `eval.rs`. Unlocks 2 rules (K-044, K-045).
- [ ] Hold/resume lifecycle in `eval.rs`. Unlocks 2 rules (G-030, G-054).
- [ ] `deontic.rs` — enforcement ordering, null propagation by impact level, constraint composition, bypass. Unlocks 20 rules.
- [ ] `autonomy.rs` — cross-document cap computation, escalation/demotion state machine, calibration expiry. Unlocks 15 rules.
- [ ] `confidence.rs` — ConfidenceReport validation, decay, cumulative tracking, session pause. Unlocks 12 rules.
- [ ] `due_process.rs` — actor identity verification, independentFirst ordering, review sampling, separation of duties. Unlocks 9 rules (now includes AI-045).
- [ ] `pipeline.rs` — staged execution with gate results, weakest-link risk profile, rejection routing. Unlocks 8 rules.
- [ ] `compensation.rs` — compensation log, reverse execution, pivot step, nested scopes, `$compensation.complete`. Unlocks 7 rules.
- [ ] `delegation.rs` — delegation chain verification, provenance recording, sub-delegation depth. Unlocks 5 rules.
- [ ] `agent.rs` + `fallback.rs` — provenance field validation, actor type immutability, narrative tier labeling, drift detection, fallback chains. Unlocks 17 rules (now includes AI-044, AI-032, AI-039, AI-040).
- [ ] `durability.rs` — crash recovery, persist-before-advance, idempotency dedup, structured validation. Unlocks 7 rules. Hard to fixture-test; may need process-restart simulation.
- [ ] `dcr.rs` — marking state (included/executed/pending), relation evaluation, zone satisfaction. Unlocks 5 rules.
- [ ] `provenance.rs` — relationship change provenance, agent output provenance, provenance completeness checks. Unlocks 3 rules.
- [ ] `verification.rs` — verification report generation, counterexample rendering. Unlocks 4 rules.
- [ ] Processor conformance meta-rules (AI-001, AI-002, AI-004, G-051, G-052, AI-050). Deferred — these are meta-level constraints on processor behavior, not fixture-testable.
- [ ] `eval_mode.rs` — continuous evaluation mode (Runtime S10): timer-driven re-evaluation, convergence cap enforcement, mode switching between `event-driven` and `continuous`.
- [ ] `explain.rs` — explanation assembly algorithm (Runtime S9). Stubbed in Phase 3.4 but needs: Phase 4 fixtures exercising explanation output, and Phase 5 implementation producing structured explanations with rule-reference citations and authority ranking.

---

## Phase 6: Adapt wos-lint to typed models

After wos-core has typed models for kernel, governance, and AI documents:

- [ ] Add `wos-core` to `wos-lint/Cargo.toml` (required to use `Project` methods and typed models).
- [ ] Replace `serde_json::Value` walking in `rules/tier1.rs` (88 occurrences) and `rules/tier2.rs` (109 occurrences) with typed field access (`state.kind == StateKind::Final` instead of `state.get("type").and_then(Value::as_str) == Some("final")`).
- [ ] Replace `collect_kernel_tags` (called once in tier2.rs:92; result stored in `self.tags` and reused) with `Project::kernel_tags()` from wos-core. Note: `Project` methods exist but depend on typed models from Phase 3.1.
- [ ] Replace `collect_kernel_events`, `collect_kernel_case_fields` with `Project` methods.
- [ ] Deserialize governance and AI documents into typed models in the T2 cross-document checks.

---

## Phase 7: Documentation

Only after the things being documented are stable.

- [ ] **`wos-spec/README.md`** — add Runtime Companion, CaseInstance schema, update counts, add wos-core to architecture.
- [ ] **`context.md`** — add WOS section. _(CLAUDE.md: "Update when a new spec is added.")_
- [ ] **Main `README.md`** — add WOS to repository structure.
- [ ] **`filemap.json`** — add wos-spec directory and crates.
- [ ] **`wos-core/README.md`** — document typed model, evaluation algorithm, trait interfaces.
- [ ] **Review document Section 12** — add Runtime Companion, CaseInstance schema, correspondence metadata, 3 missing fixtures.
- [ ] **ADR-0057** — already updated (Alternative 3 reflects runtime spec was written).

---

## Orphaned requirements (no tests, no lint rules, no TODO tasks)

- [ ] **Runtime S13 security model** — normative requirements (access control, data isolation, audit of security events) with no coverage anywhere: no lint rules, no fixtures, no Phase 5 module.
- [ ] **5 untested T1 rules** — not mentioned in any phase. Identify which T1 rules lack tests and add to Phase 4 T2 batch or a dedicated T1 verification pass.

---

## Formspec Coprocessor gap

The most significant architectural gap: there is no specified **handoff protocol** between WOS tasks and Formspec forms. Needs to address:

- [ ] How `createTask` with a Formspec `contractRef` causes a form to be presented to an actor.
- [ ] How a completed Response flows back into WOS `caseFile`.
- [ ] How Response data maps to case file fields (normatively via Mapping DSL).
- [ ] Whether Response is validated before the workflow event fires.
- [ ] Respondent Ledger requirement for rights-impacting workflows.

Best addressed as a "Formspec Coprocessor" section in the Runtime Companion or a new sidecar spec. This is the missing piece that would make WOS + Formspec fully interoperable across implementations.

**Enterprise urgency:** The enterprise implementation roadmap (`enterprise-implementation-roadmap.md`) Phase 1.3 (Case Management Baseline) depends on this handoff protocol. Without it, the SaaS platform must design its own submission-to-case bridge, which risks diverging from WOS semantics. WOS governance specs (review protocols, due process, structured audit, delegation, AI agent governance) cover most of the enterprise roadmap's case management requirements at the spec level -- but the Coprocessor gap is the seam that connects Formspec intake to WOS workflow. Resolving this gap unblocks the enterprise roadmap's ability to leverage WOS specs rather than building governance from scratch.

---

## Formspec dependencies (external, not sequenced above)

- [ ] **FEL `every`, `some`, `duration` built-ins** — WOS specs reference these. Not in Formspec codebase. Fallback: extension functions (Core S3.12). _Revisit when: Formspec Core spec freeze._
- [ ] **FEL array-of-record evaluation** — WOS specs use `every(list, 'field != value')` with a string predicate, but FEL's `countWhere` uses a boolean expression with `$` context variable. The calling conventions are incompatible: either WOS examples need rewriting to use `countWhere`-style `$` references, or FEL needs `every`/`some` with their own convention. Required for clinical and equity scenarios. _Revisit when: FEL grammar revision._

---

## Missing sidecars (referenced but never created)

- [x] **Business Calendar** — referenced by Governance S10.3, S13.3. Defines business days, holidays, operating hours. _Created: spec (`specs/sidecars/business-calendar.md`), schema (`schemas/wos-business-calendar.schema.json`), fixture (`fixtures/sidecars/benefits-business-calendar.json`), Rust model (`crates/wos-core/src/model/business_calendar.rs`), deser tests (`crates/wos-core/tests/business_calendar_deser.rs`). LINT-MATRIX rules G-058 through G-061._
- [x] **Notification Template** — referenced by Governance S12.2, S3.1. Defines templates for hold/adverse/appeal notices. _Created: spec (`specs/sidecars/notification-template.md`), schema (`schemas/wos-notification-template.schema.json`), fixture (`fixtures/sidecars/benefits-notification-templates.json`), Rust model (`crates/wos-core/src/model/notification_template.rs`), deser tests (`crates/wos-core/tests/notification_template_deser.rs`). LINT-MATRIX rules G-062 through G-065._

---

## Future specs (trigger-gated, no timeline)

- [ ] **Federation Profile** — cross-org trust, signed provenance, data sovereignty. _When: a second organization adopts WOS for cross-boundary workflows._
- [ ] **Learning Profile** — drift feedback loops, retraining governance. _When: a production deployment runs AI agents long enough to need retraining governance._
- [ ] **Review Config sidecar** — reusable review protocols across governance documents. _When: two governance documents need the same protocols._

---

## Deferred (no near-term plan)

| Item | Status | Revisit when |
| ---- | ------ | ------------ |
| Evidence model sidecar | Not started | A deployment needs typed doc refs with integrity hashes |
| Merkle tree tamper evidence | Not started | An auditor requires cryptographic provenance integrity proof |
| Multi-agent delegation chain | Partial (Gov S11) | An AI agent delegates to another AI agent |
| Full lifecycle soundness verification | Partial (Adv S8) | A deployment needs deadlock-freedom or termination proofs |
| Simulation trace format | Not started | Conformance testing needs replay beyond fixture assertions |
| FEEL-to-FEL translation | Not started | An org migrating from DMN/FEEL needs a translation guide |
| Patch operation reference | Not started | A deployment needs JSON Patch for case state mutations |
| Full JSON-LD/SPARQL | Partial (Semantic S3) | A deployment needs RDF graph queries over WOS documents |

---

## ADR-0058 disposition

| Construct | Disposition | Delivered |
| --------- | ----------- | --------- |
| 1A. Case Linking | Done (modified) | Kernel S5.5 — metadata-only, correlationKey for behavior |
| 1B. Regulatory Effective Dating | Done | Policy Parameters S1.2-S1.5 |
| 1C. Delegation of Authority | Done | Governance S11 |
| 1D. Review Cycles | Rejected | Expressible with existing statecharts. Medicaid fixture proves it. |
| 2A. Batch Operations | Rejected | Implementation concern |
| 2B. Correspondence Events | Done (modified) | Correspondence Metadata sidecar |
| 2C. Typed Hold Reasons | Done | Governance S12 |
