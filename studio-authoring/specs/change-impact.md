# Studio Spec: Change Impact

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.18 ChangeImpactReport.
**PRD anchor:** [`../VISION.md`](../VISION.md) §9.8 (Versioning and Change Impact), §16 Phase-2 Epic 2.5 (Semantic Version Compare), §16 Phase-3 Epic 3.2 (Change Impact Analysis).
**Depends on:** [`source-vault.md`](source-vault.md), [`policy-object-model.md`](policy-object-model.md), [`authoring-provenance.md`](authoring-provenance.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md), [`scenario-authoring.md`](scenario-authoring.md), [`review-and-approval.md`](review-and-approval.md).

## Scope

Change Impact is the spec that **makes workflows living governed assets**. It defines how a change at any point in the workspace (a new SourceVersion, a PolicyObject edit, a mapping update, a runtime observation cluster) propagates through the dependency graph to identify everything that may need re-review, re-validation, re-simulation, or re-publication.

This is a flagship feature, not an afterthought (PRD §9.8). Without it, Studio cannot answer the question that defines Phase 3: "This policy changed. Which workflows and scenarios are affected?"

This spec defines:

- the ChangeImpactReport record and the propagation chain;
- the four trigger kinds (source-version-change, policy-object-edit, mapping-update, runtime-observation-cluster);
- the **semantic diff** model used to compare workflow versions and produce release notes;
- the **scenario regression contract**;
- composition with every upstream spec (this spec is a *consumer* of every other spec's state and a *driver* of re-review);
- conformance.

## Out of scope

- The change-impact UI / dashboard (PRD §9.8 capability).
- Automated workflow editing in response to changes (Studio MUST NOT auto-edit; PRD §10 AI behavior — copilot proposes, humans approve).
- Runtime case state migration when a published workflow is superseded (a downstream WOS-runtime concern; the spec only identifies *potentially* affected active cases per PRD §9.8 chain).

## Terminology

- **Trigger** — the originating change event (source supersession, policy object edit, mapping update, runtime observation cluster).
- **Propagation chain** — the ordered set of affected artifacts walking the dependency graph from the trigger.
- **Affected artifact** — anything in the chain: PolicyObject, Mapping, WorkflowIntent element, Scenario, Assumption, ReviewerResolution, ApprovalDecision, PublishedWorkflowPackage, RuntimeObservation.
- **Semantic diff** — a workflow-element-aware diff that reports changes in plain language ("the appeal deadline changed from 60 to 90 days"; "the Spanish translation requirement was added"), not byte-level.
- **Regression** — a Scenario that was `passing` and is now `failing` after the change.
- **Materiality** — a reviewer-judgment classification: `material` (re-review required) | `immaterial` (acknowledgment sufficient) | `cosmetic` (no review action).

## Data model

### `ChangeImpactReport` (CM §1.18, extended)

(Note: `triggerKind` enum is extended to seven values — see §"Triggers" below.)

```text
ChangeImpactReport {
  id, triggerKind, triggerRef,
  affectedPolicyObjects[],
  affectedMappings[],
  affectedWorkflowIntents[],
  affectedWorkflowElements[],
  affectedScenarios[],
  affectedPublishedPackages[],
  affectedAssumptions[],
  affectedReviewerResolutions[],
  semanticDiff (when triggerKind = workflow-version-compare),
  summary, narrativeSummary,
  producedAt, producedBy (engine version),
  acknowledgedBy?, acknowledgedAt?,
  closedAt?, closureRationale?,
  workspaceId
}
```

Each `affected*` list carries `{subjectRef, materialityClassification, reviewActionRequired}`.

### `SemanticDiff`

```text
SemanticDiff {
  fromWorkflowVersion, toWorkflowVersion,
  changedSteps[],            // {stepRef, kind: added|removed|modified, description}
  changedDecisions[],
  changedActors[],
  changedDeadlines[],        // critical: deadline changes are user-facing
  changedNotices[],          // critical: notice changes affect rights
  changedAppeals[],          // critical
  changedDataFields[],
  changedMappings[],
  changedAssumptions[],
  changedReviewerResolutions[],
  releaseNotesDraft (rendered narrative)
}
```

Each `changed*` entry carries source-citation lineage so reviewers can see "this changed because §273.10(b) was updated in the new SourceVersion."

### Materiality classification (normative)

Three classes:

- **`material`** — the change affects rights, deadlines, notices, appeals, decisions, or data collection. Re-review of every affected reviewer's prior approval IS required before downstream advance.
- **`immaterial`** — the change is structural but does not affect outcomes (e.g., editorial reordering of policy citations, renaming an internal step that doesn't change actor work). Reviewer acknowledgment is sufficient; full re-approval is NOT required.
- **`cosmetic`** — the change is text-only and does not affect any reviewer-relevant fact (e.g., correcting a typo in a description). No reviewer action required.

Materiality is **reviewer-classified**, not auto-classified. The engine produces a *suggested* materiality; reviewers MAY override.

## The propagation chain (normative)

The change propagation chain from PRD §9.8:

```text
changed source passages
→ affected policy objects
→ affected WOS concepts
→ affected workflow steps
→ affected scenarios
→ affected published versions
→ potentially affected active cases (downstream; out of scope for this spec)
```

The walk MUST be **complete** (no affected artifact silently elided) and **deterministic** (identical workspace state + identical trigger ⇒ identical report).

The walk crosses boundaries:

- Source vault → Policy object: via SourceCitation.
- Policy object → Mapping: via subjectRef.
- Policy object → WorkflowIntent element: via WorkflowStepMapping / TimerMapping / etc. derivedFrom links.
- WorkflowIntent → Scenario: via `linkedPolicyObjects[]`.
- WorkflowIntent → PublishedWorkflowPackage: via the workflow version reference.
- Assumption → PolicyObject: via Assumption affects[] field.
- ReviewerResolution → Conflict / Assumption / OpenQuestion: via subjectRef.

The walk stops at:

- The boundary of the published artifact (the chain identifies *which* published packages are potentially affected; updating those is a separate publication cycle).
- Active runtime cases (the chain identifies a published package as "potentially affecting active cases" but does not enumerate cases — that is the runtime's concern).

## Triggers

Seven kinds of changes trigger a ChangeImpactReport:

### `source-version-change`

A new SourceVersion supersedes a prior version (per [`source-vault.md`](source-vault.md) `SA-MUST-source-030`). The trigger is the supersession event. The report walks from every PolicyObject citing the prior version.

### `policy-object-edit`

An approved PolicyObject's body is edited (typically following demotion to `draft` and re-approval). The trigger is the edit event. The report walks from the edited object.

### `mapping-update`

A StudioToWosMapping changes state or targets (e.g., `requiresSpecExtension → mapsToWos` after the upstream extension landed). The report walks from the mapping's subject PolicyObject.

### `runtime-observation-cluster`

Multiple RuntimeObservations cluster around a particular workflow element, suggesting that designed behavior diverges from observed behavior at scale (e.g., 30 cases stuck at the same step). The trigger is the cluster detection event per [`runtime-observation-seam.md`](runtime-observation-seam.md). The report walks from the workflow element to the upstream PolicyObjects that could be revised. **The seam contract is now active** (per `runtime-observation-seam.md` `SA-MUST-rtos-010`); the cluster-detection implementation is Phase-4.

### `jurisdictional-supersession`

A cross-document supersession occurs (per `source-vault.md` `SA-MUST-source-006/007`) OR an Effectiveness object's `appellateState` shifts (per [`effectiveness-and-applicability.md`](effectiveness-and-applicability.md) `SA-MUST-eff-011/012`). E.g.: a court enjoins a regulation in one circuit; an errata memo retroactively supersedes paragraphs of a prior version. The report walks from every PolicyObject whose Effectiveness intersects the changed scope.

### `wos-version-deprecation`

A parent stream version reaches deprecation per parent [`COMPATIBILITY-MATRIX.md`](../../COMPATIBILITY-MATRIX.md). E.g., `wos-ai@0.5` is superseded by `wos-ai@1.0`. The report walks from every WorkflowIntent whose `wosVersionPin` (per `compiler-contract.md` `SA-MUST-cmp-052`) targets the deprecating stream. Reviewer-driven migration produces a new `wosVersionPin` per CM §1.33 MigrationPath.

### `compliance-attestation-expiry`

A WorkspaceComplianceBaseline regime attestation reaches its `expiresAt` (per [`workspace.md`](workspace.md) `SA-MUST-ws-061`). The report walks from every PublishedWorkflowPackage whose ApprovalPackage carries an attestation referencing the expiring baseline.

### Workflow-version-compare (request-driven)

Not a propagation trigger — a *user-driven* comparison between two existing workflow versions. Produces a SemanticDiff but does not necessarily produce a propagation chain (the workflows already exist; this is an inspection, not a change-detection). The two are intentionally distinct: propagation is about **discovering** the surface of a change; semantic diff is about **summarizing** a known change for stakeholders.

### Workflow-version-compare (request-driven)

Not a propagation trigger — a *user-driven* comparison between two existing workflow versions. Produces a SemanticDiff but does not necessarily produce a propagation chain (the workflows already exist; this is an inspection, not a change-detection). The two are intentionally distinct: propagation is about **discovering** the surface of a change; semantic diff is about **summarizing** a known change for stakeholders.

## Lifecycle

A ChangeImpactReport's lifecycle:

```text
produced → acknowledged → closed
       \-> superseded (by a later report on the same trigger, rare)
```

- `produced`: the report was generated; reviewers have not yet acknowledged.
- `acknowledged`: a reviewer with appropriate authority has reviewed the report and acknowledged the change surface. Acknowledgment does NOT itself act on the affected artifacts; it just confirms the surface is understood.
- `closed`: every affected artifact has been resolved (re-reviewed and re-approved, demoted with re-approval, waived, or determined to be `cosmetic`/`immaterial`). The report is closed with a `closureRationale`.
- `superseded`: a later report on the same trigger replaces this one (rare; happens if a fresh re-detection yields more affected artifacts than the original, e.g., because the dependency graph itself changed mid-flight).

Allowed transitions:

| From | To | Trigger |
|---|---|---|
| `produced` | `acknowledged` | reviewer acknowledged |
| `acknowledged` | `closed` | every affected artifact resolved |
| `produced` | `closed` | small-impact reports may be closed without separate acknowledge step (workspace policy) |
| any | `superseded` | a later report supersedes |

Reports are durable; they remain queryable indefinitely as part of the workspace audit log.

## Normative Contract

### Change detection

- **`SA-MUST-ci-001`** — Every trigger event MUST automatically produce a ChangeImpactReport. The implementation MUST NOT suppress reports for trigger events. *(runtime-pending: change-detection engine.)*
- **`SA-MUST-ci-002`** — Report generation MUST be deterministic given the same workspace state and the same trigger event. *(fixture-pending.)*
- **`SA-MUST-ci-003`** — A report MUST enumerate all affected artifacts (PolicyObjects, Mappings, WorkflowIntents, Scenarios, PublishedPackages, Assumptions, ReviewerResolutions). Silent elision (e.g., to keep the list short) MUST NOT happen. *(runtime-pending.)*
- **`SA-MUST-ci-004`** — When a report cannot enumerate the chain completely (e.g., a cross-workspace reference is unreachable), the report MUST mark the gap explicitly and MUST NOT be `closed` until the gap is resolved or waived. *(runtime-pending.)*

### Propagation rules

- **`SA-MUST-ci-010`** — A `source-version-change` trigger MUST cascade to demote affected PolicyObjects to `draft` if the supersession is **material** (per [`source-vault.md`](source-vault.md) `SA-MUST-source-021`). The materiality classification is reviewer-driven; default-suggested materiality is `material` if the cited section's text changed and `cosmetic` if only metadata changed. *(runtime-pending.)*
- **`SA-MUST-ci-011`** — Demotion of a PolicyObject MUST cascade to its mappings (per [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) `SA-MUST-map-051`) and to scenarios linking it (per [`scenario-authoring.md`](scenario-authoring.md) `SA-MUST-scn-031`). *(runtime-pending.)*
- **`SA-MUST-ci-012`** — A `policy-object-edit` trigger that changes load-bearing fields MUST cascade through the chain. Editorial-only edits (description text, internal notes) do NOT cascade — they produce reports with `materiality = cosmetic` and require no review action. *(runtime-pending: load-bearing-field detection.)*
- **`SA-MUST-ci-013`** — A `mapping-update` trigger that changes mapping state or targets MUST cascade to the WorkflowIntents referencing the host PolicyObject's mapping. The publication-readiness implications (e.g., a `requiresSpecExtension` finally becoming `mapsToWos`) MUST be reflected in tier-S3 readiness re-evaluation. *(runtime-pending.)*
- **`SA-MUST-ci-014`** — Cascade MUST stop at boundary edges where the impact crosses a published artifact: the report identifies that a PublishedWorkflowPackage is potentially affected, but it does NOT mutate the published artifact. Updating a published artifact requires a new workflow version + new publication cycle. *(runtime-pending.)*

### Scenario regression

- **`SA-MUST-ci-020`** — Every Scenario whose `linkedPolicyObjects[]` contains an affected PolicyObject MUST be re-run after the propagation completes (assuming the scenario engine, Stage 6, is available). *(runtime-pending; cross-cutting with [`scenario-authoring.md`](scenario-authoring.md).)*
- **`SA-MUST-ci-021`** — Scenarios that flip from `passing → failing` after a re-run MUST transition to lifecycle state `regression` (per CM §2.5), NOT plain `failing`. The regression marker is critical — it distinguishes "this scenario was always broken" from "this scenario broke because of this change." *(runtime-pending; cross-cutting.)*
- **`SA-MUST-ci-022`** — Until every regression is resolved, acknowledged-as-known-gap, or waived, the affected WorkflowIntent MUST NOT advance to a new published version. *(lint-pending: tier-S5 rule.)*
- **`SA-SHOULD-ci-023`** — When multiple scenarios regress together, the change-impact engine SHOULD render them as a regression cluster in the report, so reviewers can address common upstream causes once.

### Semantic diff

- **`SA-MUST-ci-030`** — A SemanticDiff between two workflow versions MUST report **at least** these change kinds when they exist: changed steps, decisions, actors, deadlines, notices, appeals, data fields, mappings, assumptions, reviewer resolutions. *(runtime-pending: diff engine.)*
- **`SA-MUST-ci-031`** — Each diff entry MUST cite the upstream causal chain — what SourceCitation, PolicyObject edit, Mapping update, or ReviewerResolution caused the change. Bare structural diffs without causal lineage MUST be flagged as gaps in the diff. *(runtime-pending.)*
- **`SA-MUST-ci-032`** — A SemanticDiff MUST be **deterministic** given identical workflow versions. Re-running the diff produces the same output. *(fixture-pending.)*
- **`SA-MUST-ci-033`** — Release notes MUST be derived from the SemanticDiff and listed in the new PublishedWorkflowPackage's release notes (per [`review-and-approval.md`](review-and-approval.md) ApprovalPackage `releaseNotes` field). *(runtime-pending: notes generation.)*
- **`SA-SHOULD-ci-034`** — Release notes SHOULD be plain-language (PRD §16 Phase-2 Epic 2.5: "generate a summary of changes for stakeholders"). The diff engine renders prose; reviewers MAY edit the rendered notes before publication.

### Cross-spec coupling

- **`SA-MUST-ci-040`** — Every report transition MUST emit AuthoringProvenanceRecords (per [`authoring-provenance.md`](authoring-provenance.md)). *(runtime-pending.)*
- **`SA-MUST-ci-041`** — Reports MUST NOT close while any affected `PublishedWorkflowPackage` has a status that requires user action (a published package with active cases following an outdated workflow definition is a downstream concern, but the report's `closure` does not bypass the need to schedule a new workflow version). *(runtime-pending.)*
- **`SA-MUST-ci-042`** — When a report's surface includes ApprovalDecisions whose `approved-with-conditions` conditions reference the changed subject, those decisions MUST be re-evaluated and (typically) demoted to `superseded`. *(runtime-pending; cross-cutting with [`review-and-approval.md`](review-and-approval.md) `SA-MUST-ra-022`.)*

## Composition

### Attachment point

Change Impact attaches at the **workspace** layer. ChangeImpactReports are workspace-scoped artifacts. Cross-workspace propagation (when a shared SourceVersion update affects multiple workspaces) is a Phase-3 concern; this spec describes the within-workspace propagation only.

The change-impact engine **reads** state from every other Studio spec (it is a consumer of the entire dependency graph) and **writes** ChangeImpactReports plus state-transition triggers (demotion of PolicyObjects, scenario regression flags). It does NOT directly mutate PolicyObject bodies, mapping content, or workflow elements — those edits remain reviewer-driven (PRD §10 AI behavior).

### Precedence

When two triggers fire on overlapping subject sets, both produce reports. The reports stand independently — they do not merge. Reviewers MAY consolidate by closing both with a single set of upstream remediation actions (e.g., re-approving the affected PolicyObjects clears both reports' affected-PolicyObjects lists in one pass).

When a report contradicts a still-active prior report (e.g., a later `source-version-change` reverts the supersession that triggered an earlier report), the earlier report transitions to `superseded` and the later report carries the current surface.

### Conflict handling

ChangeImpactReports do not surface conflicts in the [`policy-object-model.md`](policy-object-model.md) sense. They surface **change surfaces** that may *induce* conflicts (e.g., a SourceVersion supersession may invalidate a prior reviewer-resolved Conflict). Induced conflicts are then handled by the policy-object layer.

### Versioning / migration

- The set of trigger kinds is closed (4 + 1 inspection variant); adding a new trigger kind is **schema-breaking**.
- The propagation algorithm may be refined non-breakingly as long as the produced surface is a superset of prior surfaces (no silent narrowing).
- The materiality classifications (`material` | `immaterial` | `cosmetic`) are normatively fixed; expanding the enum is **schema-breaking**.

## Conformance

### Schema validation (Stage 3)

Planned schema gates:

- ChangeImpactReport required fields and lifecycle enum.
- Trigger kind enum.
- SemanticDiff change-kind enum.
- Materiality classification enum.

### Lint rules (Stage 4)

Cross-cutting rules from other tiers also apply here:

- Tier-S1: source-version supersession (`SV-LINT-002` re-fires under change-impact pressure).
- Tier-S2: PolicyObject demotion ripples (cross-cutting `POM-LINT-008` Conflict re-evaluation).
- Tier-S3: mapping-update propagation (`MAP-LINT-007` for newly-blocked extensions).
- Tier-S5: scenario regression cascade (`SC-LINT-005`).

This spec adds:

- `CI-LINT-001` — every trigger event has a corresponding open ChangeImpactReport (`SA-MUST-ci-001`).
- `CI-LINT-002` — reports cannot close with unresolved gaps (`SA-MUST-ci-004`).
- `CI-LINT-003` — regressed scenarios block workflow advance until resolved (`SA-MUST-ci-022`).
- `CI-LINT-004` — release notes accompany every superseding workflow version (`SA-MUST-ci-033`).

### Runtime conformance fixtures (Stage 4–5)

- Deterministic propagation: identical state + trigger ⇒ identical report.
- Source supersession demotes citing PolicyObjects → ripples to mappings → ripples to scenarios.
- Scenario regression after change is correctly flagged.
- SemanticDiff is deterministic and includes causal lineage.
- Reports cannot close while affected published packages have unresolved follow-up.

### Current limitations

- Cross-workspace propagation (Phase 3) is not yet specified.
- Cluster detection for `runtime-observation-cluster` triggers is sketched but not pinned.
- Materiality classification is reviewer-driven; the engine's auto-suggestion heuristics are not specified normatively here (Stage 4 detail).

## WOS mappings

ChangeImpactReports are **`authoringOnly`** as a whole — change-management is a Studio-internal concern.

The exception: the **release notes** derived from a SemanticDiff are projected into the new PublishedWorkflowPackage's release notes (per [`review-and-approval.md`](review-and-approval.md) `SA-MUST-ra-030`). A downstream consumer of two consecutive published artifacts can read the release notes to understand what changed.

| Studio object | Mapping state | WOS path |
|---|---|---|
| ChangeImpactReport (full) | `authoringOnly` | — (workspace state) |
| Release notes (from SemanticDiff) | `mapsToWos` (compact projection) | PublishedWorkflowPackage release notes; appears in published artifact metadata |
| Affected-published-packages list | `authoringOnly` | — (workspace state); used to identify which downstream artifacts may need new versions |
| Materiality classifications | `authoringOnly` | — (workspace state) |

The WOS-side concept of a workflow version is `$wosWorkflow`'s `version` field. Studio's ChangeImpactReport coordinates *when* and *why* a new version is needed; the new version itself is a separate publication cycle.

## Examples

### Example 1: Source-version supersession ripples through

A new SourceVersion of `Title-IV-Handbook-2025.pdf` is uploaded; the §668.34 paragraph's text changes by two words. Promoted to `current`, superseding the prior version.

1. **Trigger fires:** `source-version-change`. ChangeImpactReport produced.
2. **Walk:**
   - 12 PolicyObjects cite the prior version. The two-word change affects exactly one cited section (§668.34); 1 PolicyObject is `material`-affected, 11 are `cosmetic` (their citations were elsewhere in the document, unaffected).
   - The 1 affected PolicyObject is a Requirement that maps to `governance.policyParameters.disbursementSchedule`. Its mapping is unaffected (no JSON-path change).
   - The Requirement is referenced by 3 WorkflowIntent elements across 2 WorkflowIntents.
   - 4 Scenarios link the Requirement.
   - 1 PublishedWorkflowPackage is in production carrying the prior version of the workflow.
3. **Report contents:**
   - `affectedPolicyObjects: [requirement-X (material)]`.
   - `affectedWorkflowIntents: [wf-A, wf-B]`.
   - `affectedScenarios: [s1, s2, s3, s4]`.
   - `affectedPublishedPackages: [pkg-A-v3]`.
   - `summary: "Title IV handbook §668.34 was edited; the disbursement-schedule requirement is materially affected and propagates to 2 workflow drafts and 4 scenarios. 1 published package in production carries the older workflow version."`
4. **Reviewer actions (driven by the report):**
   - Reviewer demotes the Requirement to `draft`, edits the citation excerpt to match the new text, re-approves. Mapping moves to `superseded`; new mapping authored at `mapsToWos`.
   - Tier-S5 readiness flags re-running the 4 scenarios. They re-run; 3 pass, 1 regresses.
   - The 1 regression is investigated; turns out the new text changes a deadline that the scenario's `expectedTimer` was checking. Scenario's expected trace updated; re-run passes.
   - WorkflowIntents wf-A and wf-B are re-validated and reach `validationReady → scenarioTested → approved`.
   - For pkg-A-v3 (in production), a new version pkg-A-v4 is scheduled; the report's `affectedPublishedPackages` is acknowledged with closure rationale "v4 publication scheduled for 2026-05-15."
   - Report transitions to `closed`.

### Example 2: Policy object edit, immaterial classification

A reviewer edits a Description field on a Requirement to fix a typo.

1. Trigger: `policy-object-edit`. ChangeImpactReport produced.
2. The change-impact engine's auto-classification suggests `materiality = cosmetic` (description-only edit).
3. The reviewer confirms `cosmetic`; report closes with `closureRationale: "typo fix; no review action."`.
4. No re-approvals are demoted; no scenarios re-run; no release notes are produced.

### Example 3: Workflow-version-compare for release notes

The reviewer asks Studio to compare `wf-A v1.0` (currently published) to `wf-A v1.1` (about to publish).

1. SemanticDiff produced (no propagation — both versions exist):
   - `changedDeadlines: [{stepRef: appeal-deadline, from: 60d, to: 90d, cause: ReviewerResolution rr-2026-04-12 'Adopted federal 90-day rule per AuthorityRank'}]`.
   - `changedNotices: [{stepRef: denial-notice, kind: modified, addedContent: ['Spanish translation requirement'], cause: SourceCitation new-state-directive}]`.
   - `changedScenarios: [{scenarioRef: s2, kind: regression-resolved}]`.
2. Release notes drafted automatically:
   ```
   Workflow wf-A v1.1 release notes
   ============================================
   - Appeal deadline extended from 60 to 90 days. Cause: federal regulation
     7 CFR §273.15 outranks prior state guidance.
   - Denial notice now requires Spanish translation. Cause: state agency
     directive ND-2026-04 effective 2026-04-01.
   - Scenario s2 (Adverse determination + Spanish-speaking applicant) was
     previously regressed; resolved with the Spanish translation requirement.
   ```
3. Reviewer reviews and edits the rendered notes; final notes attached to the new PublishedWorkflowPackage's release notes field.

## Open issues

- **Cross-workspace propagation.** When a SourceVersion is shared across workspaces (per [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §6 deferred reuse), changes must propagate to every workspace. This is Phase-3 territory and not specified normatively here.
- **Cluster detection for runtime triggers.** The `runtime-observation-cluster` trigger's clustering algorithm is sketched but not pinned. Stage-4 work decides.
- **Auto-classification heuristics.** The materiality auto-suggestion is mentioned but not pinned; it depends on the load-bearing-field detection (`SA-MUST-ci-012`).
- **Diff granularity.** SemanticDiff at the field level is normative; whether sub-field diffs (e.g., a single content-element change inside a NoticeRequirement's `content[]` array) get their own diff entries is unsettled.
- **Active-cases enumeration.** The chain identifies *which* published packages are potentially affected; whether Studio attempts to enumerate active cases (which lives in the WOS runtime) or only flags the package is settled in favor of "flag only" — but the boundary may be revisited.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.18, §2.3 (PolicyObject lifecycle demotion), §2.5 (Scenario regression).
- PRD: [`../VISION.md`](../VISION.md) §9.8, §16 Phase-2 Epic 2.5, §16 Phase-3 (full Phase-3 epic set), §12 user stories.
- Upstream (read state): every other Studio spec.
- Downstream (drives action): [`policy-object-model.md`](policy-object-model.md) (demotion), [`scenario-authoring.md`](scenario-authoring.md) (regression flags), [`review-and-approval.md`](review-and-approval.md) (re-approval). (Phase-4 RuntimeObservation cluster trigger is a forward reference; the spec is deferred.)
- WOS: published workflow versioning (`$wosWorkflow.version`), release notes in PublishedWorkflowPackage.
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
