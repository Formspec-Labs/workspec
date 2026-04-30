# Studio Spec: Readiness & Validation

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.14 ValidationFinding.
**PRD anchor:** [`../VISION.md`](../VISION.md) §9.6 (Validation Center), §16 Phase-2 Epic 2.1.
**Depends on:** [`source-vault.md`](source-vault.md), [`policy-object-model.md`](policy-object-model.md), [`authoring-provenance.md`](authoring-provenance.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md).

## Scope

Readiness & Validation is Studio's **lint-engine analog**: a registry of rules tagged by tier (S1–S6) that surface ValidationFindings against workspace state. The Validation Center (PRD §9.6) is the user-facing surface; this spec defines the rule registry, the firing semantics, the finding lifecycle, and the publication-blocker boundary.

This spec defines:

- the six readiness tiers (S1–S6) and the kind of findings each tier surfaces;
- the ValidationFinding record shape and lifecycle;
- the **rule registry**: how rules are identified, tiered, and resolved against workspace state;
- the **severity ladder** (`info` → `warn` → `error` → `block`) and what each severity does;
- the **waiver model**: how reviewers acknowledge or waive findings without erasing them;
- the **publication-blocker contract**: which findings prevent advance to which lifecycle states;
- composition with the upstream specs that *produce* facts and the downstream specs that *consume* findings;
- conformance.

## Out of scope

- The Validation Center UX (PRD §9.6 capability; not normative).
- The internal lint-rule implementation language (Stage 4).
- Runtime validation of executed cases (lives in `wos-conformance` and adapter crates).

## Terminology

- **Tier** — one of S1, S2, S3, S4, S5, S6. Tiers are **layered**: lower tiers must be clean (or waived) before higher tiers can be evaluated reliably.
- **Rule** — a uniquely-identified readiness check (e.g., `SV-LINT-001`). Rule IDs are stable; rule prose may evolve.
- **Finding** — an instance of a rule's predicate evaluating to "fail" against a specific subject in workspace state.
- **Severity** — `info` | `warn` | `error` | `block`. Determines downstream behavior.
- **Subject** — the workspace object the finding is *about* (a SourceDocument, PolicyObject, mapping, workflow element, scenario, etc.).
- **Waiver** — a ReviewerResolution recorded against a finding that explicitly acknowledges and (optionally) lowers its severity.
- **Blocker** — a finding whose severity is `block`, OR a finding whose tier is gating a lifecycle advance per the publication-blocker contract.

## Data model

### `ValidationFinding` (CM §1.14, extended)

```text
ValidationFinding {
  id, tier (S1..S6), ruleId, severity (info|warn|error|block),
  subjectKind, subjectRef,
  message, suggestedFix?,
  detectedAt, detectedBy (rule engine version),
  lifecycleState (open|acknowledged|resolved|waived),
  waivedBy?, waivedRationale?, waivedAt?,
  resolvedBy?, resolvedAt?, resolvedAction?,
  workspaceId
}
```

### `ReadinessRule` (registry)

```text
ReadinessRule {
  id (stable, e.g. "SV-LINT-001"),
  tier (S1..S6),
  defaultSeverity,
  promptId (cross-reference to a Studio MUST/SHOULD, e.g. "SA-MUST-source-020"),
  predicateSummary,
  remediationGuidance,
  activeIn (workspace-types where this rule applies; default = all)
}
```

The rule registry is **append-only by ID**: rule IDs are stable across schema versions. Rule prose, severity defaults, and predicates may evolve; ID retirement is handled by version bump + deprecation note (see Composition).

## The six tiers

Tiers are **layered**: a clean (or waived) lower tier is a precondition for a reliable higher-tier evaluation. The tier-to-lifecycle gating is in the Normative Contract.

### Tier S1 — Source and extraction readiness

Surfaces findings about **the source layer and AI extraction**: missing citation anchors, dangling citations, premature finalization of preliminary sources, low-confidence extractions without reviewer attention.

Representative rules (sourced from [`source-vault.md`](source-vault.md) Conformance and [`policy-object-model.md`](policy-object-model.md) Conformance):

- `SV-LINT-001` — every SourceCitation resolves to a real SourceSection (`SA-MUST-source-020`).
- `SV-LINT-002` — citation excerpts match section text (`SA-MUST-source-021`).
- `SV-LINT-003` — no PolicyObject relies solely on `disputed` or `superseded` versions (`SA-MUST-source-033`).
- `SV-LINT-004` — `current` SourceVersions have `effectiveStart` (`SA-MUST-source-004`).
- `SV-LINT-005` — section anchors unique within a SourceVersion (`SA-MUST-source-010`).
- `SV-LINT-006` — ExtractedClaims with `confidence < 0.5` are not auto-approved (`SA-MUST-pom-010`).

Default severity: `error`. Findings at S1 block advance to S2-and-beyond evaluations on the affected subject.

### Tier S2 — Policy object readiness

Surfaces findings about **structured policy objects**: missing fields, unresolved conflicts/assumptions, missing AuthorityRank for conflicting sources, missing effective dates, citation-or-assumption gaps, unsupported elements.

Representative rules:

- `POM-LINT-001` — every approved PolicyObject has at least one citation or basis-assumption (`SA-MUST-pom-004`).
- `POM-LINT-007` — no circular Supersession (`SA-MUST-pom-039`).
- `POM-LINT-008` — every Conflict is resolved or waived before downstream advance (`SA-MUST-pom-041`).
- `PROV-LINT-002` — every approved object's chain resolves to a citation, assumption, or attestation (`SA-MUST-prov-020`).
- `PROV-LINT-003` — `originClass = approved-interpretation` carries a ReviewerResolution (`SA-MUST-prov-012`).
- `PROV-LINT-004` — `originClass = local-practice` carries an attestation (`SA-MUST-prov-013`).

Default severity: `error`.

### Tier S3 — Mapping readiness

Surfaces findings about **the Studio→WOS bridge**: approved objects without mappings, workflow steps without policy backing, mappings to invalid WOS paths, target collisions, `requiresSpecExtension` blocking advance.

Representative rules (sourced from [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) Conformance):

- `MAP-LINT-001` — every approved PolicyObject has a mapping (`SA-MUST-map-001`).
- `MAP-LINT-002` — `mapsToWos` mappings carry valid `wosConceptId` and `wosJsonPath` (`SA-MUST-map-010` + `011`).
- `MAP-LINT-003` — `requiresSpecExtension` mappings carry a substantive ExtensionRecord (`SA-MUST-map-020` + `022`).
- `MAP-LINT-004` — `unmappedButApproved` mappings carry substantive rationale; finding stays at `warn` perpetually as a "noisy unmapped" reminder (`SA-MUST-map-041`).
- `MAP-LINT-005` — no two PolicyObjects collide on the same target.
- `MAP-LINT-006` — workflow-bearing PolicyObjects are not `unmappedButApproved` without override (`SA-MUST-map-004`).
- `MAP-LINT-007` — workflow-bearing PolicyObjects do not have an `open` ExtensionRecord blocking advance (`SA-MUST-map-005`).
- `MAP-LINT-008` — `x-` targets carry an extension-registry entry (`SA-MUST-map-014`).

Default severity: `error` for collision and missing mapping, `warn` for `unmappedButApproved`-noisy.

### Tier S4 — Workflow readiness

Surfaces findings about **the WorkflowIntent itself**: adverse outcomes without notices, appeal rights without branches, deadlines without timers, decision-rule inputs not collected, actor authority gaps, agent declarations missing.

Representative rules (cross-cutting from [`policy-object-model.md`](policy-object-model.md) and the WorkflowIntent compilation):

- `WF-LINT-001` — every adverse `Outcome` (`triggersDueProcess = true`) links a NoticeRequirement and an AppealRight (`SA-MUST-pom-030`).
- `WF-LINT-002` — every AppealRight has an appeal branch in the WorkflowIntent.
- `WF-LINT-003` — every Deadline has a TimerMapping or an explicit review obligation.
- `WF-LINT-004` — DecisionRule inputs are collected before the rule fires (`SA-MUST-pom-031`).
- `WF-LINT-005` — every actor has documented authority for every step it owns; agent ActorMappings link an AI-Use object (`SA-MUST-pom-036`).
- `WF-LINT-006` — sensitive DataElements have retention policy on every collecting EvidenceRequirement (`SA-MUST-pom-037`).
- `WF-LINT-007` — every required EvidenceRequirement has a workflow collection step.
- `WF-LINT-008` — every workflow step has a derivedFrom citation chain that resolves (per [`authoring-provenance.md`](authoring-provenance.md)).

Default severity: `error`. Tier-S4 findings on a WorkflowIntent block advance from `mapped → validationReady`.

### Tier S5 — Scenario readiness

Surfaces findings about **scenario coverage and outcomes**: critical paths without scenarios, scenarios lacking expected outcomes, failing scenarios without waivers, post-change scenarios not rerun.

Representative rules (sourced from [`scenario-authoring.md`](scenario-authoring.md), forthcoming):

- `SC-LINT-001` — every adverse Outcome has at least one Scenario exercising it.
- `SC-LINT-002` — every AppealRight has at least one Scenario exercising the appeal branch.
- `SC-LINT-003` — every Scenario carries `expectedOutcome` fields.
- `SC-LINT-004` — failing Scenarios are either `acceptedAsKnownGap` (with rationale) or block advance.
- `SC-LINT-005` — after a SourceVersion supersession that affects a Scenario's linked PolicyObjects, the Scenario MUST re-run before the workflow advances.

Default severity: `error` for missing critical-path coverage, `warn` for soft-coverage gaps.

### Tier S6 — Publication readiness

Surfaces findings about **the moment of publication**: critical findings unresolved, required reviewers missing, WOS schema/lint failures, approval package incomplete.

Representative rules:

- `PUB-LINT-001` — no `error` or `block` findings remain unresolved at publication.
- `PUB-LINT-002` — every required reviewer role has at least one ApprovalDecision (per [`review-and-approval.md`](review-and-approval.md), forthcoming).
- `PUB-LINT-003` — the compiled `$wosWorkflow` artifact passes `wos-workflow.schema.json` validation.
- `PUB-LINT-004` — the compiled artifact passes WOS lint (`crates/wos-lint`).
- `PUB-LINT-005` — the approval package contains: `$wosWorkflow`, scenario suite, validation report, citation manifest, release notes, approval certificate.
- `PUB-LINT-006` — every `unmappedButApproved` mapping is listed in the release notes (`SA-MUST-map-042`).

Default severity: `block` for incomplete artifact / failing schema / missing reviewer; `error` for unresolved error-tier findings; `warn` for soft gaps.

## Finding lifecycle

```text
open → { acknowledged | resolved | waived }
acknowledged → { resolved | waived }
```

- `open`: initial state; the finding is fresh.
- `acknowledged`: a reviewer has seen the finding and accepted it as a known item that does not yet have a fix or waiver. Acknowledged findings still count toward severity gates (an acknowledged `error` still blocks advance).
- `resolved`: the underlying condition that caused the rule to fire has been corrected; the next rule re-evaluation MUST confirm resolution and clear the finding.
- `waived`: a reviewer has explicitly waived the finding. Waived findings do not block advance, but they remain visible and project into the published artifact's release notes.

A finding that re-fires after `resolved` (because the condition recurred) MUST be recorded as a *new* finding instance — the resolved one stays in history. This preserves the audit trail.

## Normative Contract

### Rule registry integrity

- **`SA-MUST-rv-001`** — Every readiness rule MUST have a stable, unique `ruleId`. Rule IDs MUST NOT be reused after retirement. *(schema-pending: registry uniqueness.)*
- **`SA-MUST-rv-002`** — Every rule MUST belong to exactly one tier (S1..S6). *(schema-pending: tier enum.)*
- **`SA-MUST-rv-003`** — Every rule SHOULD reference at least one Studio MUST/SHOULD prompt ID (`promptId`); rules without a prompt reference are valid but harder to audit. *(soft.)*
- **`SA-MUST-rv-004`** — Adding a new rule, retiring an existing rule, or changing a rule's `defaultSeverity` MUST be recorded as a registry version bump and announced in the workspace; existing findings are not retroactively re-evaluated unless the workspace explicitly opts in to re-validation. *(runtime-pending.)*

### Rule firing

- **`SA-MUST-rv-010`** — Rules MUST be evaluated automatically against the workspace state on every state-change event (object created, edited, lifecycle-transitioned, citation-superseded, mapping-changed, etc.). *(runtime-pending: change-detection and rule-replay.)*
- **`SA-MUST-rv-011`** — Rule evaluation MUST be **deterministic**: given the same workspace state and the same registry version, the set of findings MUST be identical. *(fixture-pending.)*
- **`SA-MUST-rv-012`** — Rule evaluation MUST emit findings in **one direction only** — the engine produces findings; it never silently mutates the subject of a rule. (A reviewer may edit the subject in response to a finding; that is a separate event.) *(runtime-pending; PRD §5 Principle 9: AI proposes; humans approve.)*
- **`SA-MUST-rv-013`** — A rule that depends on multiple subjects (e.g., a tier-S4 rule that compares an Outcome to its referencing NoticeRequirement and AppealRight) MUST list all dependent subjects in the resulting finding's metadata so reviewers see the full surface. *(schema-pending.)*

### Severity ladder

- **`SA-MUST-rv-020`** — Severity ordering: `info` < `warn` < `error` < `block`. A workspace MAY raise but MUST NOT lower the default severity of a rule for a specific finding without a recorded waiver. *(runtime-pending.)*
- **`SA-MUST-rv-021`** — `block` findings MUST prevent the relevant lifecycle advance and MUST NOT be auto-clearable by the engine — only by reviewer resolution or explicit waiver. *(runtime-pending: gating.)*
- **`SA-MUST-rv-022`** — `error` findings MUST prevent advance from the rule's tier to the next tier (e.g., open S2 errors block advance from `mapped` to `validationReady`). *(runtime-pending.)*
- **`SA-MUST-rv-023`** — `warn` findings MUST be visible at publication time; they are listed in the release notes but do not block advance. *(runtime-pending; cross-cutting with `SA-MUST-map-042`.)*
- **`SA-SHOULD-rv-024`** — `info` findings SHOULD be queryable but MAY be suppressed by default in the Validation Center UX.

### Waivers

- **`SA-MUST-rv-030`** — A waiver MUST carry a substantive `waivedRationale` (≥ 50 characters), `waivedBy` (reviewer id), `waivedAt` (server timestamp), and `waivedScope` — either `this-instance-only` or `this-rule-on-this-subject-until-condition`. Time-bounded waivers MUST automatically expire and re-fire the underlying finding. *(schema-pending.)*
- **`SA-MUST-rv-031`** — Waiver of a `block`-severity finding MUST require a reviewer with override authority (workspace policy decides; default: workspace owner). *(runtime-pending: role policy.)*
- **`SA-MUST-rv-032`** — Waivers MUST NOT delete findings; the finding's `lifecycleState = waived` and the original message remains visible. *(schema-pending: lifecycle enum; runtime-pending.)*
- **`SA-MUST-rv-033`** — Every waiver MUST emit an AuthoringProvenanceRecord with `eventKind = findingWaived` (per [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-001`). *(runtime-pending.)*

### Publication-blocker contract

- **`SA-MUST-rv-040`** — A WorkflowIntent MUST NOT advance from `validationReady → scenarioTested` while any tier-S1, S2, S3, or S4 finding on its referenced subjects has severity `error` or `block` and lifecycle `open` or `acknowledged`. *(lint-pending: tier-S5 readiness rule.)*
- **`SA-MUST-rv-041`** — A WorkflowIntent MUST NOT advance from `scenarioTested → approved` while any tier-S5 finding has severity `error` or `block` and lifecycle `open` or `acknowledged`. *(lint-pending: tier-S6 rule.)*
- **`SA-MUST-rv-042`** — A WorkflowIntent MUST NOT advance from `approved → published` while any tier-S6 finding has severity `error` or `block` and lifecycle `open` or `acknowledged`. *(lint-pending: tier-S6 rule.)*
- **`SA-MUST-rv-043`** — Waivers (`waived` state) bypass the gates above; they do not reset the finding to `open`. The waivedRationale projects into the release notes. *(runtime-pending.)*

## Composition

### Attachment point

The readiness engine attaches at the **workspace** layer. Findings are workspace-scoped; rule definitions are global (drawn from the registry shipped with each Studio version).

The engine **reads** state from every workspace-state spec ([`source-vault.md`](source-vault.md), [`policy-object-model.md`](policy-object-model.md), [`authoring-provenance.md`](authoring-provenance.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md), [`scenario-authoring.md`](scenario-authoring.md), [`review-and-approval.md`](review-and-approval.md), [`change-impact.md`](change-impact.md)). It **writes** only ValidationFinding records.

Tier S6 has a special boundary: it consumes WOS-side validation outputs (schema-validation result from `wos-workflow.schema.json`, lint result from `wos-lint`, conformance result from `wos-conformance`) and lifts them into Studio-side findings. The contract for this lift is in WOS Mappings below.

### Precedence

When two rules fire on the same subject, both findings stand — they do not merge or eclipse each other. A reviewer may resolve them with a single edit (one fix often clears multiple findings); the engine's next pass clears whichever findings the fix addressed.

When a workspace's rule registry is upgraded mid-flight (a Studio version bump), existing findings remain in their current lifecycle states. The new registry's rules begin firing; old rules whose definitions changed become "legacy"; reviewers may choose to re-evaluate.

### Conflict handling

Two rules with contradictory predicates on the same subject (e.g., one rule says "every NoticeRequirement must specify two languages" and another says "every NoticeRequirement must specify one language") would represent a registry-level bug. The spec assumes such contradictions are caught at registry-definition time, not at firing time. Should one slip through, **both findings stand** until a reviewer resolves at the registry level.

### Versioning / migration

- Adding a new rule to the registry: **non-breaking** for the rule itself; the rule begins firing on the next state-change event. Workspaces with pre-existing state may receive a wave of new findings on first re-evaluation.
- Changing a rule's `defaultSeverity`: **soft-breaking**. Existing findings retain their original severity (frozen at finding-creation time); new findings use the new default.
- Retiring a rule: existing findings transition to `acknowledged-as-legacy`; they no longer block but remain visible.
- Changing the tier of a rule: **breaking**. The publication-blocker contract is tier-keyed; tier changes require a registry version bump and reviewer attention on every workspace.

## Conformance

### Schema validation (Stage 3)

Planned schema gates:

- ValidationFinding required fields and tier/severity enums.
- ReadinessRule registry shape.
- Waiver schema (`SA-MUST-rv-030`).

### Lint rules (Stage 4)

The readiness engine **is** the lint surface; it does not have lint rules of its own beyond meta-checks:

- `RV-META-001` — every rule in the registry has a `promptId` (recommended, not required).
- `RV-META-002` — every rule's `tier` is one of S1..S6.
- `RV-META-003` — `defaultSeverity` is one of `info` | `warn` | `error` | `block`.

These meta-checks run at registry build time, not against workspace state.

### Runtime conformance fixtures (Stage 4–5)

- Deterministic firing: identical workspace state + identical registry → identical findings (`SA-MUST-rv-011`).
- Severity gating: an open `error` at tier S2 blocks advance from `approved` to `validated`.
- Waiver semantics: waiving a `block` finding requires override authority and emits a provenance event.
- Tier S6 lift: a WOS schema validation failure surfaces as a Studio `PUB-LINT-003` finding.

### Current limitations

- The full rule registry is **provisional** — every rule listed in this spec is a pointer into the rule's source spec, not a fully-specified predicate. Stage 4 (the readiness engine) is where each rule receives its precise predicate definition.
- Cross-workspace registry distribution (e.g., a tenant adopting a new Studio version) is unspecified.

## WOS mappings

ValidationFindings and the rule registry are **`authoringOnly`** as a whole. The readiness engine is a Studio-internal concern; it is not a WOS construct.

The exception: Tier S6 findings that result from **lifting WOS-side validation outputs** are bound to specific WOS surfaces:

| Studio rule | Source of truth | WOS path |
|---|---|---|
| `PUB-LINT-003` | `wos-workflow.schema.json` validation | the schema itself; result is a pass/fail with errors |
| `PUB-LINT-004` | [`crates/wos-lint`](../../crates/wos-lint) | the 197 WOS lint constraints |
| `PUB-LINT-005` | [`crates/wos-conformance`](../../crates/wos-conformance) | conformance fixtures (when the workspace runs WOS conformance against its draft) |

These surfaces are **read** by the Studio engine; they are not Studio-defined. The lift produces a Studio ValidationFinding whose `message` carries the WOS-side diagnostic verbatim plus a Studio-friendly `suggestedFix` where possible.

Findings themselves do not project into the published `$wosWorkflow` artifact (they are workspace-state). However, **waivers** project into the published artifact's release notes per `SA-MUST-rv-023` and `SA-MUST-map-042`. Waiver projection is part of the authoring-provenance compact emission per [`authoring-provenance.md`](authoring-provenance.md) `SA-MUST-prov-030`.

## Examples

### Example 1: Tier-cascade — a citation supersession ripples through

A new SourceVersion of an SOP supersedes the prior version. The new section text changes by one phrase that affects an approved Requirement PolicyObject.

1. **S1 fires:** `SV-LINT-002` (citation excerpt mismatch) on the Requirement's citation. Severity `error`.
2. **S2 may fire:** if the Requirement is the basis for a Conflict that was previously resolved, the supersession may surface a re-evaluation `POM-LINT-008` finding.
3. **Until S1 is cleared**, S2/S3/S4/S5 evaluations on the Requirement are skipped (`SA-MUST-rv-022`).
4. The reviewer resolves S1 by either updating the citation excerpt to match the new section text or marking the change immaterial via ReviewerResolution. The S1 finding clears.
5. Tier S2–S4 re-evaluate. If clean, the workflow can advance again.

### Example 2: Waiver of a noisy unmapped mapping

A workspace's `Reasonable Accommodation Tracking Field` is a state-specific local-practice concept; no WOS counterpart and no upstream extension is yet in scope. Reviewer authors:

1. PolicyObject `kind = DataElement`, `originClass = local-practice` (with attestation).
2. StudioToWosMapping `state = unmappedButApproved`, with rationale.
3. Tier-S3 `MAP-LINT-004` fires at severity `warn`. The finding's `lifecycleState = open`.
4. The workflow owner waives the finding with rationale "Local-practice field; tracked outside WOS via state agency reporting system. Re-evaluate at Q4 2026 review." `waivedScope = this-rule-on-this-subject-until-condition` with expiration 2026-12-31.
5. The workflow advances; the waiver projects into the release notes.
6. On 2027-01-01, the waiver auto-expires; the finding re-fires. The Q4 reviewer re-decides.

### Example 3: Tier-S6 publication blocker — failing WOS schema

The compiler emits a draft `$wosWorkflow` artifact. The artifact fails `wos-workflow.schema.json` validation: a `governance.notices[0].timing.duration` field is missing.

1. `PUB-LINT-003` fires with severity `block`, message: "wos-workflow.schema.json validation failed: governance.notices[0]: missing required property 'timing.duration'".
2. The workflow cannot advance to `published`.
3. The reviewer traces back: the upstream PolicyObject NoticeRequirement is missing a `timing` field that the schema requires.
4. The reviewer edits the NoticeRequirement, re-approves, the compiler re-emits, the schema validates, the finding clears, the workflow publishes.

This is the *correct* behavior — Studio is letting WOS's schema, not Studio's prose, be the final structural arbiter.

## Open issues

- **Rule predicate language.** Stage 4's predicate language for the rule registry (Rust-native? embedded DSL? FEL?) is unsettled. The spec describes rule *outcomes* but not *how rules are written*.
- **Re-evaluation triggers.** The list of "state-change events" that re-fire rules is sketched (`SA-MUST-rv-010`) but not enumerated. Stage 4 will pin this to a concrete event taxonomy.
- **Concurrent waivers.** Two reviewers waiving the same finding simultaneously is a race. Resolution policy (last-write-wins; first-write-wins; merge) is unspecified.
- **Acknowledged-but-still-blocking semantics.** The current spec has `acknowledged` blocking advance the same as `open`; some workflows may want `acknowledged` to act as a soft acknowledgment that does not block. The choice is unsettled and may become a workspace policy.
- **WOS conformance fixture lift.** PUB-LINT-005 lifts conformance results, but the conformance run is expensive and not always run by the workspace. Whether tier S6 *requires* a conformance run before publication is unsettled.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.14, §3 (mapping states feed S3 rules).
- PRD: [`../VISION.md`](../VISION.md) §9.6, §16 Phase-2 Epic 2.1, §12 user stories.
- Upstream: [`source-vault.md`](source-vault.md), [`policy-object-model.md`](policy-object-model.md), [`authoring-provenance.md`](authoring-provenance.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md).
- Downstream: [`scenario-authoring.md`](scenario-authoring.md), [`review-and-approval.md`](review-and-approval.md), [`change-impact.md`](change-impact.md).
- WOS validation surfaces: [`../../schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json), [`../../crates/wos-lint`](../../crates/wos-lint), [`../../crates/wos-conformance`](../../crates/wos-conformance).
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
