# Idea Scratch

WOS design backlog — scored entries, architectural decisions, capability status, dependencies, open questions.

**Lens:** greenfield. No users, no legacy, no migration concerns. *Code is cheap, time is cheap, architecture is invaluable.* Delete only when truly not needed; defer when low architectural lock-in; reject when contradicting committed axes. No backward-compat scaffolding.

**Audit trail (2026-04-16):**
- Merge with orchestration-patterns research doc.
- Four-agent spec-suite audit (reinvention / missing-lead / contradictions / pre-schema).
- Code-scout validation against live crates/fixtures.
- Opus-high greenfield pass: removed `named`/`x-*` extensibility wrappers, reframed Tech Debt as architectural lock-in, honest prior-art attribution on Genuine Invention claims, ruthless trim of anticipatory ceremony.

---

## 2026-04-16 Design Direction

### The reframed question

"Should the WOS kernel grow BPMN-equivalent orchestration patterns so AI can generate a single document?" is the wrong question. BPMN parity invites scope creep and positions WOS as a competitor to a 20-year-old standard nobody loves authoring.

The right question: **what must WOS be such that AI can generate a complete, executable, governed workflow in one JSON document that runs on a WOS-native runtime and can also export to engine-specific formats (BPMN, Temporal, SCXML) as interop targets?**

### Five design axes

Properties the format must deliver. Not borrowed from any reference system.

1. **AI-generability** — closed finite vocabularies, structured diagnostics, canonical forms, schema-enforceable authoring contracts.
2. **Multi-target semantic fidelity** — every normative behavior has a specified semantic rule, not "implementation-defined." Bindings translate faithfully and document what they lose.
3. **Governance as first-class primitive** — deontic operators, due process, review protocols, authority ranking, provenance as native constructs on a declared seam.
4. **Replay determinism over an append-only event stream** — case state derived by folding events, not reactive re-evaluation. Deterministic replay including governance is a spec guarantee. **Consequence:** DAG processing (reactive re-evaluation) is explicitly rejected; see Reject #5.
5. **Long-running version migration** — workflows in flight outlive governance documents. Version pinning and migration policy are first-class.

Axes describe format properties, not capability inventory.

### Ground truth: WOS today

**Closed vocabularies:**

- **State kinds** — 4 (`atomic | compound | parallel | final`) — `wos-kernel.schema.json` `$defs/State.properties.type`
- **Action kinds** — 7 (`createTask | invokeService | setData | emitEvent | startTimer | cancelTimer | log`) — `wos-kernel.schema.json:406`
- **Case-field types** — 8 — `wos-kernel.schema.json:617`
- **FEL** — normatively required for guards, milestones, action parameters — `specs/kernel/spec.md §7.4`; kernel imports `fel_core` (`crates/wos-core/src/eval.rs:13`)
- **Integration Profile binding types** — 7 — `request-response | event-emit | event-consume | callback | arazzo-sequence | tool | policy-engine`
- **Six extension seams** — `actorExtension | contractHook | provenanceLayer | lifecycleHook | custodyHook | extensions` — `specs/kernel/spec.md §10`
- **Eleven kernel-generated events** — `$join`, five `$timeout.*`, `$error`, `$compensation.complete`, three `$related.*` — `specs/kernel/spec.md §4.10`

**Open (tracked for closure):** `Transition.event` is free-form string (→ #20 closes to 5 typed kinds, no `named` escape hatch). `HoldPolicy.holdType`, `CaseRelationship.type`, `AppealMechanism.reviewerConstraint` are prose enums without schema enforcement (→ #46 closes them). `custodyHook.additionalProperties: true` escape hatch (→ #22 closes it; Trellis shape moves to #21 extension registry).

**Layering — four documents:**

| Concern | Document | Schema |
|---|---|---|
| Structure (state machine topology) | Kernel | `wos-kernel.schema.json` |
| Governance (due process, holds, delegation, policy) | Workflow Governance | `wos-workflow-governance.schema.json` |
| Execution algorithms (compensation, sync, history) | Lifecycle Detail Companion | (prose) |
| Runtime contract (event delivery, durability, ordering) | Runtime Companion | (prose) |

Governance attaches via `lifecycleHook` keyed on **semantic tags** (`determination`, `review`, `adverse-decision`, `quality-check`, `intake`, `appeal`, `notification`, `hold`), not transition IDs.

**Conformance** — 197 normative constraints in `LINT-MATRIX.md` (37 static / 55 cross-doc / 105 dynamic); 3230+ lines of typed rule code.

**AI-readiness** — `x-lm.critical` annotations on 131 schema nodes (all currently pass description+examples per validation); `.llm.md` files per spec; rule-ID-keyed conformance fixtures; `wos-lint` structured diagnostics.

**Code-level smells (tracked in #22):**

- `wos-core` exports L2/L3 modules in kernel crate (`crates/wos-core/src/lib.rs:22-37`)
- `ProvenanceKind` — 93-variant monolith enum (`crates/wos-core/src/provenance.rs`)
- `wos-runtime/src/runtime.rs` — 3821 lines, mixed dispatch
- `wos-formspec-binding` depends on `wos-runtime` — inversion
- `impactLevel` lives in kernel but is consumed only by governance (decided: stays; see #22)
- Kernel fixtures named after Layer 1/2/3 concerns (relocate under #22)
- `wos-correspondence-metadata.schema.json` under `schemas/kernel/` self-describes as sidecar (relocate under #22)
- `DRAFTS/` contains 12 kernel version proposals — triage before any schema/spec PR lands (see Next Steps)

**BPMN relationship** — Harel statechart semantics, not BPMN topology (`kernel/spec.md:636`). Appendix A acknowledges BPMN event-taxonomy adoption; #20 makes that adoption normative (typed union) rather than informative. Any durable execution runtime is valid (Kernel §A). Export path is via a `wos-bpmn-export` crate; WOS is the authoring surface.

### Non-goals

- Not BPMN parity.
- Not Formspec's 4-phase reactive processing model. Forms are frozen inputs; workflows are append-only event streams.
- Not demoting governance to "advisory presentation." Audit logs, notifications, deadlines are behavioral obligations.
- Not putting deontic operators in the extension registry — MUST/MAY/SHALL-NOT are core primitives.
- Not single-version response pinning. Cases outlive governance versions.
- Not eliminating BPMN. Export target, not authoring surface.

---

## Adopt

### #2 Deterministic Adverse-Decision Notice (dual-form) — **Imp 9 · Cx 7 · Debt 6**

- **Idea:** Adverse-decision notices (Governance §3.2) MUST be produced by a specified deterministic algorithm that derives two co-synchronized outputs from Facts + Reasoning tiers — a machine-readable artifact (structured, citable, diffable) and a human-prose artifact (plain language, suitable for legal service). Neither model-generated. Identical inputs MUST produce identical outputs in both forms.
- **Scaffolding (per code-scout validation):** `AdverseDecisionPolicy` typed but no required fields (permissive). `NoticeTemplate` has TWO conflicting definitions (Due Process sidecar = thin `sections: array of string`; Notification Template sidecar = richly typed with FEL conditions). `AppealRouting`, `ContinuationPolicy` typed. "Processor rejects missing sections" is actually a static lint (G-062 heuristic id-matching), not runtime rejection. `NoticeSent` is a hardcoded stub (`event_handler.rs:72-81`). Zero runtime rendering code.
- **Remaining work:** deterministic assembly algorithm + rendering pipeline + determinism fixtures + **NoticeTemplate reconciliation** (drop thin Due Process version; Notification Template is canonical).
- **Dependencies:** #24a (tightened Facts Tier) must land first. Part of unified ADR sequence #23 → #24a → #2. Delivery mechanism = Notification Template §4.4 (FEL-conditional sections + `requiredVariables` enforcement).
- **Why it matters:** WOS stakes its identity on rights-impacting due-process governance. Without this, implementers serve AI-generated text as legal notice or ship single-form output that diverges between machine and human views.

### #3 Policy-Based Migration Routing — **Imp 5 · Cx 6 · Debt 2**

- **Idea:** `migrationPolicy` enum: `grandfather | migrateAll | migrateByState | expression`. In-flight tasks complete under version-at-creation; workflow advances under new topology after.
- **Context:** Composes with Governance §2.9 `schemaUpgrade.migrationMechanism` (how) + `scope` (instance/workflow/tenant). #3 adds *when* and *which instances*. Runtime Companion §11 has manual `migrate` with grandfather semantics only; no policy enum exists.
- **Open sub-questions:** `tenant`-scope behavioral contract is undefined (zero code matches `tenant` — confirmed by validation). Version pinning on provenance records — do `migrateByState`/`expression` policies update `definitionVersion` at the boundary or only for records created after? Ownership of these questions lands with the #3 ADR.

### #12 Capability Preconditions — **Imp 6 · Cx 2 · Debt 4**

- **Idea:** `preconditions` array on agent capabilities: FEL expressions evaluated before invocation. Unsatisfied → skip, fall through to fallback chain.
- **Context:** `Capability` `$def` has four fields (`id`, `description`, `inputContractRef`, `outputContractRef`); `ActionOverride` similarly lacks it. Authors today embed precondition checks inside capability bodies — invisible to `wos-lint`, untestable in isolation, drain token/confidence budget before fallback fires.
- **Scope:** Schema property + FEL evaluation context (same as deontic constraints) + reuse existing fallback chain + 1-2 fixtures.

### #13 Verifiability Test Principle (kernel-level) — **Imp 4 · Cx 1 · Debt 1**

- **Idea:** Design-goal bullet in Kernel §1.2 + cross-references in Governance §6.1 and AI Integration §1.2: *"Can a second system, given only the spec and definition, cheaply verify behavior was correct?"*
- **Context:** Task-level verifiability exists (Governance §9.1 Verifiability Matrix). Kernel-level framing missing — leaves downstream readers unable to see why 4-tier provenance, processor-enforced guardrails, and calibrated confidence exist. Doc-only change.
- **Verification Report sidecar** is an artifact of this principle, not a replacement for it.

### #20 Typed Event Meta-Vocabulary (Delta 1) — **Imp 8 · Cx 6 · Debt 6**

- **Idea:** Replace `Transition.event: string` with `Transition.event: TypedEvent` — a strict enum-tagged union `{ kind: "timer" | "message" | "signal" | "condition" | "error", ... }`. **No `named` wrapper. No escape hatch.** Every user-authored event is typed as one of five kinds.
- **Context:** `Transition.event` is the kernel's single load-bearing openness (`wos-kernel.schema.json:348`). Subsumes the five timeout categories from Kernel §9.7. Aligns with Integration Profile `event-emit`/`event-consume` bindings. The eleven kernel-generated events (`$`-prefixed) are already closed and live outside the user-event typing.
- **Co-typing:** `Action.event` (used by `startTimer` actions, Kernel §9.2) must be co-typed or typed events become an untyped back-door for arbitrary event names.
- **Scope:** Schema change + migrate all 16/17 affected kernel fixtures + Rust update in `crates/wos-core/src/model/kernel.rs` + lint rule K-007 promoted to schema validation + `specs/kernel/spec.md §4.10` update + Appendix A row updated to reflect normative BPMN event-taxonomy adoption + conformance fixtures per kind.
- **Load-bearing sub-question:** timer calendar semantics (wall clock vs. business days). Directly determines whether `noticeGracePeriod` (Governance §3.2, ISO 8601 duration) means calendar or business days — a legal-compliance question. Irreversible once fixtures land.
- **Other sub-questions:** message correlation (explicit `correlationKey` vs. case ID); signal scope (case-local / parent-subcase / global); governance-hook interactions (deontic on timer expiry, due-process on message receipt).

### #21 Extension Registry (Delta 2, seams-only MVP) — **Imp 5 · Cx 4 · Debt 3**

- **Idea:** `schemas/registry/wos-extension-registry.schema.json` + `specs/registry/extension-registry.md`. **Seams-only MVP scope:** catalog the six kernel seams (§10) plus third-party extensions (e.g., Trellis custody shape). Lifecycle (draft → stable → deprecated → retired), composition semantics (conflict resolution on same-tag attachment), discovery (well-known URL + config override).
- **Future scope (separate ADRs):** typed event meta-kinds (after #20 lands), Integration Profile binding types, Semantic Profile ontology mappings. Don't bundle in the MVP.
- **Context:** Kernel §10 enumerates six seams authoritatively; registry is the **discovery layer** that catalogs those plus third-party extensions. Kernel enumeration stays canonical.
- **Role in closing escape hatches:** #46 closes `CaseRelationship.type` / `HoldPolicy.holdType` enums, #22 closes `custodyHook.additionalProperties`. Extensions that previously lived in those holes relocate to registry entries.

### #22 Crate Split Along Tier Boundaries (Delta 3) — **Imp 6 · Cx 6 · Debt 4**

- **Idea:** Split `crates/wos-core/` into `wos-kernel | wos-governance | wos-ai | wos-advanced`. Replace `ProvenanceKind` (93 variants) with tier-typed record. Invert `wos-formspec-binding → wos-runtime` dependency. Split `wos-runtime/src/runtime.rs` along action-kind dispatch. Add CI dependency-fence analogous to Formspec's `check:deps`. **Does NOT imply document split — monolithic document + tier-separated crates is the end state.**
- **In-scope cleanups:** relocate kernel-fixtures named after L1/L2/L3 concerns; relocate `schemas/kernel/wos-correspondence-metadata.schema.json` to `schemas/sidecars/`.
- **`custodyHook` decision:** close `additionalProperties: true` as part of #22. Trellis's shape becomes a registry entry under #21 with a typed namespace (`trellis.v1.*`) — gives schema validation *and* the escape hatch purpose.
- **Decided:** `impactLevel` stays in kernel. Runtime §2.4 requires kernel evaluation independent of governance outcome; `impactLevel` gates governance strength. **Caveat:** this reinforces a §2.4-written-by-same-hand circularity. Revisit if downstream specs expose the problem.

### #23 OverrideRecord Schema — **Imp 6 · Cx 2 · Debt 4**

- **Idea:** Promote Governance §7.3's three-field requirement (rationale + authority verification + supporting evidence) into an `OverrideRecord` `$def` in `wos-workflow-governance.schema.json`. Enforce at schema-validation time.
- **Context:** §7.3 is normative prose; override records resolve to Facts Tier `extensions` (untyped object). Bounded per-override debt — schema addition doesn't foreclose future shape changes.
- **Part of unified ADR sequence:** #23 → #24a → #2.

### #24a Mandatory Facts-Tier Input Snapshot — **Imp 8 · Cx 3 · Debt 7**

- **Idea:** Tighten Facts Tier §8.2: case-file input snapshot MANDATORY and typed at `determination`-tagged transitions (not current OPTIONAL untyped `inputs`).
- **Context validated:** 0 of 146 conformance fixtures populate `inputs`. Without the snapshot, replay determinism (axis 4) has no ground truth and due-process individualized-explanation (Governance §3.3) has no data to reference.
- **Silent dependency of #2.** Foundational — every spec built on optional untyped `inputs` requires rework if we tighten later.

### #24b Structured Rule-Firing Trace — **Imp 7 · Cx 6 · Debt 4**

- **Idea:** Reasoning Tier gets a structured trace: ordered list of rules evaluated, intermediate state, outcome. For AI-assisted decisions, includes calibration path.
- **Context:** `rulesApplied` (§6.2) captures *which* rules fired but not order or intermediate state. **Load-bearing coupling with #25** — evaluation order requires defeasibility answer. Joint design mandatory.

### #25 Defeasibility Primitive in Governance — **Imp 6 · Cx 7 · Debt 6**

- **Idea:** Catala-style default logic: declared rule priorities with specificity encoding. Not FEL-in-FEL hand-coding.
- **Context:** Absent anywhere in governance or advanced governance (verified). Assertion Gate Library has no priority/override/specificity construct.
- **Load-bearing sub-questions:**
  - New `DefeasibleRule` construct or extension of existing rule bundle?
  - Priority encoding — specificity, numeric, or topological over an `overrides` relation?
  - Distinct companion doc (`policy-defeasibility`) or folded into `workflow-governance`?
  - **Composition with `sourceAuthority` rank** (§6.2, 1-4). Authority and specificity are orthogonal; same-rank tie-breaking undefined today.
  - **Composition with Integration Profile §11.2** ("policy engine can restrict, never relax"). External OPA denies + defeasible WOS rule permits = which wins?

### #26a `AccessControl.canRead` Enforcement Semantics — **Imp 6 · Cx 3 · Debt 4**

- **Idea:** Specify normative processor behavior on `canRead(actorId, fieldPath) → false`: redact from evaluation context / return `null` / raise error / skip action. Conformance fixtures per branch.
- **Context validated:** Interface exists (`traits/mod.rs:106-115`), defaults `true`, called nowhere in runtime. Pure stub. Prerequisite to #26b.

### #26b `caseFieldPolicy` Schema — **Imp 6 · Cx 6 · Debt 4**

- **Idea:** `caseFieldPolicy` `$def` in `wos-workflow-governance.schema.json` — per-field read/write scopes by actor role, referencing kernel `caseFile` field identifiers.
- **Decided:** governance-layer, not kernel. Kernel defines case file; field-level visibility depends on actor roles, impact level, and hold policies (all L1).
- **Load-bearing sub-questions:**
  - Compose with AI Integration L2 `Right` (actor policy first, then agent filter) or supersede?
  - Interaction with hold policies and redacted provenance.
  - **Assurance Invariant 6** — disclosure posture and assurance level MUST remain independent predicates. A `caseFieldPolicy` that conflates the two violates it.

### #27 Cancellation Regions — **Imp 4 · Cx 6 · Debt 3**

- **Idea:** YAWL-style named region: explicit set of tasks/states spanning arbitrary structural levels, fireable as a unit.
- **Context:** `cancellationPolicy ∈ {wait-all, cancel-siblings, fail-fast}` at `kernel/spec.md §4.4` is a join policy on parallel-state co-regions, not a cross-structural scope.
- **Open sub-questions:** region as state-ID set or predicate? Fired by event / guard / explicit action? Compensation — run / skip / author's choice?

### #28 Claim-Check Artifact References — **Imp 4 · Cx 4 · Debt 3**

- **Idea:** Typed `ExternalArtifactRef { uri, contentHash, hashAlgorithm, mediaType }` usable as a case-field value with normative integrity-check requirement at retrieval.
- **Context:** One weak precedent — `wos-correspondence-metadata.schema.json` `CorrespondenceEntry.contentRef` (untyped string, no hash). Code-scout REFUTED the claim that `inputDigest`/`outputDigest` provide kernel-level precedent — zero code, no struct field, no population. Treat #28 as **net-new integrity mechanism**.
- **Decided:** type at kernel (case-field values are kernel-owned); retrieval contract at governance via `contractHook`.
- **Bundled scope:** wire `inputDigest`/`outputDigest` into `ProvenanceRecord` type as part of #28 (filling the spec-prose-only gap discovered during validation).

### #29a Milestone Spec-Lag Closure — **Imp 5 · Cx 2 · Debt 5**

- **Idea:** Update Kernel §4.13 prose and Milestone schema to normatively describe KS.2's shipped behavior. Sync-point question was *resolved in code* by validation pass.
- **Context from code-scout:** Runtime evaluates milestones **once per "write-then-react" envelope** — post-transition-settled (`runtime.rs:602-609`), post-integration-binding, post-task-response-merge. Dedup via `fired_milestones` set. K-M-001..005 fixtures prove observable firing.
- **Scope:** Spec prose + trivial `triggerMode: "writeSettled"` schema property reflecting shipped policy.
- **`continuous` mode interaction:** Moot — `continuous_reevaluate` is dead code, never called from runtime.

### #29b Milestone Reactive Transition Firing (GSM-style) — **Imp 6 · Cx 5 · Debt 2**

- **Idea:** Make `MilestoneFired` enqueue an event transitions can react to, or expose `$milestone.*` FEL variable for guards. This is the GSM-style half KS.2 did NOT ship.
- **Context:** `evaluate_milestones` only appends provenance; does not enqueue events or mutate state. No active fixture references a transition conditioned on a milestone. `$milestone.*` appears only in `DRAFTS/`.
- **Genuine new capability** — ships after #29a lands.
- **Open sub-questions:** event-based (enqueue synthetic event) or guard-based (expose `$milestone.foo` as FEL boolean)? Interaction with `fired_milestones` dedup (re-fire requires reset?).

### #30 WS-HumanTask Lifecycle Completion — **Imp 5 · Cx 4 · Debt 2**

- **Idea:** Extend 8-state model (`created | assigned | claimed | completed | failed | delegated | escalated | skipped`) to close WS-HumanTask gap: task-level `Suspended` (WOS holds are case-level today), distinct `Cancelled` terminal (separate from `skipped` = "not applicable"), explicit `Return` with rework-iteration counter, forwarding-to-group distinct from delegation-to-person.
- **Design note:** Task-level `Suspended` may be reducible to case-level hold with `holdType: task-suspended`, reusing `lifecycleHook` seam. Open sub-question: always reducible, or do operational cases need genuinely independent task state?

### #31 Jurisdiction-Aware Business Calendar Selection — **Imp 6 · Cx 3 · Debt 4**

- **Idea:** Runtime resolution of which business calendar applies from a case-file field (e.g., `applicant.jurisdiction`), replacing current "implementation-defined" selection in `sidecars/business-calendar.md §7`.
- **Context:** Multi-calendar composition is defined; selection isn't. In multi-jurisdiction rights-impacting workflows, implementation-defined selection is a compliance risk (two conformant processors can calculate the same legal deadline differently). Not operational polish — real.

### #34 `x-lm.critical` Enforcement Gate — **Imp 6 · Cx 1 · Debt 2**

- **Idea:** CI gate (`docs:check` rule) rejecting schema PRs where `x-lm.critical: true` nodes lack `description` or `examples`.
- **Context:** 131 critical nodes across 18 schemas; **current violations = 0** (verified). Pure regression prevention. Assurance schema has zero `x-lm.critical` annotations (see #57).

### #35 Equity Config Enforcement Semantics — **Imp 7 · Cx 5 · Debt 5**

- **Idea:** Specify processor obligations for `RemediationTrigger.action` (`review | audit | suspend | notify`); wire `DisparityMethod` evaluation to runtime schedule per `ReportingSchedule`; define "suspended workflow" behaviorally. Conformance fixtures.
- **Context:** Equity Config sidecar is fully specified structurally: `ProtectedCategory`, 4 statistical methods, automated reporting, remediation triggers. Only 1 conformance fixture (AG-001) tests downstream alert handling; zero code reads Equity Config. **Applies to human AND AI decisions** per spec — civil-rights concern, not AI-specific.
- **Depends on #36.**

### #36 Equity RemediationTrigger Expression Language — **Imp 6 · Cx 4 · Debt 4**

- **Idea:** Specify expression language for `RemediationTrigger.condition`. Schema declares untyped `string`; prose examples use constructs FEL can't express (*"disparity > 0.20 for 2 consecutive periods"*). Decide: (a) extend FEL with temporal/windowing operators, (b) restricted DSL, (c) FEL + windowing functions.
- **Depends on:** Kernel §7.4 grammar-extension stance (currently rejects extensions). Option (a) requires revisiting that stance.

### #37 Drift Monitor Demotion Policy Binding — **Imp 6 · Cx 3 · Debt 5**

- **Idea:** Normative binding from `alertThresholds[].action` to `DemotionRule`. Candidate: `alertThresholds[].policyRef` referencing a named demotion rule.
- **Context:** Promoted to active Adopt after M-1 merge was blocked by standalone Drift Monitor fixture (`fixtures/ai/benefits-drift-monitor.json`). Two specs describe same mechanism from opposite sides; no link.

### #38 Assertion Library Cross-Document Reference Protocol — **Imp 5 · Cx 3 · Debt 3**

- **Idea:** `assertionId` (or `assertionLibraryRef`) on `PipelineStage.assertions[]`, resolution semantics (library lookup order, version pinning).
- **Context:** Library defines `AssertionDefinition`; `PipelineStage.assertions` takes inline array; no reference property. The library concept exists in prose; the mechanism making it a *library* doesn't.

### #39 ContinuationPolicy Normative Linkage — **Imp 4 · Cx 2 · Debt 3**

- **Idea:** Specify how `AppealMechanism.continuationOfServices: true` resolves to a specific `ContinuationPolicy` entry. Candidate: `AppealMechanism.continuationPolicyRef`.
- **Context:** Promoted to active Adopt after M-2 merge was rejected (Notification Template confirmed to have non-due-process uses across 4 categories in shipped fixture).

### #40 Task SLA Authoring Surface — **Imp 6 · Cx 5 · Debt 5**

- **Idea:** Add task SLA schema properties (`slaDefinitions`, `warningThresholds`, `breachPolicy ∈ {escalate | reassign | notify | extend}`, `escalationChain`). Today §10.3 specifies these as normative processor behavior with no schema properties — explicitly: *"not declared in the Workflow Governance Document."*
- **Why reverse:** Normative obligations without an authoring surface are half-specs. Implementers must encode task SLAs outside WOS documents, producing non-conformant artifacts invisible to validation.

### #42 Autonomy-Lifecycle Conformance Fixture Batch — **Imp 5 · Cx 2 · Debt 2**

- **Idea:** Two fixtures (narrowed after validation): (1) escalation-expiry revocation (`EscalationRule.escalationExpiry`); (2) drift-alert-triggered demotion (Drift Monitor `alertThresholds[]` firing `DemotionRule`).
- **Already covered by existing fixtures:** calibration-expiry (AC-001), humanOverride-triggered demotion (ai-028/ai-029). Narrowed accordingly.

### #43 Assurance × Impact-Level Composition Rule — **Imp 6 · Cx 5 · Debt 4**

- **Idea:** Specify whether a minimum Assurance level is required for AI-assisted determinations at `rights-impacting` impact level. Respect Invariant 6 (independence of disclosure posture and assurance level).
- **Why foundational, not anticipatory:** Must be decided *before* any production deployment, otherwise the combined spec leaves a load-bearing question unanswered. Two orthogonal axes (impact, assurance) both bear on high-stakes decisions with no composition rule.

### #45 Sidecar Normative-Contract Audit — **Imp 6 · Cx 5 · Debt 5**

- **Idea:** Audit every sidecar against:
  - **Step 0 (new):** Does this sidecar deserve independent existence? (distinct semantic model or distinct artifact lifecycle)
  - **Step 1–3:** Three-question rubric (Structure / Semantics / Composition) on survivors.
- Apply the template in `wos-spec/CONVENTIONS.md` (see F-2). Retrofit all existing specs — not prospective-only.

### #46 Schema-Prose Enum Alignment Batch — **Imp 4 · Cx 2 · Debt 3**

- **Idea:** Batched schema tightenings aligning with normative prose:
  1. **`CaseRelationship.type`** — close to enum. Domain-specific relationship types route through #21 registry, **not** an `x-*` pattern escape hatch.
  2. **`HoldPolicy.holdType`** — close to enum. Reconcile three-way disagreement between §12.2 table, §7.15, and schema on `legal-hold` placement. Domain-specific hold types route through #21 registry.
  3. **`AppealMechanism.reviewerConstraint`** — tighten to required with enum including `independentFromOriginal` (aligns schema with §3.5 MUST). **Plus:** `AppealMechanism.continuationScope` has same shape smell — batch.
  4. **`DelegationScope.conditions`** — add FEL evaluation context citation to Runtime §8.2 (matches other FEL fields).
  5. **ISO 8601 duration `pattern`** — `AppealMechanism.appealWindow`, `HoldPolicy.expectedDuration`, similar fields. Batch-add validation pattern.
  6. **Drift Monitor `AlertThreshold` prose table** — add missing prose table to `drift-monitor.md` (schema `$def` already exists).

### #47 Provenance Export (PROV-O first) — **Imp 7 · Cx 8 · Debt 5**

- **Idea:** Serialize internal provenance to W3C PROV-O. 93 `ProvenanceKind` variants.
- **Scope reduced:** PROV-O first. OCEL 2.0 and IEEE XES as follow-up items once PROV-O lands and exposes subsystem mapping decisions. The ambiguous subsystems (deontic, autonomy, confidence) force design work that should land once, not three times.
- **Variant classification:** ~48% obvious mapping (lifecycle, timers, tasks, events), ~32% needs design (deontic has no native PROV vocabulary; autonomy "computed" derivation has no PROV anchor; confidence decay is time-based erosion; "blocked-action" records handle awkwardly), ~20% payload-convention-dependent (`data: Option<Value>` is untyped JSON).
- **Interacts with:** JSON-LD decision (see Open Questions). PROV-O is RDF-native; shipping PROV-O export effectively stakes out a `@context` position for provenance.

### #48 Merkle Provenance Chains — **Imp 6 · Cx 6 · Debt 4**

- **Idea:** Cryptographic hash-chaining for tamper-evident provenance logs. Append-only, signed tree heads, inclusion proofs. Attaches via Assurance `provenanceLayer` seam.
- **Context:** Matches research corpus R1 (SCITT / RFC 9162). Assurance §5.2 explicitly excludes cryptographic signing — net-new, but clear landing zone. Depends on #47 (stable format to hash against).
- **Scope:** Hash-chaining only initially (lightest path). Full SCITT transparency-service integration as later ADR.

### #52 Simulation Trace Format — **Imp 4 · Cx 4 · Debt 2**

- **Idea:** Standardized replay format for simulation runs — validation, tooling, regression testing. Reuses XES event log format from #47 follow-up.
- **Depends on #47** baseline.

### #56 Runtime §2 Isolation-Invariant Lint Rule — **Imp 5 · Cx 2 · Debt 3**

- **Idea:** Static AST lint detecting `setData` → guard dependency cycles in `continuous`-mode documents. §2.4 invariant is normative but unvalidated.
- **Context:** Code-scout confirmed `continuous_reevaluate` is dead code — lint prevents future defective documents from shipping even though convergence scenario can't manifest today.

### #57 Assurance Schema `x-lm.critical` Coverage — **Imp 3 · Cx 1 · Debt 2**

- **Idea:** Add `x-lm.critical: true` annotations to key nodes in `schemas/assurance/wos-assurance.schema.json`. Currently zero — only schema in the suite without any.
- **Context:** Surfaced by #34 validation scan.

---

## Defer

Items with real reactivation triggers (not speculation). "Defer" means captured but not active; re-score when the named trigger fires.

### #1 Agent Behavioral Attestations

- **Idea:** SLSA-style `attestations` on agent definitions (`issuer`, `subject`, `claims`, `verificationMethod`). Processor validates autonomy claims against attestations; lint warns on unattested agents at `rights-impacting` impact.
- **Context:** Capability attestation, orthogonal to Assurance §5's identity attestation (no overlap). **Premature** — no SLSA-style AI-attestation issuer ecosystem exists. Waiting reduces risk: the format should be shaped by whoever builds the ecosystem.
- **Trigger:** SLSA-style AI-agent attestation ecosystem matures OR specific deployment demands capability attestation.

### #4 Tripartite Object Model

- **Idea:** Split into ActivityDefinition / WorkflowDefinition / Task as separate documents.
- **Context:** Monolithic confirmed (research corpus reinforces). Architecturally expensive to unwind, but low lock-in — monolithic is a superset of tripartite; split is mechanical.
- **Trigger:** Activity-definition reuse across workflows becomes a real pattern.

### #6 Typed Patch Operations

- **Idea:** AST-level edits with 4-stage validation.
- **Context:** No authoring tool exists yet. Tech Debt 0.
- **Trigger:** Authoring tool ships structural edits.

### #7 OCEL 2.0 Object-Centric Case Model

- **Idea:** Typed objects with E2O relationships as *internal* case state model.
- **Context:** OCEL 2.0 is already a provenance export target (Semantic Profile §6.4 — see #47). Export surface acts as watching post for systematic lossy mappings.
- **Trigger:** Multi-object mutation patterns emerge, or flat→OCEL export shows systematic semantic loss.

### #9 JSON-LD Export Surface (reopened)

- **Idea:** JSON-LD `@context` for WOS documents, targeting export compatibility with PROV-O / OCEL / schema.org / NIEM.
- **Context reopened 2026-04-16:** Prior rejection ("plain JSON/YAML, semantic web as companion layer") dates to v3–v5. #47 targets PROV-O (RDF-native) export — rejecting JSON-LD for authoring while embracing RDF for export is incoherent. Ontology-spec work (TODO §2, unwritten) will force the question. Government linked-data mandates accelerating.
- **Trigger:** `ontology-spec.md` drafts begin OR #47 lands and exposes context-bag design decisions.
- **Scope note:** Reopened as *export surface design*, not wholesale JSON-LD-native authoring. Plain JSON wire format stays.

### #32 Multi-Instance Iteration

- **Idea:** Iteration over events or case-data arrays.
- **Context:** The one orchestration pattern with NO viable workaround in current primitives. Governance-hooks-per-instance vs. per-iteration is load-bearing (audit-trail volume implications for 100-item batches).
- **Trigger:** #20 lands. Then highest-priority deferred item.

### #33 Inclusive-OR / Event-Choice / Boundary Events

- **Idea:** Orchestration patterns with ugly-but-viable workarounds.
- **Context:** Event-based choice via parallel regions with `fail-fast`; boundary events via parallel regions with event-driven exits; inclusive-OR awkward but encodable.
- **Trigger:** Authoring frustration with workarounds (externally observable signal, e.g., issue filings).

### #49 Engine Adapters (Camunda / Temporal / Step Functions)

- **Idea:** External commercial engines act as the WOS runtime.
- **Context:** Orthogonal to Integration Profile (that covers WOS invoking external; this covers external invoking WOS). Requires #20.
- **Trigger:** First commercial deployment requesting a specific adapter.

### #50 EU AI Act Art. 13–14 / OMB M-24-10 Alignment

- **Idea:** Normative alignment spec mapping WOS constructs to specific article requirements.
- **Context:** External-deadline-driven. Citations exist in Governance §1.1; normative alignment doesn't. **Watchlist** — external compliance deadlines can force escalation without notice.
- **Trigger:** Procurement deadline OR regulatory inquiry.

### #51 Statutory Deadline Chains

- **Idea:** Interdependent government deadlines and automated legal consequences (e.g., 30-day notice → 10-day response → 5-day finalization chains).
- **Context:** No chained-deadline construct exists. Architecturally expensive — wrong abstraction here is expensive.
- **Trigger:** First deployment needing chained legal deadlines.

*Removed from Defer during 2026-04-16 greenfield audit:*
- **#5 DAG Processing Model** → moved to Reject (contradicts axis 4).
- **#17 SHACL** → stays in Reject (existing Rust lint covers cross-doc; SHACL adoption would duplicate; JSON-LD reopening doesn't resurrect SHACL wholesale).
- **#44 Prospective/simulation values** → Deleted (pure anticipation, additive later with zero lock-in).
- **#53 Full lifecycle soundness verification** → Deleted (Advanced Governance SMT is the path; Petri-net/LTL would fracture verification story).
- **#54 JSON Patch provenance** → Deleted (tool choice, re-invocable in 3 lines of design).
- **#55 FEEL-to-FEL migration guide** → Deleted (on-demand doc; write when first DMN shop asks).

---

## Reject

- **#5 DAG Processing Model** — Contradicts axis 4 (append-only event-stream folding). Reactive re-evaluation is the rejected alternative; the decision is committed at the axis level.
- **#8 FEL Conformance Profiles** — Kernel §7.4 rejects grammar extensions.
- **#10 WCOS + FEEL** — Rename + DMN-expression-language. Both abandoned.
- **#17 SHACL** — Existing Rust lint (55 T2 rules) covers cross-doc validation. SHACL adopting would duplicate with RDF-native validator. If #47 needs export-shape validation, that becomes a scoped "PROV-O export validation" item, not a SHACL resurrection.
- **#18 Minimal Governance Envelope** — Strip lifecycle from kernel; produces doc that cannot be understood in isolation.
- **#19 FEEL Expression Language** — FEL is purpose-built; FEEL carries DMN assumptions.
- **BPMN Parity as Authoring Goal** — Export target, not authoring surface. Topology rejected; event taxonomy adopted (normative via #20).

---

## Shipped

Distinct from Architectural Decisions Confirmed. These are normative features with code, schema, fixtures, and spec prose — not pending decisions.

- **Null behavior on deontic constraints** — `nullBehavior` on Permission/Prohibition/Obligation with impact-level defaults. `ai-integration.md §4.2-4.5 + §5`; `NullBehavior` `$def`. (Formerly #11.)
- **Arazzo integration sequences** — Multi-step API orchestration via Arazzo references. `integration.md §3.5`; fixtures `INT-ARAZZO-001..003`. (Formerly #14.)
- **Non-HTTP tool invocation** — `tool` binding kind supporting `command-line`, `batch-file`, `database-procedure`, `graph-query`. `integration.md §3.6`; fixtures `INT-TOOL-001..002`. (Formerly #15.)
- **Assist Governance Proxy** — Deontic constraint enforcement on Formspec Assist tool calls. `ai-integration.md §14`; schema `AssistGovernanceProxy`. Stabilizes with Assist layer upstream. (Formerly #16.)
- **RFC 9535 `outputBinding` profile** — Explicit inclusion (member access, index, wildcard, slice); exclusion (recursive descent, filter expressions); rejection MUST at load time (lint I-001). `integration.md §3.3.1`. (Shipped NB.2.)
- **CloudEvents correlation key format** — `{instanceId}:{bindingId}:{invocationId}`. `integration.md §6`. (Shipped NB.3.)
- **`finiteDomainDeclarations`** — SMT-supporting schema-level domain enumerations. Advanced Governance `VerifiableConstraint`. (Shipped AG010.)

---

## Architectural Decisions Confirmed

Decisions that hold under re-audit from first principles (2026-04-16 greenfield lens).

- **Constraint zones as overlay**, not kernel state type. Implementations shouldn't require DCR understanding; five state types violates KISS.
- **Monolithic document over tripartite** — reinforced 2026-04-16 by research corpus (compass Direction 3 + statechart lifecycle). Not preserved by fixture inertia.
- **Event-driven evaluation over DAG** — committed at axis 4. Reactive re-evaluation explicitly rejected (#5 in Reject).
- **FEL over FEEL** — purpose-built.
- **Kernel includes lifecycle** — coherent single-document understanding (§18 rejected).
- **Granular decomposition over kernel/profile binary** — target sidecar count ~12, determined per sidecar under the new keep-separate test below.
- **Hybrid layered architecture with statechart lifecycle** — compass Direction 3 + Direction 1 validated.
- **BPMN as export target, not authoring surface** — topology rejected; event taxonomy adopted normatively via #20.
- **`impactLevel` stays in kernel** — Runtime §2.4 requires governance-independent kernel eval; `impactLevel` gates governance strength. Caveat: decision and §2.4 share authorship circularity; revisit if downstream specs expose the problem.
- **Sidecar keep-separate test** — a sidecar earns independent existence when it has a **distinct semantic model** or **distinct artifact lifecycle**, not because "regulators might update it" (anticipatory / no deployments exist). Applied per Section I of 2026-04-16 greenfield audit. Survivors: Policy Parameters, Business Calendar, Equity Config, Assurance, Integration Profile, Lifecycle Detail, Runtime Companion, Advanced Governance, Notification Template, Due Process Config (partial merge pending per #45 step 0), Workflow Governance (absorbs Assertion Library), Advanced Governance (absorbs Verification Report).

---

## Structural Merges

### M-1 · Drift Monitor + Agent Config — BLOCKED

Merge blocked by `fixtures/ai/benefits-drift-monitor.json` shipping standalone Drift Monitor without paired Agent Config. **Ship #37 standalone binding instead.** Reconsider M-1 if/when the fixture is revised (Next Step item).

### M-2 · Notification Template + Due Process Config — REJECTED

Notification Template has confirmed non-due-process uses across 4 categories (`adverse-decision`, `hold-notification`, `appeal-acknowledgment`, `sla-warning`) in the single shipped fixture. Spec enumerates 6 categories. **Ship #39 standalone linkage instead.**

### Merges to execute

- **Assertion Library → Workflow Governance** as a "Named Assertions" section. Library without #38 reference protocol is incomplete; absorb rather than fix.
- **Verification Report → Advanced Governance** as an "Output Artifacts" section. Thin sidecar; doesn't justify independent spec status.
- **Due Process Config partial merge → Workflow Governance** (pending #45 step 0). If the thin NoticeTemplate is dropped (per #2) and AppealRouting + ContinuationPolicy are the remaining content, the merge closes the `ContinuationPolicy` ↔ `AppealMechanism.continuationOfServices` linkage gap structurally.

Target sidecar count: **12** (from 18).

---

## The Genuine Invention

Every item has component prior art in domain literatures. The consistent novelty across all items is **"declarative encoding as schema-enforceable workflow primitives"** — not the concepts themselves.

1. **Deontic operators as schema-enforced primitives.** Prior art: LegalRuleML (operators), von Wright deontic logic (1951). Novel: processor-enforced null behavior with impact-level defaults, wired to autonomy caps.
2. **Structured oversight modes as declarative schema.** `independentFirst`, `considerOpposite`. Prior art: cognitive debiasing literature, QA frameworks. Novel: declarative encoding in workflow spec.
3. **Due process as schema-enforceable workflow primitives.** Prior art: administrative law (Goldberg, Mathews), FedRAMP/OMB policy encoding. Novel: spec-level enforcement with conformance fixtures.
4. **4-tier provenance layering (Facts / Reasoning / Decision / Narrative).** Prior art: PROV-O agent/entity/activity. Novel: distinct epistemic-tier layering vs. PROV's actor-role model.
5. **Authority-ranked reasoning with confidence composition.** Prior art: LegalRuleML authority hierarchy, evidential reasoning. Novel: composition of authority rank × calibrated confidence as a reasoning-trace primitive.
6. **Impact-level-dependent behavior as schema-enforced processor obligations.** Prior art: OMB M-24-10, EU AI Act risk tiers, FedRAMP Low/Mod/High. Novel: declarative workflow-spec encoding.
7. **Normative civil-rights monitoring as workflow-spec primitive.** Prior art: EEOC 4/5ths rule (1978), AIF360, Fairlearn, disparate-impact statistical methods. Novel: declarative encoding with automated remediation triggers scoped to both human AND AI decisions.
8. **Normative binding of drift detection to autonomy demotion.** Prior art: ML-ops drift detection (Evidently, Arize, WhyLabs). Novel: normative binding in workflow spec — drift detection alone is not novel.

---

## Capability Status (vs. Research Corpus)

2026-04-16 audit cross-checked 12 research-corpus "missing from all systems" capabilities. Tracked via Adopt/Defer entries; see individual scores.

- **Implemented:** Temporal parameter versioning (OpenFisca-style, `policy-parameters.md`); separation of duties (specified `§7.2`, runtime enforcement via #22 cleanup); business-calendar-aware SLAs (calendar shipped; jurisdiction selection in #31); AI confidence annotations (`§6.3`, `§7.1`).
- **Partial / in flight:** decision provenance (#24a + #24b); override accountability (#23); WS-HumanTask lifecycle (#30); GSM artifact-centric progression (#29a + #29b); equity monitoring (#35 + #36); role-scoped visibility (#26a + #26b); AI drift governance (#37 + M-1 pending).
- **Not implemented (tracked):** defeasibility (#25); cancellation regions (#27); claim-check (#28); Assurance × impact composition (#43).

---

## Cross-Project Dependencies

Stability of certain WOS constructs is gated by external work:

- **Formspec Assist** — `ai-integration.md §14` proxy stabilizes when Assist upstream stabilizes.
- **Formspec Core** — FEL grammar (Kernel §7.4 imports `fel_core`). Affects #36 (equity expression language).
- **Trellis** — `custodyHook.additionalProperties` escape hatch will close via #22; Trellis shape relocates to #21 registry entry.

---

## IDEA ↔ TODO Reconciliation

Cross-references between IDEA (design backlog) and TODO.md (execution tracker):

| IDEA | TODO | Note |
|---|---|---|
| #2 | §4 "Deterministic adverse-decision notice" | Already cross-referenced from TODO |
| #28 | §6 "Claim check pattern" | Add cross-ref |
| #26a/#26b | §6 "Role-based field visibility" | Add cross-ref |
| #47 | §2 "Provenance export" | IDEA = design, TODO = execution |
| #48 | §4 "Merkle provenance chains" | Same |
| #49 | §3 | Same |
| #50 | §5 | Same |
| #51 | §6 "Statutory deadlines" | Same |
| #52 | §4 "Simulation trace format" | Same |
| — | §2 "Ontology field identity" | Unwritten spec; informs #9 trigger |
| — | TODO future: Batch / Federation / Learning | Trigger-gated; not in IDEA |

TODO audit date needs refresh to reflect 2026-04-16 work.

---

## Implementation Priority

Scored on three 0-10 axes. Derived: **Urgency = (Imp + Debt) / Cx** — captures "this matters AND the lock-in bites if we wait."

**Note on precision:** Urgency values below ~2.5 are within noise; rank order among those is approximate.

### Tech Debt rubric (greenfield reframe)

Tech Debt = **architectural lock-in risk**, not deployment convention drift.

- **9-10** — Wrong choice here locks every downstream spec into a wrong abstraction; retrofit is a rewrite.
- **7-8** — Foundational. Wrong choice propagates to multiple specs; fixable with significant effort.
- **5-6** — Confined to one spec/surface. Fixing is schema migration.
- **3-4** — Additive or cosmetic. Fixing is trivial.
- **1-2** — Purely additive; no architectural commitment.
- **0** — Dependency-gated or consumer-free; waiting is strictly neutral.

### Adopt — ranked by Urgency

| Item | Imp | Cx | Debt | Urgency | One-line case |
|------|----:|---:|-----:|--------:|---------------|
| **#34 `x-lm.critical` enforcement gate** | 6 | 1 | 2 | 8.0 | CI gate; 0 current violations, pure regression prevention. |
| **#23 OverrideRecord schema** | 6 | 2 | 4 | 5.0 | Due-process compliance, schema-checkable. |
| **#29a Milestone spec-lag closure** | 5 | 2 | 5 | 5.0 | Close live spec/runtime drift. |
| **#57 Assurance `x-lm.critical` coverage** | 3 | 1 | 2 | 5.0 | Zero annotations today — fills 18th schema. |
| **#13 Verifiability test principle** | 4 | 1 | 1 | 5.0 | Doc-only; three cross-refs. |
| **#24a Mandatory Facts-Tier input snapshot** | 8 | 3 | 7 | 5.0 | Unblocks #2 + #23; foundational. |
| **#12 Capability preconditions** | 6 | 2 | 4 | 5.0 | Small schema delta; saves token budget pre-fallback. |
| **#42 Autonomy-lifecycle fixture batch** | 5 | 2 | 2 | 3.5 | Escalation-expiry + drift-triggered demotion. |
| **#56 Runtime §2 isolation lint rule** | 5 | 2 | 3 | 4.0 | Static cycle detector (continuous mode currently unwired). |
| **#37 Drift Monitor demotion binding** | 6 | 3 | 5 | 3.67 | Promoted after M-1 blocked. |
| **#46 Schema-prose enum alignment batch** | 4 | 2 | 3 | 3.5 | Close enums; route extensions via #21 registry. |
| **#39 ContinuationPolicy normative linkage** | 4 | 2 | 3 | 3.5 | Promoted after M-2 rejected. |
| **#26a canRead enforcement semantics** | 6 | 3 | 4 | 3.33 | Interface exists as stub; spec the behavior. |
| **#31 Jurisdiction-aware calendar selection** | 6 | 3 | 4 | 3.33 | Multi-jurisdiction compliance risk. |
| **#38 Assertion library cross-doc reference** | 5 | 3 | 3 | 2.67 | `assertionId` on pipeline stages. |
| **#36 Equity RemediationTrigger expression language** | 6 | 4 | 4 | 2.5 | Prerequisite to #35. |
| **#2 Deterministic adverse-decision notice** | 9 | 7 | 6 | 2.14 | Assembly algorithm + real rendering + NoticeTemplate reconciliation. |
| **#35 Equity Config enforcement semantics** | 7 | 5 | 5 | 2.4 | Civil-rights claim actionable. |
| **#20 Typed event meta-vocabulary (Delta 1)** | 8 | 6 | 6 | 2.33 | Closes load-bearing openness; strict five kinds, no `named`. |
| **#45 Sidecar normative-contract audit** | 6 | 5 | 5 | 2.2 | Step 0: re-audit independence. |
| **#40 Task SLA authoring surface** | 6 | 5 | 5 | 2.2 | Add schema for §10.3. |
| **#43 Assurance × impact-level composition** | 6 | 5 | 4 | 2.0 | Foundational before deployment. |
| **#22 Crate split (Delta 3)** | 6 | 6 | 4 | 1.67 | Engineering hygiene. |
| **#25 Defeasibility primitive** | 6 | 7 | 6 | 1.71 | Joint design with #24b. |
| **#24b Structured rule-firing trace** | 7 | 6 | 4 | 1.83 | Joint design with #25. |
| **#28 Claim-check artifact references** | 4 | 4 | 3 | 1.75 | Net-new integrity mechanism. |
| **#30 WS-HumanTask lifecycle completion** | 5 | 4 | 2 | 1.75 | `Suspended` / `Cancelled` / `Return`. |
| **#26b caseFieldPolicy schema** | 6 | 6 | 4 | 1.67 | Multi-role confidential cases. |
| **#48 Merkle provenance chains** | 6 | 6 | 4 | 1.67 | Tamper-evident logs; depends on #47. |
| **#47 Provenance export (PROV-O first)** | 7 | 8 | 5 | 1.5 | Foundational; unlocks #48/#52. |
| **#21 Extension registry (seams-only MVP)** | 5 | 4 | 3 | 2.0 | Scope narrowed to seams + Trellis registry use. |
| **#29b Milestone reactive firing (GSM-style)** | 6 | 5 | 2 | 1.6 | New capability after #29a lands. |
| **#3 Policy-based migration routing** | 5 | 6 | 2 | 1.17 | Composes with §2.9; tenant-scope behavioral gap. |
| **#52 Simulation trace format** | 4 | 4 | 2 | 1.5 | Depends on #47. |
| **#27 Cancellation regions** | 4 | 6 | 3 | 1.17 | YAWL-style regions. |

### Defer — by trigger

| Item | Imp | Cx | Debt | Trigger |
|------|----:|---:|-----:|---------|
| **#32 Multi-instance iteration** | 6 | 7 | 5 | #20 lands. |
| **#7 OCEL 2.0 object-centric case** | 2 | 9 | 5 | Multi-object mutation or #47 export shows systematic loss. |
| **#50 EU AI Act / OMB M-24-10 alignment** | 7 | 5 | 4 | Procurement deadline or regulatory inquiry (watchlist). |
| **#9 JSON-LD export surface** | 5 | 5 | 3 | `ontology-spec.md` drafts begin or #47 exposes context decisions. |
| **#49 Engine adapters** | 5 | 8 | 3 | First commercial deployment requesting adapter. |
| **#4 Tripartite object model** | 2 | 9 | 3 | Activity-definition reuse becomes real pattern. |
| **#51 Statutory deadline chains** | 4 | 7 | 3 | First deployment needing chained legal deadlines. |
| **#33 Inclusive-OR / event-choice / boundary events** | 3 | 5 | 2 | Authoring frustration (externally observable signal). |
| **#1 Agent behavioral attestations** | 2 | 7 | 1 | SLSA-style attestation ecosystem matures. |
| **#6 Typed patch operations** | 1 | 8 | 0 | Authoring tool ships structural edits. |

Dropped from Defer during 2026-04-16 audit (deleted or moved): #5 (→Reject), #17 (stays Reject), #44, #53, #54, #55 (deleted).

---

## Dependencies

```text
#34 (x-lm.critical gate) — no dependencies
#57 (assurance x-lm.critical) — no dependencies
#29a (milestone lag closure) — no dependencies; #29b follows

Provenance-record-shape ADR sequence:
  #23 OverrideRecord ──┐
  #24a Input snapshot ──┼──> #2 Deterministic notice
                        │      (+ NoticeTemplate reconciliation: drop thin Due Process version)
  Notification Template ─┘      (delivery mechanism)

Joint design:
  #25 Defeasibility ──joint──> #24b Rule-firing trace
  Must compose with sourceAuthority rank (§6.2) AND Integration Profile §11.2.

#20 (typed events) ──┬──> #32 Multi-instance (deferred)
                     │      (+ co-type Action.event for startTimer)
                     └──> #46 enum batch (routes extensions via #21)

#21 (registry) ──> catalogs seams + Trellis shape (closing #22 custodyHook escape)

#22 (crate split) ──┬──> CI dependency fence
                    ├──> kernel fixture relocation
                    └──> correspondence-metadata relocation

#26a (canRead semantics) ──prerequisite──> #26b (caseFieldPolicy)
#36 (equity expression language) ──prerequisite──> #35 (equity enforcement)
#47 (PROV-O export) ──┬──> #48 Merkle chains
                      └──> #52 Simulation trace
#9 (JSON-LD export surface) ──gated by──> ontology-spec.md drafts OR #47 decisions

M-1 BLOCKED ──ship #37 standalone
M-2 REJECTED ──ship #39 standalone
```

---

## Open Questions

1. **Timer semantics precision** (#20). Wall-clock or business-days for `noticeGracePeriod` legal compliance? Business calendar reference opt-in or required?
2. **Registry composition** (#21). Two L1 governance docs attaching rules to the same tag — declaration order, explicit priority, or conflict rejection?
3. **Multi-instance design** (#32). Events, arrays, or both? Governance hooks per-instance vs. per-iteration (audit-volume implications).
4. **Version migration declaration surface** (#3). Kernel carries governance version, or each case? `tenant`-scope behavioral contract?
5. **Canonical forms.** Enforce "simple sequential workflow MUST be expressed as compound state with ordered children, not flat atomic sequence"? Other equivalent-representation collapses?
6. **Defeasibility layer** (#25). `workflow-governance` or distinct companion? Priority encoding? Compose with `sourceAuthority` AND Integration Profile §11.2.
7. **Case-field policy vs. L2 `Right`** (#26b). Compose or supersede? Hold policy interaction. Assurance Invariant 6 independence.
8. **Cancellation region semantics** (#27). Set or predicate? Event / guard / explicit action? Compensation — run / skip / author's choice?
9. **`ExternalArtifactRef` shape** (#28). 9th case-field type or `$def`? Retrieval contract — sync / deferred / action-body?
10. **Task suspension reducibility** (#30). Always reducible to `holdType: task-suspended`, or independent task state needed?
11. **Equity expression language** (#36). FEL extension, restricted DSL, or FEL + windowing?
12. **Assurance-level composition** (#43). Minimum floor per impact level, disclosure-only, or implementation-defined?
13. **JSON-LD export surface** (#9). When should `@context` ship — with #47 or separately via ontology-spec?
14. **#29b firing mechanism.** Event-based (enqueue synthetic event) or guard-based (`$milestone.*` FEL boolean)?

**Resolved during 2026-04-16 audit:**
- Q: `x-lm.critical` enforcement — resolved by promoting to #34.
- Q: DRAFTS triage — elevated to Next Steps item.
- Q: F-2 spec preamble rollout — resolved: retrofit via #45, not prospective-only.
- Q: M-2 prerequisite fixture audit — resolved: M-2 rejected; ship #39.
- Q: NoticeTemplate reconciliation — resolved to action: drop thin Due Process sidecar version; Notification Template is canonical (executed in #2 ADR).
- Q: Milestone sync-point semantics — resolved by code-scout: "once per write-then-react envelope."

---

## Convention

Every new or revised spec MUST include **Normative Contract** (processor MUST/SHOULD/MAY obligations), **Composition** (seam attachment, precedence, conflict resolution), and **Conformance** (fixture rule-ID patterns) sections. Retrofit existing specs via #45. Template in `wos-spec/CONVENTIONS.md`.

---

## Research Corpus

| File | Role |
|------|------|
| `compass_artifact...markdown.md` | 50+ standard survey; 7-layer recommendation (compass Direction 3 validates current architecture) |
| `Toward an open...docx` | Feature taxonomy; DCR / Catala / XES / GSM / OpenFisca discoveries |
| `Agentic AI Integration...docx` | Agent protocols (MCP, A2A); OWASP / NIST / EU AI Act |
| `AI-Native Workflow Standards...docx` | BPMN / DMN / CMMN survey (mostly redundant with compass) |
| `prompts/research-prompt.md` | Prompt for the compass artifact |

**Research-identified improvements tracked:** R1 SCITT transparency → #48. R2 SLSA agent attestations → informs #1 (deferred).

---

## Evidence Summary

Citations for the Ground Truth, capability claims, and validation findings:

| Claim | Source |
|---|---|
| State kinds closed to 4 | `wos-kernel.schema.json $defs/State.properties.type`; `kernel/spec.md §4.3` |
| `Transition.event` free-form string | `wos-kernel.schema.json:348` |
| FEL normatively required for guards | `kernel/spec.md §7.4`; `crates/wos-core/src/eval.rs:13` |
| Eleven kernel-generated events closed | `kernel/spec.md §4.10` |
| Governance attaches via tag-based `lifecycleHook` | `kernel/spec.md §10.4, §4.12`; `companions/runtime.md §8` |
| Four-document separation | Kernel + Governance + Lifecycle Detail + Runtime Companion |
| 197 constraints (37/55/105) | `LINT-MATRIX.md`; `crates/wos-lint/src/rules/` |
| Six extension seams | `kernel/spec.md §10` |
| `ProvenanceKind` monolith (93) | `crates/wos-core/src/provenance.rs` |
| `wos-runtime/src/runtime.rs` 3821 lines | `crates/wos-runtime/src/runtime.rs` |
| `wos-core` exports L2/L3 | `crates/wos-core/src/lib.rs:22-37` |
| Binding → runtime inversion | `crates/wos-formspec-binding/Cargo.toml:12` |
| `impactLevel` in kernel is governance-consumed | `wos-kernel.schema.json:64-74`; `kernel/spec.md:333` |
| 12 unresolved DRAFTS | `DRAFTS/` |
| BPMN topology rejected, event taxonomy adopted | `kernel/spec.md:636`; Appendix A |
| `x-lm.critical` zero violations | 131 nodes across 18 schemas, all pass description+examples (scanned 2026-04-16) |
| Assurance schema has zero `x-lm.critical` | `schemas/assurance/wos-assurance.schema.json` |
| Milestone spec/runtime drift (provenance shipped, reactive not) | `kernel/spec.md §4.13` (observable-only prose) vs. KS.2 fixtures K-M-001..005 + `milestones.rs:27-70` + `runtime.rs:602-609, 903-912` |
| Milestone sync-point (code-determined) | `runtime.rs:602-609, 903-912`; `integration_handlers/*.rs` |
| `continuous_reevaluate` is dead code | `eval_mode.rs:55-115` — never called from runtime |
| `AccessControl.canRead` is a stub | `traits/mod.rs:106-115, 242-258` (default `true`); zero call sites |
| `inputDigest`/`outputDigest` are spec-prose only | `kernel/spec.md:403-411` vs. zero matches in `crates/` |
| #2 scaffolding thinner than framed | `AdverseDecisionPolicy` no required fields; two `NoticeTemplate` definitions; no runtime rendering; G-062 static lint with heuristic id-matching |
| `NoticeSent` is a stub emission | `event_handler.rs:72-81` |
| `tenant`-scope unimplemented | Zero matches for `tenant`/`schemaUpgrade` in `crates/` |
| Facts Tier `inputs` never populated in conformance | 0 of 146 conformance fixtures |
| M-1 blocker: standalone Drift Monitor fixture | `fixtures/ai/benefits-drift-monitor.json` |
| M-2 rejection: non-due-process Notification uses | `fixtures/sidecars/benefits-notification-templates.json` — 4 categories |
| §12.2 / §7.15 / schema disagree on `legal-hold` | `workflow-governance.md §12.2, §7.15`; `wos-workflow-governance.schema.json:991` |
| Override §7.3 three-field normative | `governance/workflow-governance.md §7.3` |
| Temporal parameters implemented | `governance/policy-parameters.md` |
| Separation of duties §7.2 | `governance/workflow-governance.md §7.2` |
| Business calendar sidecar | `sidecars/business-calendar.md` |
| AI confidence — ConfidenceReport | `ai/ai-integration.md §6.3, §7.1` |
| Governance §2.9 `schemaUpgrade` enums | `workflow-governance.md §2.9`; `wos-workflow-governance.schema.json properties/schemaUpgrade` |
| Cancellation regions absent | `cancellationPolicy: cancel-siblings` is join-policy only — `kernel/spec.md §4.4` |
| Defeasibility absent | Searched workflow-governance, advanced-governance, kernel, assertion-library — no match |
| Assertion Library no reference protocol | `assertion-library.md`; `PipelineStage.assertions[]` inline-only |
| `ContinuationPolicy` no parent trigger | `wos-due-process.schema.json` vs. `AppealMechanism.continuationOfServices` |
| Task §10 has no schema properties | `workflow-governance.md §10` ("not declared in the Workflow Governance Document") |
| Equity Config fully specified | `equity-config.md`; `wos-equity.schema.json` |
| Drift Monitor ↔ Agent Config binding missing | `drift-monitor.md §1.5` `demoteToAssistive` vs. `agent-config.md §1.4` `DemotionRule` |
| Assurance Invariant 6 | `assurance.md §4.3` |
| Integration Profile §11.2 "restrict, never relax" | `profiles/integration.md §11.2` |

---

## Next Steps

1. Triage `DRAFTS/` (12 files) — classify archive / delete / extract. **Before any schema/spec PR lands.**
2. Ship **#34 `x-lm.critical` gate** and **#57 Assurance coverage** — zero dependencies, top Urgency.
3. Resolve **#29a milestone spec-lag** — spec prose + `triggerMode` schema property.
4. Execute **unified provenance-record-shape ADR sequence:** #23 → #24a → #2 (including NoticeTemplate reconciliation).
5. ADR for **#20 Typed events** — resolve timer calendar-semantics explicitly; strict five kinds.
6. ADR for **#47 PROV-O export** — then #48 + #52 follow.
7. Ship low-cost batch in one sprint: **#12, #13, #42, #56, #46, #26a, #36 → #35, #37, #39**.
8. **Joint design pass:** #25 + #24b (with §6.2 + §11.2 composition).
9. Execute merges: Assertion Library → Workflow Governance, Verification Report → Advanced Governance, Due Process Config partial merge → Workflow Governance (pending #45 step 0).
10. Refresh TODO.md audit date and add cross-references.
