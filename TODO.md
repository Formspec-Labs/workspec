# WOS TODO

**Last audited:** 2026-04-11
**Counts:** 18 specs, 18 schemas, 39 document fixtures + 95 conformance fixtures (0 T3 red, 99 green), 3 crates, 189 lint rules (87 tested)

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
- [x] `explain.rs` — `Explanation`, `ReasoningRecord`, `CounterfactualRecord` types + `assemble_explanation()`. Full assembly implemented in Phase 5.

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
- **Timer testing:** Use the "noop-with-delay" pattern: `{ "event": "noop", "delay": "PT11S" }` advances simulated time without triggering a real event. The conformance engine calls `advance_time(ms)` which fires any timers whose deadlines have passed.

### Batch table (priority order)

| # | Batch | Rules | Rule IDs | Engine dep |
|---|-------|-------|----------|------------|
| 1 | Cancel-siblings/fail-fast | 2 | K-044, K-045 | `eval.rs` |
| 2 | Hold/resume lifecycle | 2 | G-030, G-054 | `eval.rs` |
| 3 | Deontic enforcement | 13 | AI-009, AI-010, AI-011, AI-012, AI-013, AI-014, AI-015, AI-016, AI-017, AI-027, AI-051, AI-054, AI-055 | `deontic.rs` |
| 4 | Autonomy caps | 13 | AI-005, AI-019, AI-021, AI-022, AI-025, AI-028, AI-029, AI-030, AC-001, AC-002, AG-005, AG-006, AG-007 | `autonomy.rs` |
| 5 | Confidence framework | 7 | AI-034, AI-035, AI-036, AI-037, AI-038, AG-004, AG-016 | `confidence.rs` |
| 6 | Due process runtime | 8 | G-002, G-006, G-007, G-010, G-016, G-017, G-018, AI-045 | `due_process.rs` |
| 7 | Pipeline execution | 7 | G-012, G-013, G-019, G-020, G-021, G-032, G-049 | `pipeline.rs` |
| 8 | Compensation | 5 | K-027, K-039, K-040, K-041, K-042 | `compensation.rs` |
| 9 | Delegation runtime | 2 | G-025, G-026 | `delegation.rs` |
| 10 | Agent provenance + fallback | 12 | AI-006, AI-008, AI-033, AI-044, AI-047, AI-052, AI-053, AG-009, AI-057, AI-032, AI-039, AI-040 | `agent.rs`, `fallback.rs` |
| 11 | Crash recovery / durability | 7 | K-023, K-024, K-026, K-028, K-031, K-032, K-035 | `durability.rs` |
| 12 | DCR marking state | 3 | AG-001, AG-002, AG-003 | `dcr.rs` |
| 13 | Provenance completeness | 2 | K-018, AI-048 | `provenance.rs` |
| 14 | Verification reports | 3 | VR-001, VR-002, AG-015 | `verification.rs` |
| 15 | Sidecar runtime | 2 | G-061, G-064 | `sidecar.rs` |
| 16 | Processor conformance | 6 | AI-001, AI-002, AI-004, G-051, G-052, AI-050 | meta-rules, deferred |

**Deduplication notes:** AG-002 (DCR only), AG-015 (Verification only), AG-016 (Confidence only), AI-029 (Autonomy only). Each rule appears in exactly one batch.

### Batch progress

- [x] **Batch 1: Cancel-siblings/fail-fast** -- 2 fixtures (K-044, K-045), 2 ignored tests. Kernels: `parallel-timer-scoping.json`, `timer-tolerance-violation.json`.
- [x] **Batch 2: Hold/resume lifecycle** -- 2 fixtures (G-030, G-054), 2 ignored tests. Kernels: `hold-resume-lifecycle.json`. Governance: `hold-resume-governance.json`.
- [x] **Batch 3: Deontic enforcement** -- 13 fixtures (AI-009 through AI-017, AI-027, AI-051, AI-054, AI-055), 13 ignored tests. Kernels: `deontic-enforcement.json`. AI: `deontic-enforcement-ai.json`.
- [x] **Batch 4: Autonomy caps** -- 13 fixtures (AI-005, AI-019, AI-021, AI-022, AI-025, AI-028, AI-029, AI-030, AC-001, AC-002, AG-005, AG-006, AG-007), 13 ignored tests. Kernels: `autonomy-caps.json`. AI: `autonomy-caps-ai.json`.
- [x] **Batch 5: Confidence framework** -- 7 fixtures (AI-034 through AI-038, AG-004, AG-016), 7 ignored tests. Reuses deontic kernel/AI documents.
- [x] **Batch 6: Due process runtime** -- 8 fixtures (G-002, G-006, G-007, G-010, G-016, G-017, G-018, AI-045), 8 ignored tests. Kernels: `due-process.json`. Governance: `due-process-governance.json`.
- [x] **Batch 7: Pipeline execution** -- 7 fixtures (G-012, G-013, G-019, G-020, G-021, G-032, G-049), 7 ignored tests. Kernels: `pipeline-execution.json`.
- [x] **Batch 8: Compensation** -- 5 fixtures (K-027, K-039, K-040, K-041, K-042), 5 ignored tests. Kernels: `compensation.json`.
- [x] **Batch 9: Delegation runtime** -- 2 fixtures (G-025, G-026), 2 ignored tests. Reuses due-process kernel/governance.
- [x] **Batch 10: Agent provenance + fallback** -- 12 fixtures (AI-006, AI-008, AI-033, AI-044, AI-047, AI-052, AI-053, AG-009, AI-057, AI-032, AI-039, AI-040), 12 ignored tests. Reuses deontic kernel/AI documents.
- [x] **Batch 11: Crash recovery / durability** -- 7 fixtures (K-023, K-024, K-026, K-028, K-031, K-032, K-035), 7 ignored tests. Kernels: `durability.json`. Uses `$restart` and `$migrate` synthetic events for crash/migration simulation. K-023 (crash recovery) and K-024 (persist-before-advance) test closest observable behavior; true architectural guarantees require process-restart simulation beyond fixture scope. Note: K-031 (structured contract validation results) and K-032 (lifecycle/case state separation) are architectural invariants, not crash recovery — they share the durability kernel fixture but test fundamentally different concerns.
- [x] **Batch 12: DCR marking state** -- 3 fixtures (AG-001, AG-002, AG-003), 3 ignored tests. Kernels: `dcr-zone.json`. Advanced: `dcr-zone-governance.json`.
- [x] **Batch 13: Provenance completeness** -- 2 fixtures (K-018, AI-048), 2 ignored tests. K-018 tests relationship change provenance via synthetic `relationshipChanged` event. AI-048 tests narrative-as-dispositive detection.
- [x] **Batch 14: Verification reports** -- 3 fixtures (VR-001, VR-002, AG-015), 3 ignored tests. Tests immutability violation detection and proven-unsafe activation blocking. AG-015 tests AdvGov lifecycle gating path; VR-002 tests the same semantics from the VerifReport document side.
- [x] **Batch 15: Sidecar runtime** -- 2 fixtures (G-061, G-064), 2 ignored tests. G-061 tests expired calendar fallback. G-064 tests notification suppression when required variables missing.
- [ ] Batch 16: Processor conformance (deferred -- meta-rules: AI-001, AI-002, AI-004, G-051, G-052, AI-050). These are processor-level conformance claims, not fixture-testable behaviors. Verification strategy: processor self-declaration + conformance profile matching.

**Phase 4 totals:** 88 T3 conformance fixtures written, 88 ignored tests across 15 batches. Batch 16 (processor conformance meta-rules) deferred -- not fixture-testable.

### T2 rule tests (pre-Phase-4)

- [x] Write lint tests for 7 implemented-but-untested T2 rules: G-003, G-008, G-023, G-024, G-036, AI-026, AI-031. _(22 new tests in tier2_rules.rs: 3 per rule with clean, flagged, and skip-condition variants.)_
- [x] Implement + test K-010 (action assignTo must reference declared actor) and K-037 (fail-fast parallel must have error-tagged final state). _(New lint rules in tier2.rs + 7 tests.)_
- [x] Add `BusinessCalendar` and `NotificationTemplate` document kinds to wos-lint's `DocumentKind` enum and MARKERS list. _(Required for G-023 to detect business calendar sidecars.)_
- [x] **AG-010** (SMT subset restrictions): Already implemented -- `check_smt_expression()` in `fel_analysis.rs` dispatches to AG-011, AG-012, AG-013, AG-014. Checks AdvGov S8.2 restrictions 1 (no recursion), 2 (quantifier presence), 3 (linear arithmetic), 5 (no extension functions). Restriction 4 (finite domain enumerations) requires type knowledge unavailable at lint time. Restriction 6 (no filter expressions) cannot be violated -- the FEL AST has no `Filter` path segment variant. Tests in `fel_analysis.rs` `#[cfg(test)]` module.
- [x] **AG-012** (quantifier finite domains): Partial implementation -- `check_finite_quantifiers()` flags `every()` and `some()` calls with a warning indicating manual review required. Full finiteness verification requires type/ontology knowledge that lint does not have. _(3 new tests in tier2_rules.rs: quantifier flagged, clean, `some` variant.)_
- [x] **AI-023** (agent-free completion path): Implemented -- `check_agent_free_completion_path()` in `tier2.rs`. Collects agent IDs from AI doc, builds a flat lifecycle graph from kernel states, performs BFS from initial state to final states excluding agent-only states. Emits warning when no agent-free path exists. Supports both array and object agent formats. _(3 new tests in tier2_rules.rs: path exists clean, all-agent flagged, no-AI-doc skip.)_
- [x] T2 lint tests for G-027 and G-053 (removed from T3 Delegation batch -- these are cross-document lint rules, not runtime fixtures). 3 tests each (flagged, clean, skip-condition). Schema fix: added `allowsSubDelegation` to Delegation definition.

---

## Phase 5: Build engine capabilities (green)

Implement each capability in `wos-core` to make its fixture batch pass. Each is a new module. Order matches Phase 4 batch priority.

- [x] Timer region scoping + tolerance validation in `eval.rs`. Unlocks 2 rules (K-044, K-045). _Also fixed provenance partial matching: `record_kind` (snake_case fixture) -> `recordKind` (serde camelCase). Also added `ToleranceViolation` provenance kind, tolerance tier calculation, and duration/duration_ms to Timer struct._
- [x] Hold/resume lifecycle in `eval.rs`. Unlocks 2 rules (G-030, G-054). _Already worked — only blocked by provenance matching bug fixed in Batch 1. K-042 (compensation complete event) also unblocked._
- [x] `deontic.rs` — enforcement ordering, null propagation by impact level, constraint composition, bypass. Unlocks 13 rules. _New module: FEL-based constraint evaluation against agent output. Conformance engine now loads AI documents and runs deontic evaluation on agent events. Enforcement actions redirect lifecycle events (escalate -> "escalated", reject -> block). Null propagation: permissions escalate for rights-impacting; prohibitions default to pass; obligations treat null as violated. Bypass applies to all constraints in scope. Resolution tracks cross-level vs same-level violations. Invocation source passed through for assist proxy tests._
- [x] `autonomy.rs` — cross-document cap computation, escalation/demotion state machine, calibration expiry. Unlocks 13 rules. _New module: impact-level autonomy cap, 4-source minimum computation, human override protection, assistive task creation, escalation/demotion, tool governance (permitted list, rate limits, direct case write), calibration expiry._
- [x] `confidence.rs` — ConfidenceReport validation, decay, cumulative tracking, session pause. Unlocks 7 rules. _New module: missing report detection, uncalibrated score flagging, floor enforcement with escalation, decay factor application, cumulative confidence monitoring, session checkpoint pause, ground-truth labels._
- [x] `event_handler.rs` — unified handler for Batches 6-15. Unlocks 50 rules across due process, pipeline, compensation, delegation, agent provenance, fallback, durability, DCR, verification, and sidecar. _Single module reading event data + governance docs + kernel lifecycle structure. DCR zone satisfaction with relation evaluation and resolution error detection. Compensation reverse ordering from lifecycle state history. Post-execution provenance from kernel structure (contracts, history, service keys)._
- [x] **Phase 5 code review fixes (11 findings).** Deontic bypass now emits provenance for prohibitions and obligations (not just permissions). G-006/G-017 use structured boolean fields instead of string matching. K-026 idempotency dedup tracks seen keys (first occurrence proceeds, second+ emits dedup). Agent provenance uses fallback pattern instead of fragile negative-list check. AI-054 fixture verifies bypass does not persist across invocations. AI-016 fixture description corrected (same-level, not cross-level). Consistency threshold constants extracted with spec-reference documentation. Confidence fixtures fixed (output now includes confidenceReport so obligation passes). wos-lint indexmap dependency added.
- [ ] Processor conformance meta-rules (AI-001, AI-002, AI-004, G-051, G-052, AI-050). Deferred — these are meta-level constraints on processor behavior, not fixture-testable.
- [x] `eval_mode.rs` — continuous evaluation mode (Runtime S10): timer-driven re-evaluation, convergence cap enforcement, mode switching between `event-driven` and `continuous`. _New module: `continuous_reevaluate()` function that re-evaluates `$continuous` transitions after case state mutations. Convergence cap of 100 cycles with provenance recording. Mode-aware: skips entirely for event-driven kernels. Unit tests cover guard satisfaction, event-driven skip, and convergence cap halt._
- [x] `explain.rs` — explanation assembly algorithm (Runtime S9). _Full implementation replacing stub: filters provenance by `tier` (reasoning/counterfactual) and `relatedTransition`, separates positive/negative counterfactuals, sorts reasoning by authority rank (statute=1 > regulation=2 > policy=3 > guideline=4) then chronologically. Unspecified authority defaults to policy rank per spec. Unit tests cover empty assembly, authority ordering, chronological tie-breaking, counterfactual separation, and full end-to-end assembly._

---

## Phase 6: Adapt wos-lint to typed models

After wos-core has typed models for kernel, governance, and AI documents:

- [x] Add `wos-core` to `wos-lint/Cargo.toml` (required to use `Project` methods and typed models). _(already present from prior phase)_
- [x] Replace `serde_json::Value` walking in `rules/tier1.rs` and `rules/tier2.rs` with typed field access. Tier1 already had typed/fallback pattern; tier2 now deserializes kernel into `KernelDocument` and uses typed access for `KernelCollections` (tags, events, case_fields, actor_ids), K-010 (actor refs), K-037 (fail-fast), G-034 (targetWorkflow match), G-001/G-003/G-004/G-005/G-015 (impact-level checks), G-028 (hold policies), G-046 (delegation actors), AI-046 (disclosure), AG-017 (shadow mode), DM-002 (deployment sequence). All checks retain Value-based fallback paths for partial fixtures.
- [x] Replace `collect_kernel_tags` with typed state-tree walk in `KernelCollections::from_typed()`. Equivalent to `Project::kernel_tags()` logic operating directly on the deserialized `KernelDocument`.
- [x] Replace `collect_kernel_events`, `collect_kernel_case_fields` with typed equivalents in `KernelCollections::from_typed()`.
- [x] Deserialize governance documents into typed models in T2 cross-document checks. Used for G-027 (sub-delegation depth via `GovernanceDocument.delegations` and `max_delegation_depth`). AI document typed deserialization attempted but deferred for most agent checks — the `AgentDeclaration` typed model does not yet include per-agent fields (`autonomy`, `reviewWindow`, `escalationRules`, `modelConfig`, etc.) that AI lint rules require.

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

## Phase 8: Lint test coverage

Complete test coverage for existing lint rules. No new rules, just tests.

- [ ] **T1-TESTS: Write 21 tests for 7 untested T1 rules** (3 per rule: flagged, clean, skip) in `tier1_rules.rs`.
  - K-009 (Kernel S3.3): Actor identifiers MUST be unique. Category: `actor-consistency`.
  - K-021 (Kernel S8.2): Provenance `actorId` MUST reference a declared actor. Category: `actor-consistency`.
  - G-058 (BC S3.3): Holiday entry MUST specify exactly one of `date` or `rule`. Category: `calendar-validity`.
  - G-059 (BC S5.2): Operating hours `end` MUST be strictly after `start`. Category: `calendar-validity`.
  - G-062 (NT S4.4): Adverse-decision templates MUST include determination, reason codes, appeal rights, appeal instructions. Category: `notification-validity`.
  - G-065 (NT S4.1): Section `id` values MUST be unique within a template. Category: `notification-validity`.
  - AI-003 (AI S2.2): Processor MUST validate fallback chains at load time, rejecting cycles or missing terminal actions. Category: `fallback-chain-validity`.
  - **Context:** All 7 rules are already implemented in `wos-lint/src/rules/tier1.rs`. The task is to add test cases only — no logic changes. Follow the existing test patterns in `tier1_rules.rs`.

---

## Phase 9: Conformance profiles

Formalize the relationship between individual rule tests and processor conformance levels.

- [ ] **PROFILE-GOV: Create Governance conformance profile tests.** New file: `profile_conformance.rs` (or section in `kernel_conformance.rs`).
  - G-051 (Governance Basic): Aggregate test collecting all fixtures for G-002, G-006, G-007, G-010, G-016, G-017, G-018. Passes iff all pass.
  - G-052 (Governance Complete): Aggregate test collecting ALL G-* fixtures. Passes iff all pass.
  - **Context:** These rules are profile-level claims, not individual behaviors. The fixture naming convention (`{rule-id}-{short-description}.json`) makes globbing straightforward. Each profile test discovers and runs all matching fixtures.

- [ ] **PROFILE-AI: Create AI conformance profile tests.**
  - AI-001 (Agent Registration): Satisfied by all agent-involving fixtures (Batches 3, 4, 5, 10). Aggregate test.
  - AI-002 (Confidence Framework): Satisfied by Batch 5 fixtures (AI-034 through AI-038, AG-004, AG-016). Aggregate test.
  - **Context:** These are evidence-by-aggregation claims. Formalizing them as profile tests ensures they stay satisfied as the fixture set evolves.

---

## Phase 10: SMT verifiable subset (AG-010 enhancements)

Strengthen the lint-time analysis for AdvGov verifiable constraints.

- [ ] **AG010-FINITE: Implement `check_finite_domain_equality` in `fel_analysis.rs`.**
  - Wire into `check_smt_expression` (the existing AG-010 dispatcher).
  - Decidable patterns (pass silently): literal comparisons (`$x.status == "approved"`), boolean comparisons, `in` expressions with array containers (`$x in ["a","b","c"]`), known WOS enum paths (`instance.impactLevel`).
  - Ambiguous pattern (emit warning): variable-to-variable equality (`$x.field == $y.other`).
  - Tests: literal clean, boolean clean, `in` clean, enum path clean, variable-to-variable flagged.
  - **Context:** Implements AG-010 restriction 4 (finite domain enumerations) at lint time using AST-only analysis. No type registry needed for common cases. File: `wos-lint/src/rules/fel_analysis.rs`.

- [ ] **AG010-DECL: Add `finiteDomainDeclarations` to `VerifiableConstraint` schema.**
  - Add to `wos-advanced.schema.json` under the `VerifiableConstraint` definition.
  - Format: `{ "path": { "domain": ["value1", "value2", ...] } }`.
  - Update `check_finite_domain_equality` to load declarations and suppress variable-to-variable warnings when a declaration covers one side of the equality.
  - Tests: declaration resolves warning, declaration missing still warns, invalid declaration ignored.
  - **Context:** Allows constraint authors to declare finite domains without a full Type Registry. Step 2 of the three-step path.

- [ ] **AG010-FILTER: Verify FEL parser rejects filter expressions (restriction 6).**
  - Check that `PathSegment` enum has no `Filter` variant and the parser rejects `$path[?(@.x > 1)]` syntax.
  - If confirmed: document in LINT-MATRIX as "enforced by parser — no lint rule needed."
  - If parser accepts them: add `check_no_filter_expressions` to `fel_analysis.rs`.
  - **Context:** The FEL AST's `PathSegment` enum has `Dot`, `Index`, `Wildcard` — no `Filter(Expr)`. Likely enforced by construction, but needs explicit verification.

---

## Phase 11: Formspec coprocessor protocol

The most significant architectural gap: no specified handoff protocol between WOS tasks and Formspec forms. Enterprise implementation Phase 1.3 (Case Management Baseline) depends on this.

**Sequencing:** Gap 2 → Gap 3 → Gap 1 (functions first, then calling convention, then coprocessor examples).

- [ ] **FEL-QUANTIFIERS: Add `every`, `some`, `duration` built-in functions to Formspec.**
  - Preferred path: Add to Formspec Core S3.5 as "Quantifier Functions" (universal applicability, stays in AdvGov S8.2 verifiable subset).
  - Fallback: Register as WOS extension functions (Core S3.12) — but restriction 5 excludes extensions from verifiable constraints.
  - `duration` returns ms. Verify consistency with `timeDiff` (returns seconds per spec review).
  - **Context:** WOS specs reference these functions but they don't exist in Formspec. Required for clinical/equity scenario FEL expressions. Affects `fel-core` (Rust), `src/formspec/fel/` (Python), and `packages/formspec-engine/` (WASM bridge).

- [ ] **FEL-RECORDS: Resolve FEL array-of-record evaluation semantics.**
  - Problem: WOS uses `every(list, 'field != value')` with string predicates, but FEL's `$` is a scalar context variable.
  - Proposed: Extend `$` semantics — when element is a record, `$.propertyName` resolves to named property.
  - Unresolved issues: (a) This IS a grammar change (S3.7), not purely semantic. (b) Collision with `constraint` bind `$` (where `$` is already the field value, could be object) not addressed. (c) WOS examples need rewriting from string predicates to FEL expressions.
  - **Context:** Requires a Formspec Core spec revision. Must be resolved before coprocessor examples can reference quantifiers over case data.

- [ ] **COPROCESSOR: Author Runtime Companion S15 (Formspec coprocessor protocol).**
  - Resolve 8 review findings from `thoughts/specs/2026-04-10-formspec-integration-gaps.md` before writing normative prose:
    1. (Critical) Separate Respondent Ledger from adverse-decision notice delivery — different concerns.
    2. (Significant) Clarify `ContractValidator` input: `response.data` vs full Response.
    3. (Significant) Declare rejection policy home for coprocessor validation gate. Failed validation → `failed` state (not `claimed`).
    4. (Moderate) Add `contractRef` to Kernel S9.2 `createTask` table. Formally define `ContractReference` in prose or cite schema.
    5. (Moderate) Clarify triggering condition for coprocessor validation (governance-level hook vs per-task).
    6. (Minor) Use `direction: "both"` for prefill mapping — spec-correct answer, resolve now.
    7. (Minor) Error handling when Formspec processor unavailable during `submitTaskResponse`.
    8. (Minor) Authentication of `actorId` against `assignedActor`.
  - Deliverables: spec prose (S15), schema additions (`FormspecTaskContext`, `submitTaskResponse` operation), `TaskPresenter` host interface (S12.9), `responseMappingRef` on `ContractReference`.
  - **Context:** Design spec at `thoughts/specs/2026-04-10-formspec-integration-gaps.md`. Depends on FEL-QUANTIFIERS and FEL-RECORDS.

---

## Phase 12: Security and conformance documentation

Document architectural constraints that can't be lint-checked or fixture-tested.

- [ ] **SECURITY-PROFILE: Write conformance profile guidance for Runtime S13.**
  - S13.1 (Engine Isolation): Engine MUST NOT have direct network access; all external comms through `ExternalService` (S12.4). Architectural constraint — verified by code review, not lintable.
  - S13.2 (Expression Sandboxing): Informative. FEL provides this by construction (no I/O, no side effects).
  - S13.3 (Data Protection): Host SHOULD encrypt at rest via `InstanceStore`. Host-level concern.
  - S13.4 (Provenance Immutability): Host SHOULD implement write-once append-only storage. Explicitly SHOULD (not MUST) due to expungement requirements.
  - Deliverable: A "Conformance Profile" section in `wos-core/README.md` or a standalone `CONFORMANCE.md` explaining what S13 requires and how a host demonstrates compliance.
  - **Context:** Only S13.1 is a hard MUST. The SHOULDs are host-level deployment concerns, not engine-enforced.

- [ ] **ARCH-AI004: Document AI-004 / AI-050 verification strategy.**
  - AI-004 (delegate Formspec evaluation to conformant processor): Cannot be fixture-tested without Formspec processor instrumentation or differential testing.
  - AI-050 (proxy must not modify conformance requirements): Partially addressable via differential proxy fixtures.
  - Deliverable: A section in conformance profile guidance explaining how these are verified (self-declaration + conformance profile matching). Becomes testable when a second processor or proxy implementation exists.
  - **Context:** These are the only genuinely untestable rules. They require inter-processor trust, not engine behavior.

---

## Future specs (trigger-gated, no timeline)

| Spec | Description | Trigger |
|------|-------------|---------|
| Batch Operations | Parallel case instantiation, bulk state transitions, aggregate provenance. Originally rejected as "implementation concern" (ADR-0058 2A) but enterprise scale demands it at the spec level — without it every deployer invents their own batching semantics with incompatible provenance models. | A deployment processes >100 cases/minute or needs deterministic bulk replay |
| Federation Profile | Cross-org trust, signed provenance, data sovereignty | A second organization adopts WOS for cross-boundary workflows |
| Learning Profile | Drift feedback loops, retraining governance | A production deployment runs AI agents long enough to need retraining governance |
| Review Config sidecar | Reusable review protocols across governance documents | Two governance documents need the same protocols |

---

## Deferred (sorted by impact)

| Item | Status | Value | Trigger |
|------|--------|-------|---------|
| Full lifecycle soundness verification | Partial (Adv S8) | Proves no deadlocks, no unreachable states, guarantees termination. The difference between "we think it works" and "we can prove it works" — critical for rights-impacting workflows where stuck cases mean denied benefits. | A deployment needs deadlock-freedom or termination proofs |
| Multi-agent delegation chain | Partial (Gov S11) | Unlocks agent-to-agent orchestration — the entire "AI managing AI" pattern. Without it, every multi-agent workflow needs a human broker. Multiplier on autonomy value. | An AI agent delegates to another AI agent |
| Federation Profile (impl) | Not started | Cross-org WOS workflows: shared provenance, trust boundaries, data sovereignty. The network-effect unlock — WOS becomes an interop standard, not just an internal tool. | A second organization adopts WOS for cross-boundary workflows |
| Full Type Registry + Formspec bridge | Not started | Eliminates all manual `finiteDomainDeclarations` — automated finite-domain resolution from Formspec Definitions through contract references. Makes AG-010 restriction 4 zero-config. | A deployment enables AdvGov constraint verification beyond author annotations |
| Evidence model sidecar | Not started | Typed document references with integrity hashes. Enables tamper-evident audit trails without full Merkle complexity. The minimal viable "prove nothing was altered" primitive. | A deployment needs typed doc refs with integrity hashes |
| Simulation trace format | Not started | Replay-based conformance testing and "what-if" analysis. Fixtures prove individual rules; simulation traces prove end-to-end workflow correctness under realistic event sequences. | Conformance testing needs replay beyond fixture assertions |
| Merkle tree tamper evidence | Not started | Cryptographic integrity proof over the full provenance chain. Upgrades "append-only by policy" to "append-only by math." Required for adversarial audit environments. | An auditor requires cryptographic provenance integrity proof |
| Patch operation reference | Not started | Structured case state mutations via JSON Patch. Enables fine-grained provenance ("field X changed from A to B") instead of full-state snapshots. Reduces storage, improves diffing. | A deployment needs JSON Patch for case state mutations |
| Full JSON-LD/SPARQL | Partial (Semantic S3) | RDF graph queries over WOS documents. Enables cross-document semantic queries ("find all cases where agent X made a decision that was later overturned"). Power tool for compliance analysis. | A deployment needs RDF graph queries over WOS documents |
| FEEL-to-FEL translation | Not started | Migration guide for orgs using DMN/FEEL. Lowers adoption barrier for the largest existing market of workflow-automation users. | An org migrating from DMN/FEEL needs a translation guide |

---

## ADR-0058 disposition

All constructs from the original gap analysis are resolved:

| # | Construct | Outcome |
|---|-----------|---------|
| 1A | Case Linking | **Shipped.** Kernel S5.5 — metadata-only, `correlationKey` for behavior. |
| 1B | Regulatory Effective Dating | **Shipped.** Policy Parameters S1.2-S1.5. |
| 1C | Delegation of Authority | **Shipped.** Governance S11. |
| 1D | Review Cycles | **Not needed.** Expressible with existing statecharts. Medicaid fixture proves it. |
| 2A | Batch Operations | **Reopened.** Moved to Future Specs — enterprise scale demands spec-level batching semantics. |
| 2B | Correspondence Events | **Shipped** (modified). Correspondence Metadata sidecar. |
| 2C | Typed Hold Reasons | **Shipped.** Governance S12. |
