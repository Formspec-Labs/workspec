# WOS TODO

**Last audited:** 2026-04-16
**Counts:** 18 specs, 18 schemas, 41 document fixtures + 146 conformance fixtures (0 T3 red, 146 green), 6 crates, 197 lint rules (197 tested, 0 untested)

**Links:** [Core extraction plan](../thoughts/plans/2026-04-10-wos-core-extraction.md) (complete) · [Runtime plan](../thoughts/plans/2026-04-13-wos-runtime-crate.md) (complete) · [§1 plan](thoughts/plans/2026-04-14-wos-spec-section-1-implementation.md) (complete) · [LINT-MATRIX](LINT-MATRIX.md) · [Runtime Companion](specs/companions/runtime.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md) · [IDEA_SCRATCH](IDEA_SCRATCH.md) · [POSITIONING](POSITIONING.md) · [CONVENTIONS](CONVENTIONS.md)

**Missing / unknown references:** `ADR-0058 (wos-core-gap-analysis)` and `ADR-0057 (wos-core-implementation-boundary)` were cited from prior headers but do not exist at `../thoughts/adr/`. Status: unknown — either never authored, relocated, or inlined into other material. Resolve before next audit.

**Sequencing logic:**

- §2 foundational — provenance export landed; ontology awaiting design.
- §3 engine adapters — open sequencing question.
- §4 schema/spec closures and §5 behavioral specs are the bulk of active work, absorbed from IDEA_SCRATCH during the 2026-04-16 merge. These were previously tracked only in IDEA_SCRATCH.
- §6 audit/evidence products build on the shipped export surface.
- §7 regulatory alignment is external-deadline-driven.
- §8 interop is last.

**Scoring metadata.** Items absorbed from IDEA_SCRATCH carry `[Imp/Cx/Debt]` scores preserved from the 2026-04-16 audit. The Urgency formula from IDEA is not reproduced — re-scoring deferred until after the merge.

---

## 1 — Reference implementation blockers

> §1 closed 2026-04-14 — see Completed.

---

## 2 — Foundational (zero external dependencies)

- [x] **Provenance export** — Serialize internal provenance to W3C PROV-O, OCEL 2.0, IEEE 1849 XES. Landed 2026-04-15 — see Completed.
- [ ] **Ontology field identity** *(design not started — do not sequence as active work)* — `ontology-spec.md` does not exist. Informs AI integration, cross-document alignment, and regulatory specs in §7, but cannot be scheduled until the spec is drafted. Prerequisite design work: JSON-LD `@context` decision (see Deferred #9), semantic-field-identity protocol, cross-document alignment mechanism. Move to active only once a draft exists.

---

## 3 — Engine adapters (open question — sequencing unresolved)

> **Status:** sequencing unresolved. TODO previously placed engine adapters as a near-term priority; IDEA_SCRATCH #49 marked them Defer with trigger "first commercial deployment requesting a specific adapter." No arbitrating document. Items kept in the backlog below but **not** scheduled until this question is resolved.

Validate the runtime against real commercial workflow engines.

- [ ] **#49 Camunda 8 Worker** — Delegate BPMN task execution under WOS governance. Most common BPMN target; broadest external fixture diversity.
- [ ] **#49 Temporal Workflow** — Map WOS evaluation steps to deterministic replay. Natural fit with WOS evaluator determinism.
- [ ] **#49 AWS Step Functions** — Bridge ASL states to WOS transitions. Broadest commercial reach; narrowest semantic fit.

---

## 4 — Schema / spec closures (lock-in reducers)

Items that tighten schemas, close openness, or reduce architectural lock-in. Absorbed from IDEA_SCRATCH Adopt list on 2026-04-16.

- [ ] **#20 Typed event meta-vocabulary** `[Imp 8 / Cx 6 / Debt 6]` — Replace `Transition.event: string` with strict 5-kind typed union `{ kind: "timer" | "message" | "signal" | "condition" | "error", ... }`. No `named` wrapper; no escape hatch. Co-type `Action.event` for `startTimer`. Closes kernel's last load-bearing openness (`wos-kernel.schema.json:348`). Affects 16+ kernel fixtures; promotes lint K-007 to schema validation. **Blocked on DRAFTS triage** (12 competing kernel drafts).
- [ ] **#21 Extension registry (seams-only MVP)** `[Imp 5 / Cx 4 / Debt 3]` — `schemas/registry/wos-extension-registry.schema.json` + `specs/registry/extension-registry.md`. Catalog the six kernel seams (§10) + Trellis custody shape. Lifecycle (draft → stable → deprecated → retired), composition semantics, discovery. Future scope (separate ADRs): typed event meta-kinds, Integration Profile binding types, Semantic Profile ontology mappings.
- [ ] **#22 Crate split along tier boundaries** `[Imp 6 / Cx 6 / Debt 4]` — Split `wos-core` → `wos-kernel | wos-governance | wos-ai | wos-advanced`. Replace `ProvenanceKind` (93 variants) with tier-typed record. Invert `wos-formspec-binding → wos-runtime`. Split `runtime.rs` (3821 lines) along action-kind dispatch. Add CI dependency fence. Close `custodyHook.additionalProperties: true` (Trellis shape relocates to #21). Relocate L1/L2/L3-named kernel fixtures; relocate `wos-correspondence-metadata.schema.json` to `schemas/sidecars/`.
- [ ] **#23 OverrideRecord schema** `[Imp 6 / Cx 2 / Debt 4]` — Promote Governance §7.3 three-field requirement (rationale + authority verification + supporting evidence) into typed `OverrideRecord` `$def`. Part of unified ADR sequence #23 → #24a → #2.
- [ ] **#24a Mandatory Facts-Tier input snapshot** `[Imp 8 / Cx 3 / Debt 7]` — Tighten Facts Tier §8.2: case-file input snapshot MANDATORY and typed at `determination`-tagged transitions. 0 of 146 conformance fixtures populate `inputs` today — retrofit is cheap NOW, expensive once fixtures accumulate. Silent dependency of #2. Unblocks #23.
- [ ] **#29a Milestone spec-lag closure** `[Imp 5 / Cx 2 / Debt 5]` — Kernel §4.13 prose + Milestone schema describe KS.2's shipped behavior. Add `triggerMode: "writeSettled"` property reflecting runtime policy.
- [ ] **#34 `x-lm.critical` enforcement gate** `[Imp 6 / Cx 1 / Debt 2]` — CI rule (`docs:check`) rejecting schema PRs where `x-lm.critical: true` nodes lack `description` or `examples`. 131 critical nodes; 0 current violations.
- [ ] **#40 Task SLA authoring surface** `[Imp 6 / Cx 5 / Debt 5]` — Add schema properties for §10.3 normative prose (`slaDefinitions`, `warningThresholds`, `breachPolicy ∈ {escalate | reassign | notify | extend}`, `escalationChain`). Currently spec'd as normative processor behavior with no schema surface.
- [ ] **#46 Schema-prose enum alignment batch** `[Imp 4 / Cx 2 / Debt 3]` — Close to enum: `CaseRelationship.type`, `HoldPolicy.holdType` (reconcile §12.2 / §7.15 / schema three-way disagreement on `legal-hold`), `AppealMechanism.reviewerConstraint` (required + enum incl. `independentFromOriginal`), `AppealMechanism.continuationScope`. Add FEL context citation to `DelegationScope.conditions`. ISO 8601 duration patterns (`appealWindow`, `expectedDuration`). Add missing Drift Monitor `AlertThreshold` prose table. Domain-specific values route through #21 registry.
- [ ] **#57 Assurance schema `x-lm.critical` coverage** `[Imp 3 / Cx 1 / Debt 2]` — Add annotations to key nodes in `schemas/assurance/wos-assurance.schema.json`. Currently zero — only schema in the suite without any.

**Structural merges (schema consolidation, absorbed from IDEA_SCRATCH):**

- [ ] **Assertion Library → Workflow Governance** as a "Named Assertions" section. Library without #38 reference protocol is incomplete; absorb rather than fix.
- [ ] **Verification Report → Advanced Governance** as an "Output Artifacts" section. Thin sidecar; doesn't justify independent spec status.
- [ ] **Due Process Config partial merge → Workflow Governance** (pending #45 step 0). If the thin NoticeTemplate is dropped (per #2) and AppealRouting + ContinuationPolicy are the remaining content, the merge closes the `ContinuationPolicy` ↔ `AppealMechanism.continuationOfServices` linkage gap structurally.
- [ ] **NoticeTemplate reconciliation** — TWO conflicting schema definitions today: thin `sections: string[]` in Due Process schema vs. rich `TemplateSection[]` with FEL conditions in Notification Template schema. Drop the thin version; Notification Template is canonical. Executed as part of #2.
- [ ] **M-1 Drift Monitor + Agent Config — BLOCKED.** Merge blocked by `fixtures/ai/benefits-drift-monitor.json` shipping standalone without paired Agent Config. Ship #37 standalone binding instead; reconsider merge if fixture is revised.
- [ ] **M-2 Notification Template + Due Process Config — REJECTED.** Notification Template has confirmed non-due-process uses across 4 categories (`adverse-decision`, `hold-notification`, `appeal-acknowledgment`, `sla-warning`). Ship #39 standalone linkage instead.

---

## 5 — Behavioral / semantic specs

Items that specify processor behavior, governance semantics, or runtime obligations. Absorbed from IDEA_SCRATCH Adopt list.

- [ ] **#3 Policy-based migration routing** `[Imp 5 / Cx 6 / Debt 2]` — `migrationPolicy` enum: `grandfather | migrateAll | migrateByState | expression`. In-flight tasks complete under version-at-creation; workflow advances under new topology after. Composes with Governance §2.9 `schemaUpgrade.migrationMechanism` + `scope`. **Open sub-questions:** `tenant`-scope behavioral contract undefined (0 code matches); version pinning on provenance records.
- [ ] **#12 Capability preconditions** `[Imp 6 / Cx 2 / Debt 4]` — `preconditions` array on agent capabilities; FEL expressions evaluated before invocation. Unsatisfied → skip, fall through to fallback chain. Saves token/confidence budget before fallback fires.
- [ ] **#13 Verifiability test principle** `[Imp 4 / Cx 1 / Debt 1]` — Doc-only. Kernel §1.2 design-goal bullet + cross-refs in Governance §6.1 and AI Integration §1.2: *"Can a second system, given only the spec and definition, cheaply verify behavior was correct?"*
- [ ] **#24b Structured rule-firing trace** `[Imp 7 / Cx 6 / Debt 4]` — Reasoning Tier structured trace: ordered rules evaluated, intermediate state, outcome. For AI-assisted decisions, includes calibration path. **Load-bearing coupling with #25** — evaluation order requires defeasibility answer. Joint design mandatory.
- [ ] **#25 Defeasibility primitive** `[Imp 6 / Cx 7 / Debt 6]` — Catala-style default logic with declared rule priorities and specificity encoding. Not FEL-in-FEL hand-coding. **Open sub-questions:** new `DefeasibleRule` construct vs. rule-bundle extension; priority encoding (specificity, numeric, topological); companion doc vs. fold into workflow-governance; composition with `sourceAuthority` rank (§6.2, 1-4); composition with Integration Profile §11.2 ("policy engine can restrict, never relax").
- [ ] **#26a `AccessControl.canRead` enforcement semantics** `[Imp 6 / Cx 3 / Debt 4]` — Specify normative processor behavior on `canRead(actorId, fieldPath) → false`: redact / return `null` / raise error / skip action. Conformance fixtures per branch. Interface exists as pure stub today (defaults `true`, zero call sites). Prerequisite to #26b.
- [ ] **#26b `caseFieldPolicy` schema** `[Imp 6 / Cx 6 / Debt 4]` — `caseFieldPolicy` `$def` in workflow-governance schema; per-field read/write scopes by actor role referencing kernel `caseFile` field identifiers. Governance-layer. **Open sub-questions:** compose with AI Integration L2 `Right` or supersede; interaction with hold policies and redacted provenance; Assurance Invariant 6 independence.
- [ ] **#27 Cancellation regions** `[Imp 4 / Cx 6 / Debt 3]` — YAWL-style named region: explicit set of tasks/states spanning arbitrary structural levels, fireable as a unit. Distinct from existing `cancellationPolicy` join policy on parallel-state co-regions. **Open sub-questions:** set or predicate; event/guard/action trigger; compensation run/skip/author's choice.
- [ ] **#28 Claim-check artifact references** `[Imp 4 / Cx 4 / Debt 3]` — Typed `ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }` as case-field value with normative integrity-check at retrieval. Type at kernel; retrieval contract at governance via `contractHook`. Wire `inputDigest`/`outputDigest` into `ProvenanceRecord` (currently spec-prose-only, zero code).
- [ ] **#29b Milestone reactive transition firing (GSM-style)** `[Imp 6 / Cx 5 / Debt 2]` — `MilestoneFired` enqueues event transitions can react to, or `$milestone.*` FEL boolean for guards. Ships after #29a. **Open sub-questions:** event-based vs. guard-based; interaction with `fired_milestones` dedup.
- [ ] **#30 WS-HumanTask lifecycle completion** `[Imp 5 / Cx 4 / Debt 2]` — Extend 8-state model to close WS-HumanTask gap: task-level `Suspended` (WOS holds are case-level today), distinct `Cancelled` terminal (separate from `skipped`), explicit `Return` with rework-iteration counter, forwarding-to-group distinct from delegation-to-person. Open: `Suspended` always reducible to case-level hold with `holdType: task-suspended`?
- [ ] **#31 Jurisdiction-aware business calendar selection** `[Imp 6 / Cx 3 / Debt 4]` — Runtime resolution of which calendar applies from a case-file field (e.g., `applicant.jurisdiction`). Replaces current "implementation-defined" selection in `sidecars/business-calendar.md §7`. Multi-jurisdiction rights-impacting workflows: compliance risk without this.
- [ ] **#35 Equity Config enforcement semantics** `[Imp 7 / Cx 5 / Debt 5]` — Specify processor obligations for `RemediationTrigger.action` (`review | audit | suspend | notify`); wire `DisparityMethod` evaluation to runtime per `ReportingSchedule`; define "suspended workflow" behaviorally. Conformance fixtures. Applies to human AND AI decisions. **Depends on #36.**
- [ ] **#36 Equity RemediationTrigger expression language** `[Imp 6 / Cx 4 / Debt 4]` — Specify expression language for `RemediationTrigger.condition`. Options: FEL extension with temporal/windowing ops, restricted DSL, or FEL + windowing functions. Depends on Kernel §7.4 grammar-extension stance (currently rejects extensions).
- [ ] **#37 Drift Monitor demotion policy binding** `[Imp 6 / Cx 3 / Debt 5]` — Normative binding from `alertThresholds[].action` to `DemotionRule`. Candidate: `alertThresholds[].policyRef`. Promoted to standalone after M-1 merge blocked.
- [ ] **#38 Assertion Library cross-document reference protocol** `[Imp 5 / Cx 3 / Debt 3]` — `assertionId` (or `assertionLibraryRef`) on `PipelineStage.assertions[]`; resolution semantics (library lookup order, version pinning). The library concept exists in prose; the reference mechanism doesn't.
- [ ] **#39 ContinuationPolicy normative linkage** `[Imp 4 / Cx 2 / Debt 3]` — Specify how `AppealMechanism.continuationOfServices: true` resolves to a specific `ContinuationPolicy`. Candidate: `continuationPolicyRef`. Promoted to standalone after M-2 rejected.
- [ ] **#42 Autonomy-lifecycle conformance fixture batch** `[Imp 5 / Cx 2 / Debt 2]` — Two fixtures: (1) escalation-expiry revocation (`EscalationRule.escalationExpiry`); (2) drift-alert-triggered demotion (Drift Monitor `alertThresholds[]` firing `DemotionRule`). Already covered: calibration-expiry (AC-001), humanOverride-triggered demotion (ai-028/ai-029).
- [ ] **#43 Assurance × impact-level composition rule** `[Imp 6 / Cx 5 / Debt 4]` — Specify whether a minimum Assurance level is required for AI-assisted determinations at `rights-impacting` impact level. Respect Invariant 6 (independence of disclosure posture and assurance level). Must be decided before any production deployment.
- [ ] **#45 Sidecar normative-contract audit** `[Imp 6 / Cx 5 / Debt 5]` — Retrofit all sidecars against CONVENTIONS.md: Step 0 (does this sidecar deserve independent existence?) + three-question rubric (Structure / Semantics / Composition) on survivors.
- [ ] **#56 Runtime §2 isolation-invariant lint rule** `[Imp 5 / Cx 2 / Debt 3]` — Static AST lint detecting `setData` → guard dependency cycles in `continuous`-mode documents. §2.4 invariant is normative but unvalidated; `continuous_reevaluate` is dead code today, so lint prevents future defective documents.

---

## 6 — Audit and evidence products

Build on the stable provenance export surface from §2.

- [ ] **#2 Deterministic adverse-decision notice (dual-form)** `[Imp 9 / Cx 7 / Debt 6]` — Specified deterministic algorithm (not model-generated) deriving two co-synchronized outputs from the same Facts + Reasoning provenance: a machine-readable artifact (structured, citable, diffable under audit) and a human-prose artifact (plain language, suitable for legal service). Identical inputs MUST produce identical outputs in both forms. Sits at Governance §3.2 — explicitly separated from the non-authoritative Narrative tier (AI Integration §13). Serves subject, attorney, auditor, and implementer from one algorithm. Delivery mechanism = Notification Template §4.4 (FEL-conditional sections + `requiredVariables` enforcement). Scaffolding today: `AdverseDecisionPolicy` typed but permissive; `NoticeSent` is a hardcoded stub (`event_handler.rs:72-81`); zero runtime rendering code. Remaining work: deterministic assembly algorithm + rendering pipeline + determinism fixtures + NoticeTemplate reconciliation. **Dependencies:** #24a (tightened Facts Tier) must land first.
- [ ] **#48 Merkle provenance chains** `[Imp 6 / Cx 6 / Debt 4]` — Cryptographic hash-chaining for tamper-evident logs; append-only, signed tree heads, inclusion proofs. Attaches via Assurance `provenanceLayer` seam. Hash-chaining only initially (lightest path); full SCITT / RFC 9162 transparency-service integration as later ADR.
- [ ] **#52 Simulation trace format** `[Imp 4 / Cx 3 / Debt 2]` — Normative replay semantics for simulation runs (validation, tooling, regression testing). Event log format is XES (already shipped via `wos-export::xes`). Remaining work: normative replay contract + conformance fixtures.

---

## 7 — Regulatory alignment

External-deadline-driven. Benefits from ontology (§2) landing first so field identity is stable before regulatory text cites it.

- [ ] **#50 EU AI Act alignment** `[Imp 7 / Cx 5 / Debt 4]` — Art. 13–14 alignment spec: draft → 1.0.0. Watchlist — external compliance deadlines can force escalation.
- [ ] **#50 OMB M-24-10 compliance** — Compliance support spec: draft → 1.0.0.

---

## 8 — Interoperability and speculative research

Pick up when §§2–7 stabilize.

- [ ] **SCXML interoperability** — Bidirectional WOS ↔ SCXML mapping (currently informative only).
- [ ] **#51 Statutory deadline chains** `[Imp 4 / Cx 7 / Debt 3]` — Interdependent government deadlines and automated legal consequences (e.g., 30-day notice → 10-day response → 5-day finalization chains). No chained-deadline construct exists. Architecturally expensive — wrong abstraction here is expensive.

---

## Deferred (with triggers)

Items captured but not active; re-score when the named trigger fires.

| IDEA # | Item | Imp | Cx | Debt | Trigger |
|---|---|---:|---:|---:|---|
| #1 | Agent Behavioral Attestations | 2 | 7 | 1 | SLSA-style AI-agent attestation ecosystem matures OR specific deployment demands capability attestation. |
| #4 | Tripartite Object Model | 2 | 9 | 3 | Activity-definition reuse across workflows becomes a real pattern. |
| #6 | Typed Patch Operations | 1 | 8 | 0 | Authoring tool ships structural edits. |
| #7 | OCEL 2.0 Object-Centric Case Model | 2 | 9 | 5 | Multi-object mutation patterns emerge, or flat→OCEL export shows systematic semantic loss. |
| #9 | JSON-LD Export Surface | 5 | 5 | 3 | `ontology-spec.md` drafts begin OR shipped PROV-O export pulls `@context` into authoring. |
| #32 | Multi-Instance Iteration | 6 | 7 | 5 | #20 lands. Highest-priority deferred item. |
| #33 | Inclusive-OR / Event-Choice / Boundary Events | 3 | 5 | 2 | Authoring frustration with workarounds (externally observable signal). |

---

## Future specs (trigger-gated)

| Spec | Description | Trigger |
|------|-------------|---------|
| Batch Operations | Parallel case instantiation, bulk state transitions | Sustained deployments above 100 cases/minute |
| Federation Profile | Cross-org trust, signed provenance | Second organization adopts WOS |
| Learning Profile | Retraining governance | Long-lived AI agents need retraining policy |

---

## Rejected

Decisions locked; do not re-litigate.

| IDEA # | Item | Reason |
|---|---|---|
| #5 | DAG Processing Model | Contradicts axis 4 (append-only event-stream folding). Reactive re-evaluation explicitly rejected. |
| #8 | FEL Conformance Profiles | Kernel §7.4 rejects grammar extensions. |
| #10 | WCOS + FEEL | Rename + DMN-expression-language both abandoned. |
| #17 | SHACL | Existing Rust lint (55 T2 rules) covers cross-doc validation; SHACL would duplicate. Shipped PROV-O is JSON-LD; if output-shape validation is needed, scope a dedicated item — don't resurrect SHACL wholesale. |
| #18 | Minimal Governance Envelope | Strip lifecycle from kernel → doc that cannot be understood in isolation. |
| #19 | FEEL Expression Language | FEL is purpose-built; FEEL carries DMN assumptions. |
| — | BPMN Parity as Authoring Goal | Export target, not authoring surface. Topology rejected; event taxonomy adopted normatively via #20. |

---

## Parked

- [ ] Full lifecycle soundness verification (e.g. linear-time logic). Advanced Governance SMT is the path.
- [ ] JSON Patch for fine-grained provenance.
- [ ] FEEL-to-FEL migration guide — on-demand, write when first DMN shop asks.

---

## Open questions

1. **Engine-adapter sequencing** — TODO §3 ↔ IDEA Deferred. Defer until first commercial request, or schedule now to validate runtime against production-shape workloads?
2. **Broken ADR references** (ADR-0057, ADR-0058) — draft retroactively or delete?
3. **Ontology-spec authoring ownership** — who drafts, when?
4. **Timer semantics** (#20). Wall-clock or business-days for `noticeGracePeriod` legal compliance? Business calendar reference opt-in or required?
5. **Registry composition** (#21). Two L1 governance docs attaching rules to the same tag — declaration order, explicit priority, or conflict rejection?
6. **Multi-instance design** (#32). Events, arrays, or both? Governance hooks per-instance vs. per-iteration.
7. **Version migration declaration surface** (#3). Kernel carries governance version or each case? `tenant`-scope behavioral contract?
8. **Canonical forms.** Enforce "simple sequential workflow MUST be expressed as compound state with ordered children, not flat atomic sequence"?
9. **Defeasibility layer** (#25). `workflow-governance` or distinct companion? Priority encoding? Compose with `sourceAuthority` AND Integration Profile §11.2.
10. **Case-field policy vs. L2 `Right`** (#26b). Compose or supersede? Hold policy interaction. Assurance Invariant 6 independence.
11. **Cancellation region semantics** (#27). Set or predicate? Event / guard / explicit action? Compensation run / skip / author's choice?
12. **`ExternalArtifactRef` shape** (#28). 9th case-field type or `$def`? Retrieval contract — sync / deferred / action-body?
13. **Task suspension reducibility** (#30). Always reducible to `holdType: task-suspended`, or independent task state needed?
14. **Equity expression language** (#36). FEL extension, restricted DSL, or FEL + windowing?
15. **Assurance-level composition** (#43). Minimum floor per impact level, disclosure-only, or implementation-defined?
16. **JSON-LD authoring surface** (Deferred #9). Should `@context` land in authoring or stay export-only?
17. **#29b firing mechanism.** Event-based (enqueue synthetic event) or guard-based (`$milestone.*` FEL boolean)?

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

**Normative features (from IDEA_SCRATCH Shipped)**

- [x] **Null behavior on deontic constraints** (formerly IDEA #11) — `nullBehavior` on Permission/Prohibition/Obligation with impact-level defaults. `ai-integration.md §4.2-4.5 + §5`; `NullBehavior` `$def`.
- [x] **Arazzo integration sequences** (formerly IDEA #14) — Multi-step API orchestration via Arazzo references. `integration.md §3.5`; fixtures `INT-ARAZZO-001..003`. (See NB.4.)
- [x] **Non-HTTP tool invocation** (formerly IDEA #15) — `tool` binding kind (`command-line`, `batch-file`, `database-procedure`, `graph-query`). `integration.md §3.6`; fixtures `INT-TOOL-001..002`. (See NB.4.)
- [x] **Assist Governance Proxy** (formerly IDEA #16) — Deontic constraint enforcement on Formspec Assist tool calls. `ai-integration.md §14`; schema `AssistGovernanceProxy`. Stabilizes with Assist layer upstream.

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

**Provenance export (PE)**

- [x] PE.1 — `wos-export` crate: PROV-O JSON-LD (§5.3–5.6), XES XML (§6.3), OCEL 2.0 JSON (§6.4); `timestamp` added to `ProvenanceRecord`; 3 SP-EXPORT-* conformance fixtures green (9daf447, 7cedfae, d8fbcf0, 7cd3cd3, 3ed010e, bd4e52f, b55b67e). Known limitations: higher-tier PROV-O bundles (§5.4) not emitted; OCEL events link to instance object only (per-case-file-item E2O links deferred); SHACL validation out of scope; agent actor-type falls back to plain `prov:Agent` pending `ProvenanceRecord` actor-type extension.

**Integration Profile binding kinds (NB)**

- [x] NB.1 — typed `IntegrationBindingKind` enum + `IntegrationBindingHandler` trait; replaced stringly-typed dispatch (f017910).
- [x] NB.2 — outputBinding RFC 9535 profile pinned (wildcard + slice; filter/recursive-descent rejected); lint rule I-001; spec §3.3.1 (e6e916d).
- [x] NB.3 — CloudEvents bindings (`event-emit`, `event-consume`, `callback`) with subject correlation `{instanceId}:{bindingId}:{invocationId}`; full envelope captured in provenance; 6 fixtures INT-EMIT/CONSUME/CALLBACK-001–003 (75c8b21).
- [x] NB.4 — Arazzo, tool, and policy-engine bindings; `PolicyDecision` normalized to `{decision, reasons, obligations}`; 7 fixtures INT-ARAZZO/TOOL/POLICY-001–004 (d79c02b).

**Security / architecture docs**

- [x] Runtime S13 isolation conformance guidance.
- [x] AI-004 / AI-050 behavioral verification strategy (ARCH-AI004).
