# Vertical slice: SNAP Redetermination from sources

**Persona:** Sarah Chen, SNAP redetermination program manager (per studio-authoring persona testing rounds 1+2). Her four real source documents become a published `$wosWorkflow` artifact via the Studio authoring journey.

**Goal:** prove the v4 framework produces real `wos-spec` content end-to-end. Specifically: close the BLUF's "wrong-by-omission" gap by demonstrating that Studio's readiness rules force a workflow with realistic pipeline stages, multiple deontic constraints, multi-signer signature flow, and equity monitoring.

## Sources (4 documents)

[`sources/`](sources/):

1. [`7-cfr-273-redetermination-excerpt.md`](sources/7-cfr-273-redetermination-excerpt.md) — federal SNAP redetermination rules excerpt (illustrative).
2. [`state-snap-manual-ch8-excerpt.md`](sources/state-snap-manual-ch8-excerpt.md) — state interpretation, occasionally narrower than CFR (illustrative).
3. [`office-memo-recert-v3.2.md`](sources/office-memo-recert-v3.2.md) — office practice with non-policy operational caveats (illustrative).
4. [`2024-corrective-action-letter.md`](sources/2024-corrective-action-letter.md) — federal letter superseding two practices in the office memo (illustrative; closes Sarah's "cross-document supersession" task from persona round 1).

The four documents collectively exercise every source-vault concept: parsed/indexed/classified versions, multi-jurisdiction effectiveness, **cross-document supersession** (the corrective-action letter overrides parts of the office memo), JSON-LD ingest path candidate (the federal CFR is JSON-LD-published on eCFR.gov).

## PolicyObjects extracted

[`policy-objects/`](policy-objects/):

The structured intermediate representation. Each file is one PolicyObject, citing its source, carrying provenance, declaring its mapping. Demonstrates:

- **WOS-projecting kinds** (NoticeRequirement, AppealRight, Deadline, DecisionRule, Outcome, ActorMapping, EvidenceRequirement) — these compile to `$wosWorkflow` body content.
- **Studio-only kinds** (PolicySource, AuthorityRank, Supersession, Conflict, Assumption, ProtectedCategory) — these stay workspace-side.
- **Bridge kinds** (WorkflowStepMapping, TransitionMapping, TimerMapping, ActorMapping, TaskMapping, CaseFileMapping) — these produce kernel constructs.
- **Effectiveness object** referenced by ref (not copied) on multiple PolicyObjects per CM §1.25.
- **Deontic kinds** (Obligation, Permission, Prohibition) with composition rules per `policy-object-model.md` §"Deontic constraint composition".
- **DPV sensitivity** on every DataElement (e.g., `dpv:GovernmentBenefit`, `dpv:HealthData`, `dpv:FinancialPreference`).
- **AI extraction provenance** (every PolicyObject's provenance carries an `aiLineage` block where AI assistance was used).
- **Cross-document supersession** via `Supersession` PolicyObject (the corrective-action letter overrides the office memo).

## Mappings

[`mappings/`](mappings/):

Each PolicyObject's StudioToWosMapping record. Demonstrates:

- `mapsToWos` for the WOS-projecting kinds.
- `authoringOnly` for Studio-only kinds.
- `requiresSpecExtension` with ExtensionRecord candidate for the workflow-level Effectiveness (`ApplicabilityScope` slight-extension proposal queued in `studio-to-wos-mapping.md`).
- `unmappedButApproved` with explicit rationale for one workspace-specific operational practice that has no current WOS counterpart (Sarah's "Christmas-to-New-Year informal extension" from persona round 1).

## WorkflowIntent

[`workflow-intent.json`](workflow-intent.json):

The user-facing draft. 16 `WorkflowElement`s spanning the canonical kinds (phase / step / decision / review / notice / deadline / appeal / exception / hold / data-collection / evidence-request / system-check / AI-assistance / manual-override / completion-outcome / phase-end). Carries `wosVersionPin`, workflow-level `effectivenessRef`, and per-element `effectivenessRef` where narrowing is explicit (e.g., the Spanish-translation NoticeRequirement narrows to one state).

## Compiled artifact (the wos-spec content!)

[`wos-workflow.json`](wos-workflow.json):

The compiled `$wosWorkflow` document. Shape-compatible with parent [`../../../examples/benefits-adjudication.workflow.json`](../../../examples/benefits-adjudication.workflow.json). Demonstrates:

- All ADR-0076 D-2 embedded blocks present where applicable (`governance`, `agents`, `aiOversight`, `signature`, `custody`, `advanced`, `assurance`).
- Multi-stage pipeline (intake → eligibility check → determination → notice → appeal-or-close), exceeding the BLUF's flagged 3-toy-stage limit.
- Multi-signer signature (caseworker + supervisor for adverse outcomes), exceeding the BLUF's flagged 1-signer limit.
- Multiple deontic constraints (Obligation to send notice, Permission for manual override under documented authority, Prohibition on adverse decision without case-worker review), exceeding the BLUF's flagged 4-toy-deontic limit.
- Equity monitoring via `advanced.equity.protectedCategories[*]` for race/ethnicity, language-spoken, disability.
- Integration bindings: federal income-verify ServiceBinding, application.submitted EventBinding, policy-engine fraud-screen binding.

## Scenarios

[`scenarios/`](scenarios/):

`wos-tooling.scenarios[*]` entries exercising the workflow. Includes:

- `happy-path` — eligible household, approval.
- `adverse-determination` — denial with notice + appeal.
- `appeal-filed` — exercises the appeal sub-flow.
- `equity-probe` — varies cohorts across race/ethnicity to probe disparate impact (per `scenario-authoring.md` `SA-MUST-scn-040`).
- `accessibility-check` — verifies notice content satisfies WCAG / multi-language requirements.
- `manual-override` — exercises caseworker override with rationale.

## ApprovalPackage

[`approval-package.json`](approval-package.json):

The publication artifact bundle. Demonstrates:

- `wosVersionPin` recording the parent stream versions.
- `complianceAttestations[]` for SOC2-Type-II + StateRAMP-Moderate (workspace-declared baselines).
- `identitySigningKeyRefs[]` enabling downstream verification.
- `custodyAnchorReceipt` cryptographically anchoring the package per parent `custodyHook` per ADR-0061.
- `unmappedListings[]` enumerating the one `unmappedButApproved` mapping (workspace-specific holiday-period flex).
- `approvals[]` including approvals from compliance-reviewer, legal-reviewer, technical-reviewer, operations-reviewer (multi-role gating per workspace policy).

## "Wrong-by-omission" analysis

[`wrong-by-omission-analysis.md`](wrong-by-omission-analysis.md):

Maps each BLUF-flagged "wrong-by-omission" gap to the Studio readiness rule that catches it. The slice proves: a workflow authored via Studio cannot exhibit the BLUF's wrong-by-omission classes because the readiness rules block publication.

## What Stage-3+ tightens

- JSON Schema validation against Studio Stage-3 schemas (landed 2026-05-01 at `studio/schemas/`) + parent `wos-workflow.schema.json` for the compiled artifact. After Wave-2 review remediation (2026-05-02): `wos-workflow.json` validates clean against the parent schema; collection-form policy-object documents validate via the Stage-3 `oneOf` shape; Effectiveness, IdentitySubject, Binding, and Source artifacts all materialized under `effectiveness/`, `identity/`, `bindings/`, `sources/` directories.
- Stage-4 lint runs: every readiness-rule fires correctly.
- Stage-5 compiler reproduces `wos-workflow.json` byte-for-byte from `policy-objects/ + mappings/ + workflow-intent.json`.
- Stage-6 scenario simulator runs the scenarios and verifies expected vs. actual.
- Stage-7 reference architecture documents how the slice runs on a real adapter (Restate target).
- Stage-8 promotes this slice to a fully end-to-end runnable example (FAFSA ISIR is the planned long-form companion).

## Cross-references

- Studio specs: [`../../specs/`](../../specs/).
- Studio concept model: [`../../CONCEPT-MODEL.md`](../../CONCEPT-MODEL.md).
- Parent examples: [`../../../examples/benefits-adjudication.workflow.json`](../../../examples/benefits-adjudication.workflow.json) + [BLUF](../../../examples/benefits-adjudication.bluf.md).
- Parent conformance fixtures: [`../../../crates/wos-conformance/fixtures/`](../../../crates/wos-conformance/fixtures/).
