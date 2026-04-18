# WOS TODO

**Last audited:** 2026-04-16
**Counts:** 18 specs, 19 schemas, 41 document fixtures + 146 conformance fixtures (0 T3 red, 146 green), 7 crates, 197 lint rules in `LINT-MATRIX.md` (🚨 **unreconciled**: code registry has 97 reified entries per commit `1f8eae5` — §4.2 Task 2 closes the ~100-rule gap)

**Links:** [Core extraction plan](../thoughts/plans/2026-04-10-wos-core-extraction.md) (complete) · [Runtime plan](../thoughts/plans/2026-04-13-wos-runtime-crate.md) (complete) · [§1 plan](thoughts/plans/2026-04-14-wos-spec-section-1-implementation.md) (complete) · [LINT-MATRIX](LINT-MATRIX.md) · [Runtime Companion](specs/companions/runtime.md) · [Feature Matrix](WOS-FEATURE-MATRIX.md) · [Implementation Status](WOS-IMPLEMENTATION-STATUS.md) · [IDEA_SCRATCH](IDEA_SCRATCH.md) · [POSITIONING](POSITIONING.md) · [CONVENTIONS](CONVENTIONS.md) · [ADR 0065](../thoughts/adr/0065-wos-authoring-stack-mirrors-formspec.md)

**2026-04-16 architecture review.** An external review ([archived handoff](thoughts/archive/reviews/2026-04-16-architecture-review-handoff.md)) produced four hygiene items (§4) and six LLM-authoring items (§5). Architectural validation decisions live in [ADR 0064](../thoughts/adr/0064-wos-granularity-and-ai-native-positioning.md). Positioning updated in [POSITIONING.md](POSITIONING.md) and [README.md](README.md) to lead with the Claim A / Claim B framing. Active plans:

- **§4.1 Extension fix** — 19 schemas patched with `patternProperties ^x-` at every nested level (2026-04-16 root + 2026-04-17 nested). §10.6 amended to document both `extensions` property and sibling `x-` keys, and to reserve `x-wos-*` namespace. Follow-up: K-EXT-001/K-EXT-002 lint rules + fixtures (captured in rule-coverage plan).
- **§4.3 Precedence clause** — Added to both companions (landed). Follow-up: COMP-001 drift-detection lint rule.
- **§4.2 Rule-coverage conformance** — [plan](thoughts/plans/2026-04-16-wos-rule-coverage-conformance.md). Prerequisite for §4.4.
- **§4.4 Split release trains** — [plan](thoughts/plans/2026-04-16-wos-release-trains.md). Depends on §4.2.
- **§5.1 Schema description audit** — [plan](thoughts/plans/2026-04-16-wos-schema-description-audit.md). Prerequisite for §5.4.
- **§5.2 Structured lint diagnostics** — [plan](thoughts/plans/2026-04-16-wos-structured-lint-diagnostics.md). Prerequisite for §5.4.
- **§5.3 Trace-emitting conformance** — [plan](thoughts/plans/2026-04-16-wos-trace-emitting-conformance.md). Prerequisite for §5.4.
- **`wos-authoring` crate (new 2026-04-17)** — [plan](thoughts/plans/2026-04-17-wos-authoring-crate.md). Intent-driven authoring helpers over `wos-core`, analogous to parent Formspec's `formspec-studio-core`. Prerequisite for `wos-mcp` and `wos-synth-core`.
- **`wos-mcp` crate (new 2026-04-17)** — [plan](thoughts/plans/2026-04-17-wos-mcp-crate.md). Thin MCP adapter over `wos-authoring` with dual entry (MCP stdio + in-process dispatch), analogous to parent Formspec's `formspec-mcp`. Enables external-agent authoring (Claude Desktop / Cline) and serves as the production `ToolContext` implementation for `wos-synth-core`.
- **§5.4 `wos-synth-core` + providers (rewritten 2026-04-17)** — [plan](thoughts/plans/2026-04-16-wos-synth-crate.md). The original monolithic `wos-synth` scaffold at commit `2815e4d` is being reshaped into four crates per ADR 0065: `wos-synth-core` (loop + `Prompter` trait + `ToolContext` trait), `wos-synth-anthropic` (concrete provider), `wos-synth-mock` (test provider), `wos-synth-cli` (binary). Honors dependency inversion at crate boundaries rather than feature flags.
- **§5.5 Synthesis benchmark** — [plan](thoughts/plans/2026-04-16-wos-synthesis-benchmark.md). Falsifies Claim A with metrics. Implemented as `wos-bench@0.x`, which consumes `wos-synth-core` + a provider of choice (e.g. `wos-synth-anthropic` or `wos-synth-mock`) rather than the monolithic `wos-synth`.
- **§5.6 Repositioning docs** — README + POSITIONING updated (landed).
- **§8 Open questions** — [doc (archived 2026-04-17)](thoughts/archive/reviews/2026-04-16-architecture-review-open-questions.md) captured 5 maintainer-decision questions (Claim A scope, `wos-synth` location, release tool, load-bearing seeds, compat-matrix convention) plus a 6th (wos-synth ↔ benchmark split) added during synthesis. All six resolved 2026-04-17 with plan updates landed in §4.2 / §4.4 / §5.4 / §5.5.
- **Schema regression tests** — [plan](thoughts/plans/2026-04-17-wos-schema-regression-tests.md). Adds three pytest suites (meta-validity, fixture validity, spec-example validity) modeled on parent Formspec's `tests/conformance/spec/test_spec_examples.py`. Closes the "no regression guard for schema edits" gap flagged in the code-review.
- **v0 spike (new 2026-04-17)** — [plan](thoughts/plans/2026-04-17-wos-synth-v0-spike.md). 2–3 day disposable crate that validates the six-crate architecture before the larger plans (§wos-authoring, §wos-mcp, §5.4) execute. Prerequisite for maximum-confidence execution of those plans.

**Missing / unknown references:** `ADR-0058 (wos-core-gap-analysis)` and `ADR-0057 (wos-core-implementation-boundary)` were cited from prior headers but do not exist at `../thoughts/adr/`. Status: unknown — either never authored, relocated, or inlined into other material. Resolve before next audit.

**Priority logic (2026-04-16 re-sort).** Two goals drive order: (A) reduce architectural lock-in while it's still cheap, (B) make WOS immediately usable by a first real adopter. Items are ranked by cost-to-defer, not cost-to-do. Cheap-and-cheap-forever items are bundled separately so they don't crowd the critical path. The prior Urgency formula from IDEA_SCRATCH (`(Imp+Debt)/Cx`) is retired — it over-rewarded low-Cx regression-prevention items. Scores `[Imp/Cx/Debt]` are preserved per item as metadata — they inform relative weight within each tier but do not override cross-tier ordering.

**Score definitions (0–10 scale):**

- **Imp** — **Importance.** How much does this item move the project forward (architectural leverage, first-adopter enablement, civil-rights/compliance weight). Higher = do it.
- **Cx** — **Complexity.** How much real work (design + implementation + test) this takes. Higher = bigger lift.
- **Debt** — **Architectural tech debt if deferred.** How much extra rework lands later if we don't do it now. Higher = cheaper now than later. Confined-scope fixes score low; load-bearing foundational items (0/N fixtures, unclosed escape hatches) score high.

**Score validation (2026-04-16).** Scores audited in parallel by four code-scout agents against live schemas, specs, crates, and fixtures. Adjustments applied: DRAFTS Debt 7→5, #24a Cx 3→4, #20 Cx 6→7, #46 Cx 2→3, #39 Cx 2→1, #12 Cx 2→3, #56 Debt 3→2, #35 Debt 5→4, #40 Debt 5→4, #30 Cx 4→5, #28 Debt 3→2, Assertion-Library merge Cx 3→2, #22 Cx 6→4, #48 Debt 4→6, #51 Debt 3→5. Factual corrections applied to #22 (runtime.rs lives in wos-runtime at 4451 lines, not wos-core at 3821; binding-inversion already landed), #28 (inputDigest/outputDigest already wired through export crate, not prose-only), #56 (continuous_reevaluate has 4 in-crate test callers, not "dead code").

---

## 1 — Reference implementation blockers

> §1 closed 2026-04-14 — see Completed.

---

## 2 — Foundational (zero external dependencies)

- [x] **Provenance export** — Serialize internal provenance to W3C PROV-O, OCEL 2.0, IEEE 1849 XES. Landed 2026-04-15 — see Completed.
- [ ] **Ontology field identity** *(design not started — do not sequence as active work)* — `ontology-spec.md` does not exist. Informs AI integration, cross-document alignment, and regulatory specs in §6, but cannot be scheduled until the spec is drafted. Prerequisite design work: JSON-LD `@context` decision (see Deferred #9), semantic-field-identity protocol, cross-document alignment mechanism. Move to active only once a draft exists.

---

## 3 — Engine adapters (open question — sequencing unresolved)

> **Status:** sequencing unresolved. TODO previously placed engine adapters as a near-term priority; IDEA_SCRATCH #49 marked them Defer with trigger "first commercial deployment requesting a specific adapter." No arbitrating document. Items kept in the backlog below but **not** scheduled until this question is resolved.

- [ ] **#49 Camunda 8 Worker** `[Imp 5 / Cx 8 / Debt 3]` — Delegate BPMN task execution under WOS governance. Most common BPMN target; broadest external fixture diversity.
- [ ] **#49 Temporal Workflow** `[Imp 5 / Cx 8 / Debt 3]` — Map WOS evaluation steps to deterministic replay. Natural fit with WOS evaluator determinism.
- [ ] **#49 AWS Step Functions** `[Imp 5 / Cx 8 / Debt 3]` — Bridge ASL states to WOS transitions. Broadest commercial reach; narrowest semantic fit.

---

## 4 — Active backlog (priority-ordered)

Previously split across "schema closures" and "behavioral specs." Collapsed and re-sorted 2026-04-16 by cost-to-defer + first-adopter enablement.

### 4.1 — Critical path (lock-in + usable)

Items that get materially more expensive if deferred, or that block a first real adopter. Do these first.

- [ ] **DRAFTS triage** `[Imp 5 / Cx 3 / Debt 5]` *(prerequisite — not an IDEA item)* — `DRAFTS/` contains 12 kernel version proposals (v2–v7 + competing v7 drafts). Classify archive / delete / extract. **Blocks #20.** Must complete before any schema/spec PR touching the kernel lands. Files are inert markdown (not referenced from schemas/crates), so Debt is a review-time tax rather than structural lock-in.
- [ ] **#24a Mandatory Facts-Tier input snapshot** `[Imp 8 / Cx 4 / Debt 7]` — Tighten Facts Tier §8.2: case-file input snapshot MANDATORY and typed at `determination`-tagged transitions. 0 conformance fixtures populate `inputs` today; retrofit touches ~51 determination-tagged fixtures (out of 157), plus schema tightening and new conformance rule. Cheap now, expensive once fixtures accumulate. Silent dependency of #2. Unblocks #23.
- [ ] **#23 OverrideRecord schema** `[Imp 6 / Cx 2 / Debt 4]` — Promote Governance §7.3 three-field requirement (rationale + authority verification + supporting evidence) into typed `OverrideRecord` `$def`. Part of unified ADR sequence #23 → #24a → #2.
- [ ] **NoticeTemplate reconciliation** `[Imp 7 / Cx 2 / Debt 5]` — TWO conflicting schema definitions today: thin `sections: string[]` in Due Process schema vs. rich `TemplateSection[]` with FEL conditions in Notification Template schema. Drop the thin version; Notification Template is canonical. **Blocks #2.** High Debt: second schema locks in a second divergent authoring surface the longer it ships.
- [ ] **#2 Deterministic adverse-decision notice (dual-form)** `[Imp 9 / Cx 7 / Debt 6]` — Specified deterministic algorithm (not model-generated) deriving two co-synchronized outputs from the same Facts + Reasoning provenance: a machine-readable artifact (structured, citable, diffable under audit) and a human-prose artifact (plain language, suitable for legal service). Identical inputs MUST produce identical outputs in both forms. Sits at Governance §3.2 — explicitly separated from the non-authoritative Narrative tier (AI Integration §13). Delivery mechanism = Notification Template §4.4 (FEL-conditional sections + `requiredVariables` enforcement). Scaffolding today: `AdverseDecisionPolicy` typed but permissive; `NoticeSent` is a hardcoded stub (`event_handler.rs:72-81`); zero runtime rendering code. Remaining work: deterministic assembly algorithm + rendering pipeline + determinism fixtures. **Dependencies:** #24a + #23 + NoticeTemplate reconciliation.
- [ ] **#20 Typed event meta-vocabulary** `[Imp 8 / Cx 7 / Debt 6]` — Replace `Transition.event: string` with strict 5-kind typed union `{ kind: "timer" | "message" | "signal" | "condition" | "error", ... }`. No `named` wrapper; no escape hatch. Co-type `Action.event` for `startTimer`. Closes kernel's last load-bearing openness. Migration surface is ~168 fixtures containing `"event":` strings (much larger than originally framed); plus schema + Rust model + K-007 lint promotion to schema validation. **Depends on DRAFTS triage.**
- [ ] **#31 Jurisdiction-aware business calendar selection** `[Imp 6 / Cx 3 / Debt 4]` — Runtime resolution of which calendar applies from a case-file field (e.g., `applicant.jurisdiction`). Replaces current "implementation-defined" selection. Multi-jurisdiction rights-impacting workflows: compliance risk without this.

### 4.2 — Next (unblocks once §4.1 lands)

- [ ] **#22a ProvenanceKind tier-typing** `[Imp 4 / Cx 4 / Debt 3]` *(extracted from #22; re-scored 2026-04-16 post-PE.2)* — Replace the 93-variant `ProvenanceKind` monolith enum (`crates/wos-core/src/provenance.rs`) with a tier-typed record (kernel / governance / ai / advanced). **Debt lowered 5→3:** PE.2 added the `audit_layer` field and an exhaustive `audit_layer_for_kind` match, so new variants must now explicitly declare their tier at compile time — the "ossification" pressure is partly relieved. Remaining value is data-shape cleanliness: separating record payloads by tier so each tier's struct carries only the fields it can populate. Still load-bearing for the broader #22 crate split but no longer urgent. The rest of #22 (directory split, runtime.rs split, CI fence) remains organizational and stays in §4.6.
- [ ] **#46 Schema-prose enum alignment batch** `[Imp 4 / Cx 3 / Debt 3]` — Close to enum: `CaseRelationship.type`, `HoldPolicy.holdType` (reconcile §12.2 / §7.15 / schema three-way disagreement on `legal-hold`), `AppealMechanism.reviewerConstraint` (required + enum incl. `independentFromOriginal`), `AppealMechanism.continuationScope`. Add FEL context citation to `DelegationScope.conditions`. ISO 8601 duration patterns. Add missing Drift Monitor `AlertThreshold` prose table. Domain-specific values route through #21 registry.
- [ ] **#21 Extension registry (seams-only MVP)** `[Imp 5 / Cx 4 / Debt 3]` — `schemas/registry/wos-extension-registry.schema.json` + `specs/registry/extension-registry.md`. Catalog the six kernel seams (§10) + Trellis custody shape. Lifecycle (draft → stable → deprecated → retired), composition semantics, discovery. Catalogs relocations from #46 and closes `custodyHook` escape.
- [ ] **#29a Milestone spec-lag closure** `[Imp 5 / Cx 2 / Debt 5]` — Kernel §4.13 prose + Milestone schema describe KS.2's shipped behavior. Add `triggerMode: "writeSettled"` property reflecting runtime policy.
- [ ] **#37 Drift Monitor demotion policy binding** `[Imp 6 / Cx 3 / Debt 5]` — Normative binding from `alertThresholds[].action` to `DemotionRule`. Candidate: `alertThresholds[].policyRef`. Promoted to standalone after M-1 merge blocked.
- [ ] **#39 ContinuationPolicy normative linkage** `[Imp 4 / Cx 1 / Debt 3]` — Specify how `AppealMechanism.continuationOfServices: true` resolves to a specific `ContinuationPolicy`. `ContinuationPolicy` `$def` already exists (`wos-due-process.schema.json:160`) and `continuationOfServices: boolean` already exists (`wos-workflow-governance.schema.json:324`); work is one `continuationPolicyRef` string + brief resolution prose. Promoted to standalone after M-2 rejected.

### 4.3 — Cheap batch (ship together in one sprint)

Low-cost, low-risk, no lock-in. Independent of critical-path work — can land in parallel. Ordering within the batch doesn't matter.

- [ ] **#34 `x-lm.critical` enforcement gate** `[Imp 6 / Cx 1 / Debt 2]` — CI rule (`docs:check`) rejecting schema PRs where `x-lm.critical: true` nodes lack `description` or `examples`. 131 critical nodes; 0 current violations.
- [ ] **#57 Assurance schema `x-lm.critical` coverage** `[Imp 3 / Cx 1 / Debt 2]` — Add annotations to key nodes in `schemas/assurance/wos-assurance.schema.json`. Only schema in the suite without any.
- [ ] **#13 Verifiability test principle** `[Imp 4 / Cx 1 / Debt 1]` — Doc-only. Kernel §1.2 design-goal bullet + cross-refs in Governance §6.1 and AI Integration §1.2.
- [ ] **#12 Capability preconditions** `[Imp 6 / Cx 3 / Debt 4]` — `preconditions` array on agent capabilities; FEL expressions evaluated before invocation. Unsatisfied → skip, fall through to fallback chain.
- [ ] **#56 Runtime §2 isolation-invariant lint rule** `[Imp 5 / Cx 2 / Debt 2]` — Static AST lint detecting `setData` → guard dependency cycles in `continuous`-mode documents. `continuous_reevaluate` is defined at `crates/wos-core/src/eval_mode.rs:55` with 4 in-crate test callers (not dead code, as earlier framing claimed); lint prevents future defective documents from shipping.
- [ ] **#42 Autonomy-lifecycle conformance fixture batch** `[Imp 5 / Cx 2 / Debt 2]` — Two fixtures: (1) escalation-expiry revocation; (2) drift-alert-triggered demotion. Already covered: calibration-expiry (AC-001), humanOverride-triggered demotion (ai-028/ai-029).

### 4.4 — Behavioral backlog (after §4.1–§4.3 stabilize)

Specifies processor behavior, governance semantics, or runtime obligations. Not usability-critical, not foundational lock-in — schedule once the critical path and cheap batch have landed. Dependencies noted where they exist.

- [ ] **#26a `AccessControl.canRead` enforcement semantics** `[Imp 6 / Cx 3 / Debt 4]` — Specify normative processor behavior on `canRead(actorId, fieldPath) → false`: redact / return `null` / raise error / skip action. Conformance fixtures per branch. Interface exists as pure stub today (defaults `true`, zero call sites). **Prerequisite to #26b.**
- [ ] **#26b `caseFieldPolicy` schema** `[Imp 6 / Cx 6 / Debt 4]` — `caseFieldPolicy` `$def` in workflow-governance schema; per-field read/write scopes by actor role. Governance-layer.
- [ ] **#36 Equity RemediationTrigger expression language** `[Imp 6 / Cx 4 / Debt 4]` — FEL extension vs. restricted DSL vs. FEL + windowing. **Prerequisite to #35.**
- [ ] **#35 Equity Config enforcement semantics** `[Imp 7 / Cx 5 / Debt 4]` — Specify processor obligations for `RemediationTrigger.action`; wire `DisparityMethod` to runtime per `ReportingSchedule`; define "suspended workflow" behaviorally. Applies to human AND AI decisions. Runtime seam partially in place (`ProvenanceKind::EquityAlert`, lifecycle emission in `event_handler.rs`); behavioral enforcement still absent.
- [ ] **#24b + #25 joint design** *(rule-firing trace + defeasibility)* `[#24b: 7/6/4 · #25: 6/7/6]` — Reasoning Tier gains ordered rule list, intermediate state, outcome; Catala-style default logic with declared rule priorities. Load-bearing coupling — evaluation order requires defeasibility answer. Must compose with `sourceAuthority` rank (§6.2) and Integration Profile §11.2 ("restrict, never relax").
- [ ] **#43 Assurance × impact-level composition rule** `[Imp 6 / Cx 5 / Debt 4]` — Specify whether a minimum Assurance level is required for AI-assisted determinations at `rights-impacting` impact. Respect Invariant 6.
- [ ] **#38 Assertion Library cross-document reference protocol** `[Imp 5 / Cx 3 / Debt 3]` — `assertionId` on `PipelineStage.assertions[]`; resolution semantics. The library concept exists in prose; the reference mechanism doesn't.
- [ ] **#40 Task SLA authoring surface** `[Imp 6 / Cx 5 / Debt 4]` — Add schema properties for §10.3 normative prose (`slaDefinitions`, `warningThresholds`, `breachPolicy`, `escalationChain`). Currently spec'd as normative processor behavior with no schema surface. Adjacent scaffolding exists (`sla-warning` category in notification-template schema; SLA-aware business calendar schema), which reduces retrofit cost if deferred.
- [ ] **#30 WS-HumanTask lifecycle completion** `[Imp 5 / Cx 5 / Debt 2]` — Extend 8-state model: task-level `Suspended`, distinct `Cancelled` terminal, explicit `Return` with rework counter, group-forwarding distinct from person-delegation.
- [ ] **#27 Cancellation regions** `[Imp 4 / Cx 6 / Debt 3]` — YAWL-style named region spanning arbitrary structural levels, fireable as a unit. Distinct from existing `cancellationPolicy` join policy.
- [ ] **#28 Claim-check artifact references** `[Imp 4 / Cx 4 / Debt 2]` — Typed `ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }` as case-field value with normative integrity-check at retrieval. `inputDigest`/`outputDigest` fields are already wired through `ProvenanceRecord` and the export crate (`wos-export/src/{ocel,xes,prov_o}.rs`); remaining work is the `ExternalArtifactRef` type and population/retrieval contract.
- [ ] **#29b Milestone reactive transition firing (GSM-style)** `[Imp 6 / Cx 5 / Debt 2]` — `MilestoneFired` enqueues event, or `$milestone.*` FEL boolean for guards. Ships after #29a.
- [ ] **#3 Policy-based migration routing** `[Imp 5 / Cx 6 / Debt 2]` — `migrationPolicy` enum: `grandfather | migrateAll | migrateByState | expression`. Composes with Governance §2.9. **Open sub-questions:** `tenant`-scope behavioral contract undefined (0 code matches); version pinning on provenance records.

### 4.5 — Structural merges (schema consolidation)

Absorbed from IDEA_SCRATCH. Schedule alongside whichever critical-path item naturally touches them.

- [ ] **Assertion Library → Workflow Governance** `[Imp 4 / Cx 2 / Debt 3]` — Absorb as "Named Assertions" section. Library without #38 reference protocol is incomplete; absorb rather than fix. Source is a thin 55-line spec + 139-line schema; merge is mechanical.
- [ ] **Verification Report → Advanced Governance** `[Imp 3 / Cx 2 / Debt 2]` — Absorb as "Output Artifacts" section. Thin sidecar.
- [ ] **Due Process Config partial merge → Workflow Governance** `[Imp 5 / Cx 3 / Debt 4]` (pending #45 step 0) — If thin NoticeTemplate drops (per #2) and AppealRouting + ContinuationPolicy remain, the merge closes the `ContinuationPolicy` ↔ `AppealMechanism.continuationOfServices` linkage gap structurally.
- **M-1 Drift Monitor + Agent Config — BLOCKED.** Merge blocked by `fixtures/ai/benefits-drift-monitor.json` shipping standalone. Ship #37 standalone binding instead; reconsider merge if fixture is revised.
- **M-2 Notification Template + Due Process Config — REJECTED.** 4 non-due-process categories. Ship #39 standalone linkage instead.

### 4.6 — Engineering hygiene (deprioritized)

Organizational debt, not architectural. First adopter won't notice. Schedule when the relevant code is actively being touched for another reason.

- [ ] **#22 Crate split along tier boundaries** `[Imp 5 / Cx 3 / Debt 3]` *(ProvenanceKind tier-typing extracted to §4.2 as #22a)* — Split `wos-core` → `wos-kernel | wos-governance | wos-ai | wos-advanced`. Split `wos-runtime/src/runtime.rs` (now 4451 lines, up from 3821) along action-kind dispatch. Add CI dependency fence. Remaining scope is purely organizational; first adopter won't notice. **Note:** `wos-formspec-binding → wos-runtime` inversion is already landed (`wos-formspec-binding/Cargo.toml:10-13`); `runtime.rs` lives in `wos-runtime`, not `wos-core`.
- [ ] **#45 Sidecar normative-contract audit** `[Imp 6 / Cx 5 / Debt 5]` — Retrofit all sidecars against CONVENTIONS.md: Step 0 (does this sidecar deserve independent existence?) + three-question rubric (Structure / Semantics / Composition).

---

## 5 — Audit and evidence products

Build on the stable provenance export surface from §2. Schedule after §4.1 lands.

- [ ] **#48 Merkle provenance chains** `[Imp 6 / Cx 6 / Debt 6]` — Cryptographic hash-chaining for tamper-evident logs. Attaches via Assurance `provenanceLayer` seam. Hash-chaining only initially; full SCITT / RFC 9162 transparency-service integration as later ADR. **Debt raised:** PROV-O / XES / OCEL exports shipped 2026-04-15 without hash-chain hooks — every adopter of those formats now consumes unlinkable output; retrofitting means versioning three export surfaces simultaneously.
- [ ] **#52 Simulation trace format** `[Imp 4 / Cx 3 / Debt 2]` — Normative replay semantics for simulation runs. Event log format is XES (already shipped via `wos-export::xes`). Remaining work: normative replay contract + conformance fixtures.

---

## 6 — Regulatory alignment

External-deadline-driven. Benefits from ontology (§2) landing first.

- [ ] **#50 EU AI Act alignment** `[Imp 7 / Cx 5 / Debt 4]` — Art. 13–14 alignment spec: draft → 1.0.0. Watchlist — external compliance deadlines can force escalation.
- [ ] **#50 OMB M-24-10 compliance** `[Imp 6 / Cx 4 / Debt 3]` — Compliance support spec: draft → 1.0.0. Narrower than EU AI Act; overlaps existing assurance / impact-level plumbing. More process-documentation-shaped than structural, so Debt is lower.

---

## 7 — Interoperability and speculative research

Pick up when §§2–6 stabilize.

- [ ] **SCXML interoperability** `[Imp 3 / Cx 6 / Debt 2]` — Bidirectional WOS ↔ SCXML mapping (currently informative only).
- [ ] **#51 Statutory deadline chains** `[Imp 4 / Cx 7 / Debt 5]` — Interdependent government deadlines and automated legal consequences. Architecturally expensive — wrong abstraction here is expensive. **Debt raised:** once #31 jurisdiction-aware calendars and #20 typed events land, deadline chains must compose with both; deferring past those without at least a sketch risks an incompatible construct.

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

- [x] PE.1 — `wos-export` crate: PROV-O JSON-LD (§5.3–5.6), XES XML (§6.3), OCEL 2.0 JSON (§6.4); `timestamp` added to `ProvenanceRecord`; 3 SP-EXPORT-* conformance fixtures green (9daf447, 7cedfae, d8fbcf0, 7cd3cd3, 3ed010e, bd4e52f, b55b67e). Known limitations: higher-tier PROV-O bundles (§5.4) not emitted; OCEL events link to instance object only (per-case-file-item E2O links deferred); SHACL validation out of scope.
- [x] PE.2 — `ProvenanceRecord` schema extension + full SP §5.3/§5.5/§6.3 emission (2026-04-16, branch `feat/provenance-export` at `0fb895d` — unmerged). Eight optional SP-mandated fields added to `ProvenanceRecord`: `audit_layer`, `actor_type`, `lifecycle_state`, `definition_version`, `inputs`, `outputs`, `input_digest`, `output_digest`. Runtime populates all eight at stamp time via new `populate_provenance_record_fields` helper (wired at all 9 append sites; 1:1 with `provenance_log.push`/`.extend` invariant verified). Exporters emit the full §5.3/§5.5/§6.3 mappings: PROV-O `prov:used`/`prov:wasGeneratedBy` Entity nodes, `wos:atLifecycleState`, `wos:definitionVersion`, §5.5 actor-type subclass pairs (`[prov:Person, wos:HumanAgent]` / `[prov:SoftwareAgent, wos:SystemAgent]` / `[prov:SoftwareAgent, wos:AIAgent]`); XES `org:group`, repeated-key `wos:input`/`wos:output`, trace-level `wos:definitionVersion`, `wos:lifecycleState`, per-event digests; OCEL uniform `eventTypes` schema + indexed `inputs.{i}`/`outputs.{i}` scalar attrs (OCEL 2.0 compliance — no array-valued attributes). §6.5 Facts-tier filter applied uniformly via shared `is_facts_tier` helper; exhaustive `audit_layer_for_kind` match (93/93 variants) compile-gates future tier additions. New SP-EXPORT-004 fixture locks the filter. SHA-256 digests via new `sha2` crate dep. 407 tests passing, zero TODO(spec-upstream) markers remaining. Four rounds of semi-formal code review; all findings addressed (da20e80, d33b3ef, 32e453f, d86709b + 10 findings-fix commits: 8f3583a, 8cf6802, 0357b26, 1c86299, 418c0f9, 5ee7291, 2809393, 0f2a4a0, b735923, 0fb895d). Known limitations remaining: higher-tier PROV-O bundle wrapping (§5.4 — requires export API redesign to accept tier-discriminated output); OCEL case-file-item objects + per-item E2O/O2O links (§6.4 — requires case state snapshot protocol); SHACL validation (needs RDF library dependency); `ActorKind::Agent` mapping (`actor_type = "agent"`) pending AI Integration agent-registry threading through runtime context. Follow-up plan at `thoughts/plans/2026-04-16-wos-provenance-record-schema-extension.md`.

**Integration Profile binding kinds (NB)**

- [x] NB.1 — typed `IntegrationBindingKind` enum + `IntegrationBindingHandler` trait; replaced stringly-typed dispatch (f017910).
- [x] NB.2 — outputBinding RFC 9535 profile pinned (wildcard + slice; filter/recursive-descent rejected); lint rule I-001; spec §3.3.1 (e6e916d).
- [x] NB.3 — CloudEvents bindings (`event-emit`, `event-consume`, `callback`) with subject correlation `{instanceId}:{bindingId}:{invocationId}`; full envelope captured in provenance; 6 fixtures INT-EMIT/CONSUME/CALLBACK-001–003 (75c8b21).
- [x] NB.4 — Arazzo, tool, and policy-engine bindings; `PolicyDecision` normalized to `{decision, reasons, obligations}`; 7 fixtures INT-ARAZZO/TOOL/POLICY-001–004 (d79c02b).

**Security / architecture docs**

- [x] Runtime S13 isolation conformance guidance.
- [x] AI-004 / AI-050 behavioral verification strategy (ARCH-AI004).
