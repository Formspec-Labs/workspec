# Studio Spec: Scenario Authoring

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.13 Scenario, §2.5 Scenario lifecycle.
**PRD anchor:** [`../VISION.md`](../VISION.md) §9.7 (Scenario Simulation and Scenario Artifacts), §16 Phase-2 Epic 2.3.
**Depends on:** [`policy-object-model.md`](policy-object-model.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md).

## Scope

A workflow is not credible until key real-world scenarios are exercised and reviewed (PRD §5 Principle 5). This spec defines the **Scenario artifact** — a durable, authorable, reviewable test case — and the **expected-vs-actual contract** by which scenarios validate WorkflowIntent behavior.

Scenarios are **first-class** authoring objects, not one-off demos. Their durability is what bridges the gap between non-technical reviewer language ("simulate a denial with a missing W-2") and formal conformance ([`../../schemas/wos-tooling.schema.json`](../../schemas/wos-tooling.schema.json) `scenarios[*]`).

This spec defines:

- the Scenario record shape and the 12 scenario types (PRD §9.7);
- the lifecycle (CM §2.5: `generated → reviewed → passing|failing → acceptedAsKnownGap → regression`);
- the **expected-trace** model (the authored expectations) vs. the **actual-trace** model (the simulator's output);
- the **comparison rule** that produces simulation reports;
- the **conformance-trace correspondence** — how a Studio Scenario projects into a WOS conformance trace;
- composition with the Studio→WOS compiler (Stage 5) and the future scenario engine (Stage 6);
- conformance.

## Out of scope

- The simulation runtime implementation (Stage 6).
- The Validation Center UX for scenario reports (PRD §9.6 capability).
- Performance/load testing (a different concern; scenarios are correctness tests).

## Terminology

- **Scenario** — the durable Studio artifact (CM §1.13).
- **Expected trace** — the authored expectations: path, decisions, deadlines triggered, notices generated, outcome.
- **Actual trace** — the simulator's recorded output for a single scenario run.
- **Simulation report** — a reviewer-facing comparison of expected vs. actual.
- **Conformance trace** — the WOS-side artifact (in `wos-tooling.schema.json` `scenarios[*]` shape) emitted at compilation.
- **Critical path** — a workflow path that affects rights or carries adverse outcome (always must have at least one scenario per `SC-LINT-001`/`002` in [`readiness-validation.md`](readiness-validation.md)).

## Data model

### `Scenario` (CM §1.13, extended)

```text
Scenario {
  id, name, purpose, scenarioType, workflowIntentRef,
  linkedPolicyObjects[],  // which PolicyObjects this scenario exercises
  initialCaseState,       // starting case-file shape
  inputs,                 // applicant inputs, attached documents
  eventSequence[],        // ordered events (applicant submits, staff reviews, system checks, time advances, agent actions, …)
  timeAdvances[],         // wall-clock advances injected to fire timers/deadlines
  expectedTrace,          // ExpectedTrace (see below)
  lifecycleState,         // generated | reviewed | passing | failing | acceptedAsKnownGap | regression
  lastSimulation,         // {actualTrace, status, runAt, runEngineVersion}
  reviewState,            // generated | passed | reviewed (per CM §2.5)
  reviewerId?, reviewedAt?,
  acceptedKnownGapRationale?,
  workspaceId, version
}

ExpectedTrace {
  path[],                 // ordered list of expected workflow elements visited
  expectedDecisions[],    // {atStep, expectedOutcome}
  expectedNotices[],      // {atStep, NoticeRequirementRef}
  expectedAppealBranch?,  // when applicable
  expectedTasks[],        // {atStep, TaskMappingRef}
  expectedTimers[],       // {timerRef, expectedFireAt}
  expectedTerminalOutcome,// the final Outcome ref
  expectedProvenance[]    // expected provenance records (subset; not exhaustive)
}

ActualTrace {
  path[],                 // observed
  decisions[],
  notices[],
  appealBranchTaken?,
  tasks[],
  timers[],
  terminalOutcome,
  provenance[],
  divergences[]           // computed diff vs. expected
}
```

`linkedPolicyObjects[]` is **load-bearing**: it explicitly states which approved PolicyObjects the scenario is testing. It is the basis for tier-S5 readiness rules ("every adverse Outcome must have at least one Scenario whose `linkedPolicyObjects[]` includes that Outcome").

### Scenario types (PRD §9.7, normatively listed)

The 12 canonical scenario types — each scenario MUST declare exactly one:

1. **`happy-path`** — typical case, favorable outcome, no exceptions.
2. **`incomplete-application`** — missing data or evidence at intake.
3. **`deadline-missed`** — a timer fires; a deadline elapses.
4. **`adverse-determination`** — denial/termination outcome with required notice.
5. **`notice-generated`** — exercises notice content/delivery requirements.
6. **`appeal-filed`** — exercises the appeal branch.
7. **`exception-applies`** — an ExceptionRule is invoked.
8. **`supporting-document-missing`** — evidence gap at decision time.
9. **`manual-override`** — a staff member overrides a default outcome.
10. **`system-failure-fallback`** — an integration or system check fails; fallback path runs.
11. **`agent-fallback`** — an agent's confidence drops below floor; fallback (typically human review) runs.
12. **`policy-change`** — a SourceVersion changes mid-flight; tests how active cases are handled.

Workspaces MAY define additional `scenarioType` values via an `x-` extension only when no canonical type fits and a reviewer has documented the gap. Stage-3 schema work decides whether the type list is closed; the spec's normative position is that the 12 cover real-world workflows.

## Lifecycle

The Scenario lifecycle from CM §2.5:

```text
generated → reviewed → { passing | failing } → acceptedAsKnownGap → regression
```

Allowed transitions:

| From | To | Trigger |
|---|---|---|
| `generated` | `reviewed` | reviewer confirms the scenario expresses the intended case (the *expectations*, not the simulation outcome) |
| `reviewed` | `passing` | last simulation matched expected outcomes |
| `reviewed` | `failing` | last simulation diverged |
| `passing` | `failing` | a re-run after a workflow change diverges; this is a **regression** (see `regression` state) |
| `failing` | `acceptedAsKnownGap` | reviewer waives the divergence with rationale |
| `failing` | `passing` | the workflow or scenario was edited to make them agree; re-run passes |
| `acceptedAsKnownGap` | `passing` | the underlying gap was fixed; re-run passes; the known-gap exit is recorded |
| `passing` | `regression` | a re-run flips a previously-passing scenario to failing — escalates above ordinary `failing` |
| `regression` | `failing` | reviewer triages the regression as a non-emergency known failure |
| `regression` | `passing` | regression resolved; re-run passes |

The `regression` state is **escalation-only** — it is `failing` plus a marker that this scenario was passing before. Tier-S5 readiness rules treat `regression` as more severe than ordinary `failing`.

## Normative Contract

### Authorship and content

- **`SA-MUST-scn-001`** — Every Scenario MUST declare a `scenarioType` (one of the 12 canonical types) and at least one `linkedPolicyObjects` entry. Scenarios with empty `linkedPolicyObjects[]` MUST be rejected at creation. *(schema-pending: required field; lint-pending: tier-S5 rule.)*
- **`SA-MUST-scn-002`** — Every Scenario MUST carry an `expectedTrace.expectedTerminalOutcome` referencing an Outcome PolicyObject in the host WorkflowIntent. *(schema-pending.)*
- **`SA-MUST-scn-003`** — When `scenarioType = adverse-determination`, the Scenario MUST exercise at least one Outcome where `polarity = adverse` and `triggersDueProcess = true`; the `expectedNotices[]` MUST list at least one corresponding NoticeRequirement. *(lint-pending: `SC-LINT-001` cross-cutting with [`policy-object-model.md`](policy-object-model.md) `SA-MUST-pom-030`.)*
- **`SA-MUST-scn-004`** — When `scenarioType = appeal-filed`, the Scenario MUST exercise the appeal branch from the corresponding NoticeRequirement to the AppealRight. *(lint-pending: `SC-LINT-002`.)*
- **`SA-MUST-scn-005`** — When `scenarioType = agent-fallback`, the Scenario MUST drive an ActorMapping with `actorKind = agent` to its fallback chain — confidence-floor breach, deontic violation, or capability gap. The Scenario MUST link the AI-Use PolicyObject. *(lint-pending; cross-cutting with `SA-MUST-pom-036`.)*
- **`SA-MUST-scn-006`** — Every Scenario MUST identify a host `workflowIntentRef`. Scenarios without a host workflow MUST be rejected. *(schema-pending.)*
- **`SA-SHOULD-scn-007`** — A Scenario's `name` SHOULD be reviewer-readable in plain language ("Denial with notice and appeal — household income above threshold"); machine-generated names are valid but reviewer renaming is encouraged at the `generated → reviewed` transition.

### Simulation and expected vs. actual

- **`SA-MUST-scn-010`** — A Scenario simulation MUST produce an ActualTrace populated from the simulator's observed events. The simulator MUST NOT silently substitute expected values when an event was not actually produced. *(runtime-pending: scenario engine, Stage 6.)*
- **`SA-MUST-scn-011`** — The comparison engine MUST compute `divergences[]` by comparing each ExpectedTrace field against the corresponding ActualTrace field. Divergences are typed (`path-mismatch` | `decision-mismatch` | `notice-missing` | `notice-extra` | `task-missing` | `timer-mismatch` | `outcome-mismatch` | `provenance-missing`). *(runtime-pending.)*
- **`SA-MUST-scn-012`** — A Scenario MUST be marked `passing` only when `divergences[]` is empty. Any non-empty `divergences[]` produces `failing`. *(runtime-pending.)*
- **`SA-MUST-scn-013`** — The simulation report MUST be reviewer-readable: each divergence rendered in plain language, linked to the Studio object whose expected behavior diverged. *(runtime-pending: report rendering.)*
- **`SA-MUST-scn-014`** — Simulation MUST be deterministic given the same {WorkflowIntent version, ExpectedTrace, scenario engine version, simulated time}. Non-determinism in workflow definitions (e.g., un-seeded random branches) MUST surface as a tier-S4 `WF-LINT-009` finding (forthcoming) before the scenario can be authored. *(fixture-pending.)*

### Conformance trace correspondence

The Studio→WOS compiler (Stage 5) emits a corresponding **WOS conformance trace** for each Studio Scenario. The correspondence is the **load-bearing semantic bridge** between Studio's authorable scenarios and WOS's executable conformance fixtures.

- **`SA-MUST-scn-020`** — For every reviewed Scenario, the compiler MUST emit a corresponding entry in [`../../schemas/wos-tooling.schema.json`](../../schemas/wos-tooling.schema.json) `scenarios[*]`. Scenarios in `lifecycleState = generated` (not yet reviewed) MUST NOT be emitted. *(runtime-pending: compiler emission.)*
- **`SA-MUST-scn-021`** — The emitted conformance trace's id MUST be deterministic from the Studio Scenario's id (e.g., `wos-scenario-${studio-scenario-id}-v${version}`) so that downstream WOS conformance reports trace back to the Studio source. *(runtime-pending.)*
- **`SA-MUST-scn-022`** — The emitted conformance trace MUST carry: the WorkflowIntent's compiled `$wosWorkflow` URL, the inputs, the event sequence, the expected terminal outcome, and the expected notice/appeal/timer events that map to WOS-side observables. Studio-only fields (`linkedPolicyObjects`, `acceptedKnownGapRationale`, `lastSimulation`, `reviewerId`) are **not** emitted. *(runtime-pending: projection schema.)*
- **`SA-MUST-scn-023`** — When a Scenario is `failing` or `acceptedAsKnownGap`, the conformance trace MUST be emitted with a `status` flag (`failing` or `accepted-gap`) so WOS conformance does not treat it as expected-passing. *(runtime-pending; cross-cutting with `crates/wos-conformance` semantics.)*
- **`SA-MUST-scn-024`** — Running WOS conformance against the published `$wosWorkflow` MUST exercise the emitted scenarios; failing scenarios that were `passing` in Studio MUST surface as Studio tier-S6 `PUB-LINT-005` findings (lifted from WOS conformance — see [`readiness-validation.md`](readiness-validation.md)). *(runtime-pending; cross-cutting.)*
- **`SA-SHOULD-scn-025`** — When the Studio simulation engine and the WOS conformance engine disagree on a scenario's outcome, the disagreement is a **bug** — Studio's simulator should match WOS's runtime behavior. Disagreements SHOULD surface as a special tier-S5 `SC-LINT-006` finding `simulator-conformance-mismatch` for engineering attention.

### Cross-spec coupling

- **`SA-MUST-scn-030`** — Every Scenario lifecycle change MUST emit AuthoringProvenanceRecords with `eventKind = scenarioTested` (or `findingRaised`/`findingResolved` for divergences). *(runtime-pending; cross-cutting with [`authoring-provenance.md`](authoring-provenance.md).)*
- **`SA-MUST-scn-031`** — When a referenced PolicyObject is demoted (e.g., due to source supersession per [`source-vault.md`](source-vault.md) `SA-MUST-source-021`) the Scenarios listing it in `linkedPolicyObjects[]` MUST move from `passing` to `regression` if they were previously `passing`, or from `reviewed` to `generated` if they were unreviewed at the time of change. *(runtime-pending; cross-cutting with [`change-impact.md`](change-impact.md).)*

## Composition

### Attachment point

Scenarios attach to a single host WorkflowIntent within a workspace. They are workspace-state artifacts. The Studio→WOS compiler (Stage 5) emits a projection into the published artifact's tooling configuration; the future scenario engine (Stage 6) consumes Scenarios for simulation.

A Scenario MAY reference PolicyObjects from outside its host WorkflowIntent (cross-workflow references), but the host workflow's compiler is the only emitter — cross-workflow conformance traces are not produced.

### Precedence

When a Scenario's expected behavior conflicts with the WorkflowIntent's compiled WOS behavior (e.g., expected outcome differs from what the workflow actually produces), the **scenario is the test, not the spec** — the workflow is not adjusted to match the scenario. The reviewer either:

1. Edits the workflow (because the workflow was wrong), then re-runs the scenario.
2. Edits the scenario's `expectedTrace` (because the scenario was wrong), then re-runs.
3. Marks the scenario `acceptedAsKnownGap` with rationale.

Studio MUST NOT auto-adjust the workflow to satisfy a failing scenario.

### Conflict handling

Two Scenarios with contradictory expectations on the same workflow path are valid — they may be exploring different scenario types (e.g., a happy-path and an adverse-determination both visit the same intake step but diverge later). The spec does not surface "scenario-vs-scenario" conflicts unless both expect the same `expectedTerminalOutcome` for an identical `eventSequence`, in which case a tier-S5 `SC-LINT-007` finding (forthcoming) flags the duplication.

### Versioning / migration

- A Scenario carries its own `version` independent of the WorkflowIntent.
- Workflow version bumps invalidate scenario `lastSimulation` results — re-running is required before the new workflow version can advance.
- Adding a new scenario type to the canonical 12 is **schema-breaking**; removing one is also breaking.
- Adding new ExpectedTrace fields (e.g., `expectedAgentInvocations[]` in a future Phase) is non-breaking if optional, breaking if required.

## Conformance

### Schema validation (Stage 3)

Planned schema gates:

- Scenario required fields and lifecycle enum.
- `scenarioType` enum (the 12 canonical types).
- ExpectedTrace shape; ActualTrace shape; Divergence taxonomy.
- Conformance-trace projection schema (the subset emitted to `wos-tooling.schema.json`).

### Lint rules (Stage 4)

Tier-S5 readiness rules planned (introduced in [`readiness-validation.md`](readiness-validation.md)):

- `SC-LINT-001` — every adverse Outcome has at least one Scenario.
- `SC-LINT-002` — every AppealRight has at least one Scenario.
- `SC-LINT-003` — every Scenario carries `expectedTerminalOutcome`.
- `SC-LINT-004` — failing Scenarios are accepted-as-known-gap or block advance.
- `SC-LINT-005` — post-supersession re-run gate.
- `SC-LINT-006` — simulator-vs-conformance mismatch (cross-cutting).
- `SC-LINT-007` — duplicate scenarios with contradictory expectations (forthcoming).

### Runtime conformance fixtures (Stage 4–6)

- Deterministic simulation given identical inputs (`SA-MUST-scn-014`).
- Failing scenario blocks advance to `scenarioTested` (cross-cutting with `SA-MUST-rv-040`).
- WOS conformance run on published artifact replays emitted scenarios.
- Cross-spec demotion: PolicyObject demotion ripples Scenario lifecycle (`SA-MUST-scn-031`).

### Current limitations

- The simulator (Stage 6) does not yet exist; tier-S5 readiness is therefore reviewer-driven until the engine ships.
- The exact projection schema for conformance traces depends on Stage-3 decisions about `wos-tooling.schema.json` shape.

## WOS mappings

Scenarios are **`mapsToWos`** at the projection layer; the Studio Scenario itself is workspace-state, but the projected conformance trace is a load-bearing WOS-side artifact.

| Studio object | Mapping state | WOS path |
|---|---|---|
| Scenario (full) | `authoringOnly` | — (workspace state) |
| Conformance trace projection | `mapsToWos` | [`../../schemas/wos-tooling.schema.json`](../../schemas/wos-tooling.schema.json) `scenarios[*]` |
| Scenario lifecycle / review state | `authoringOnly` | — (workspace state) |
| Linked PolicyObject refs | `authoringOnly` | — (workspace state; the policy objects themselves project per [`policy-object-model.md`](policy-object-model.md)) |
| Last-simulation `actualTrace` | `authoringOnly` | — (the WOS conformance run produces its own trace at conformance time) |

The published artifact's tooling section (per `wos-tooling.schema.json`) carries the scenarios; running WOS conformance against the published artifact regenerates the actual traces with the WOS runtime, providing an independent verification.

## Examples

### Example 1: Happy-path SNAP application

Scenario name: "Eligible household, complete application, approved within 30 days"

```text
scenarioType: happy-path
linkedPolicyObjects: [eligibility-Requirement, intake-EvidenceRequirement, approval-Outcome]
initialCaseState: { applicantId: "test-001", program: SNAP }
inputs: { householdSize: 3, monthlyIncome: 1200, expenses: 800, citizenshipDocs: [...] }
eventSequence: [
  { kind: applicantSubmits, at: T+0 },
  { kind: staffReviews,    at: T+3d },
  { kind: systemChecks,    at: T+3d, source: "ws-data-broker" },
  { kind: staffDecides,    at: T+5d, decision: approve }
]
timeAdvances: [T+0, T+3d, T+5d]
expectedTrace: {
  path: [intake, eligibility-check, decision, notice, terminal],
  expectedDecisions: [{ atStep: decision, expectedOutcome: approval-Outcome }],
  expectedNotices: [{ atStep: notice, NoticeRequirementRef: approval-notice }],
  expectedTerminalOutcome: approval-Outcome
}
```

After `generated → reviewed → passing`, the compiler emits a corresponding `wos-tooling.scenarios[]` entry with id `wos-scenario-snap-happy-path-v1`. WOS conformance against the published artifact replays this and verifies the actual outcome matches.

### Example 2: Adverse determination + appeal

Scenario name: "Income above threshold, denied with notice and appeal filed"

```text
scenarioType: appeal-filed
linkedPolicyObjects: [eligibility-Requirement, denial-Outcome, denial-NoticeRequirement, appeal-AppealRight]
initialCaseState: { applicantId: "test-002", program: SNAP }
inputs: { householdSize: 2, monthlyIncome: 4500, ... }
eventSequence: [
  { kind: applicantSubmits, at: T+0 },
  { kind: staffReviews,    at: T+3d },
  { kind: staffDecides,    at: T+5d, decision: deny },
  { kind: noticeIssued,    at: T+5d },
  { kind: applicantAppeals, at: T+15d }
]
expectedTrace: {
  path: [intake, eligibility-check, decision, notice, await-appeal, appeal-branch, hearing],
  expectedNotices: [denial-notice (with required content + 90-day appeal language)],
  expectedAppealBranch: { from: notice, to: hearing, deadline: 90d },
  expectedTerminalOutcome: appeal-pending-Outcome
}
```

Tier-S5 readiness `SC-LINT-002` is satisfied: this scenario exercises the AppealRight branch.

### Example 3: Agent fallback

Scenario name: "Agent confidence < 0.7 on incomplete-evidence triage; routes to human review"

```text
scenarioType: agent-fallback
linkedPolicyObjects: [triage-AIUse, fallback-Obligation, human-review-TaskMapping]
initialCaseState: { applicantId: "test-003", program: SNAP }
inputs: { householdSize: 4, monthlyIncome: 2200, evidenceFlags: [w2-missing, paystub-missing] }
eventSequence: [
  { kind: applicantSubmits, at: T+0 },
  { kind: agentInvoked,    at: T+0, agentId: triage-agent, observedConfidence: 0.42 },
  { kind: agentFallbackTriggered, at: T+0, reason: "confidence-below-floor" },
  { kind: humanReviewQueued, at: T+0 }
]
expectedTrace: {
  path: [intake, agent-triage, agent-fallback, human-review-queue],
  expectedTasks: [human-review-TaskMapping],
  expectedTerminalOutcome: pending-human-review-Outcome
}
```

This satisfies `SA-MUST-scn-005` and supports the parent CLAUDE.md invariant that agent fallback chains terminate in human review.

## Open issues

- **Scenario type closure.** Whether the 12 canonical types are closed at Stage 3 or open via `x-` is unsettled.
- **Multi-scenario suites.** A workflow's full scenario suite is currently a flat list; whether suites organize hierarchically (by program, by outcome, by tier) is unsettled. Stage-3 schema work decides.
- **Random/property-based scenarios.** Some test paradigms use generated rather than authored scenarios. Whether Studio supports property-based test generation (with fixed-seed determinism per `SA-MUST-scn-014`) is deferred.
- **Cross-workflow scenarios.** A scenario that crosses two workflows (e.g., SNAP intake feeding into TANF eligibility) is not directly representable today. Whether to support is deferred.
- **Conformance-trace projection schema.** The exact shape of the emitted entry depends on Stage-3 decisions about `wos-tooling.schema.json`. Currently the spec describes the projection rule but not the field-by-field shape.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.13, §2.5.
- PRD: [`../VISION.md`](../VISION.md) §9.7, §16 Phase-2 Epic 2.3, §12 user stories.
- Upstream: [`policy-object-model.md`](policy-object-model.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md).
- Downstream: [`readiness-validation.md`](readiness-validation.md), [`change-impact.md`](change-impact.md), [`runtime-observation.md`](runtime-observation.md).
- WOS: [`../../schemas/wos-tooling.schema.json`](../../schemas/wos-tooling.schema.json), [`../../crates/wos-conformance`](../../crates/wos-conformance), [`../../specs/profiles/`](../../specs/profiles/).
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
