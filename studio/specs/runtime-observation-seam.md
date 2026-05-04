# Studio Spec: Runtime Observation Seam

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap)) — **seam contract only**; full Phase-4 implementation deferred.
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.17 (RuntimeObservation, Phase-4 placeholder), §1.26 (RuntimeObservationSeam — defined HERE).
**PRD anchor:** [`../VISION.md`](../VISION.md) §9.11 (runtime-observation-driven improvement), §17 Phase 4 (Runtime feedback).
**Depends on:** [`change-impact.md`](change-impact.md), [`authoring-provenance.md`](authoring-provenance.md), [`scenario-authoring.md`](scenario-authoring.md), [`binding-and-integration.md`](binding-and-integration.md).

## Why this spec exists (and why it's seam-only)

Three current specs already reference RuntimeObservation as if it had a wire format:

- `change-impact.md` lists `triggerKind = runtime-observation-cluster` as a ChangeImpactReport trigger.
- `authoring-provenance.md` lists `originClass = runtime-observed` as one of the five claim provenance origins.
- `CONCEPT-MODEL.md` §1.17 reserves the entity name.

Without this spec those three references are dangling — a structural debt today, not in 18 months. **A seam contract — the wire format and ingest path — is specified now so that the rest of the spec set has resolvable references. The simulator/replayer/cluster-detector implementation is Phase-4.**

The Plan-agent-recommended frame: this is a **contract definition, not a spec of semantics**. It locks the surface so Phase-4 implementers (1) write only the importer/replayer/cluster bits and (2) inherit Studio-side wiring already in place.

## Scope

This spec defines:

- The **wire format** of a RuntimeObservation (the case-trace data structure that crosses the seam).
- The **ingest path** options (subscription, poll, batch import).
- The **trigger contract** for cluster detection (when do N observations trigger a ChangeImpactReport).
- The **replay contract** (how an observation is replayed against the published workflow for divergence detection).
- The **promotion contract** (how an observation becomes a candidate Scenario or PolicyObject claim).

## Out of scope

- The implementation of any of the above (Phase 4).
- The runtime-side emitter (lives in `crates/wos-runtime` per parent repo; this spec defines what Studio consumes, not what runtime emits).
- The cluster-detection heuristics (Phase 4 — needs production traffic to calibrate).
- Privacy de-identification of observed traces before ingest (Phase 4 — depends on parent encryption/key-bag work in `crates/wos-server/VISION.md`).
- A UI for browsing observations (Phase 4 product work).

## Terminology

- **Observation** — one normalized case trace; the unit at the seam.
- **Cluster** — N observations sharing a structural divergence pattern; the unit that triggers a ChangeImpactReport.
- **Replay** — re-execution of an observation against the currently-published WorkflowIntent for designed-vs-observed comparison.
- **Promotion** — converting an observation into a candidate Scenario or candidate PolicyObject claim.
- **Seam** — the contract surface this spec defines. Three Phase-4 implementations satisfy this seam (importer, replayer, cluster-detector); they are NOT this spec.

## Wire format

A RuntimeObservation arriving across the seam carries this normalized shape:

```text
RuntimeObservation {
  id, observedAt, sourceSystem,
  caseId,                          // case in the source system; opaque to Studio
  workflowVersion {                // which published workflow the case followed
    workflowIntentId,
    publishedVersion,
    wosVersionPin                  // per §1.33 MigrationPath
  },
  caseFileSnapshot? {              // sanitized — sensitive fields per DPV class redacted unless explicit grant
    ... (opaque snapshot for replay)
  },
  eventSequence[] {
    eventId, eventName, recordedAt, recordedBy,
    payloadShape (typed; sensitive fields redacted per §1.32 DPV vocabulary),
    cloudEventsExtensions {        // per binding-and-integration.md
      woscausationeventid,
      woscorrelationkey
    }
  },
  decisionOutcomes[] {
    decisionRef (workflow element id),
    outcomeRef,
    rationale?,
    confidence?,
    actorMappingRef
  },
  manualOverrides[] {
    overrideRef (workflow element id),
    overridingActorMappingRef,
    rationale,
    recordedAt
  },
  unmodeledSteps[] {
    stepDescription,
    occurredAt,
    actorMappingRef?,              // null when no current ActorMapping covers this actor
    flagReason ('outside-published-flow' | 'actor-not-modeled' | 'state-not-modeled' | ...)
  },
  bottlenecks[] {
    elementRef,
    durationP50, durationP95,
    sampleSize,
    flagReason ('exceeds-deadline' | 'exceeds-historical-baseline' | ...)
  },
  causalChain {
    priorObservationId?,           // when this observation succeeds another (e.g., appeal of a prior denial)
    correlationKey                 // per CloudEvents
  },
  privacyClassMembership[]         // per §1.32 DPV: which class buckets this observation belongs to (so reviewers see only what they're cleared for)
}
```

The shape is **deliberately conservative**: every field is optional except `id`, `observedAt`, `sourceSystem`, `caseId`, `workflowVersion`. The wire format admits partial observations (e.g., a runtime that emits only `eventSequence` and nothing else); cluster detection can still derive useful signal from sparse data.

## Ingest paths

Three modes are reserved at the seam; Phase-4 chooses one or implements all three:

1. **Subscription** — Studio subscribes to a kernel-event-named stream (per `binding-and-integration.md` EventBinding). The runtime emits observations as events as they happen; Studio's seam consumer normalizes them into the wire format above.
2. **Poll** — Studio polls a runtime-side endpoint at intervals.
3. **Batch import** — operator-driven export from runtime → file → Studio import.

All three modes produce identical RuntimeObservation entries in the workspace; downstream processing does not differ by ingest path.

## Trigger contract (cluster detection)

A **cluster** is N observations sharing a structural divergence. Phase-4 cluster-detection is a heuristic engine; this spec defines the contract:

- Input: a stream / batch of observations.
- Output: zero or more `Cluster` records, each enumerating: cluster pattern (`stuck-at-step` | `unmodeled-divergence` | `bottleneck-spike` | `manual-override-pattern` | `decision-outcome-drift`), participating observation ids, divergence summary, suggested PolicyObject targets to revisit.

A cluster of severity ≥ `acknowledge-required` produces a ChangeImpactReport with `triggerKind = runtime-observation-cluster` per [`change-impact.md`](change-impact.md). The cluster's `suggestedTargets` populate the report's `affectedPolicyObjects[]` and `affectedScenarios[]`.

## Replay contract

Studio CAN replay an observation against the currently-published WorkflowIntent for divergence detection:

- Input: one RuntimeObservation + the WorkflowIntent it claims to have followed.
- Output: a `ReplayResult` with `divergencePoints[]` enumerating where designed and observed traces diverge; `divergenceKind` per point (`branch-not-taken` | `transition-blocked-by-guard` | `unmodeled-state-reached` | `actor-not-permitted` | `deadline-violation` | etc.).
- A replay does NOT modify state; it produces a comparison artifact.

The replay implementation reuses the same DurableRuntime adapter Studio's compiler targets (per parent CLAUDE.md "DurableRuntime — the line between spec-authoritative semantics and adapter-tier orchestration"); Studio does NOT implement a separate replay engine.

## Promotion contract

An observation MAY be promoted into a candidate Scenario or a candidate PolicyObject claim:

- **Observation → Scenario** (per `scenario-authoring.md` runtime-observation-replay scenario type): the observation's eventSequence becomes a Scenario's expected sequence; reviewer adjusts as needed; the scenario joins the workflow's scenario suite.
- **Observation → PolicyObject candidate**: an unmodeled step / unmodeled actor / consistent manual-override pattern surfaces a candidate ExtractedClaim with `originClass = runtime-observed`; reviewer either approves it (the model gains the missing element) or rejects it (the observed behavior is out-of-policy and the workflow stands).

Both promotion paths run through the standard review pipeline (reviewer attestation + provenance record + lifecycle gates). **Runtime observations never bypass review.**

## Normative Contract

### Seam integrity

- **`SA-MUST-rtos-001`** — Every RuntimeObservation arriving at the seam MUST carry `id`, `observedAt`, `sourceSystem`, `caseId`, and `workflowVersion`. Observations missing any of these MUST be rejected at ingest. *(schema-pending: Phase 4.)*
- **`SA-MUST-rtos-002`** — RuntimeObservations MUST carry privacy-class-membership flags; downstream rendering MUST suppress fields outside the viewing reviewer's authority grants. *(substrate-pending: Phase 4 + parent encryption/key-bag work.)*
- **`SA-MUST-rtos-003`** — Sensitive fields (DPV `dpv:HealthData`, `dpv:Identifier`, `dpv:FinancialPreference`, ...) MUST be redacted unless an explicit AuthorityGrant authorizes the viewing reviewer to see them. *(substrate-pending: Phase 4.)*

### Cluster detection

- **`SA-MUST-rtos-010`** — A Cluster of severity ≥ `acknowledge-required` MUST produce a ChangeImpactReport with `triggerKind = runtime-observation-cluster`. *(substrate-pending: Phase 4.)*
- **`SA-MUST-rtos-011`** — Cluster severity MUST be derived from a workspace-policy-configured threshold (e.g., "10+ observations in 7 days at the same step constitutes acknowledge-required"). The cluster engine MUST NOT auto-set thresholds; workspace administrators do. *(substrate-pending: Phase 4.)*

### Replay

- **`SA-MUST-rtos-020`** — A replay MUST NOT modify workspace state; it produces a comparison artifact only. *(substrate-pending: Phase 4.)*
- **`SA-MUST-rtos-021`** — Replay MUST use the same DurableRuntime adapter the compiler targets for the published workflow. Replays against a different adapter than the one the runtime used MUST be flagged as `replay-adapter-mismatch` in the ReplayResult. *(substrate-pending.)*

### Promotion

- **`SA-MUST-rtos-030`** — Promoting an observation to a Scenario or to a candidate PolicyObject claim MUST go through the standard review pipeline: reviewer attestation + AuthoringProvenanceRecord + lifecycle gates. **Runtime observations never bypass review.** *(substrate-pending: Phase 4.)*
- **`SA-MUST-rtos-031`** — Promoted PolicyObject claims MUST carry `originClass = runtime-observed` per `authoring-provenance.md`; promoted Scenarios MUST carry the runtime-observation-replay type per `scenario-authoring.md`. *(substrate-pending: Phase 4.)*

## Composition

### Attachment point

The seam attaches at the Workspace level. Each Workspace MAY enable runtime observation ingest; ingest is opt-in per workspace policy.

### Precedence

Where an observation conflicts with the published WorkflowIntent (e.g., observed manual override in a path the intent does not allow), reviewer judgment governs. The cluster detector flags; reviewers adjudicate. **The observation does not override the intent automatically.**

### Versioning / migration

- Wire format additions: non-breaking if optional.
- Wire format field removals: schema-breaking; require deprecation cycle.
- Cluster engine heuristic changes: workspace-policy-configurable; not schema-breaking.

## Conformance

### Schema validation (Stage 3)

- RuntimeObservation wire format required fields enforced (`SA-MUST-rtos-001`).
- Privacy-class-membership shape (per §1.32 DPV).
- Cluster record shape and severity ladder.
- ReplayResult divergence-point shape.

### Lint rules (Stage 4)

- `RTOS-LINT-001` — observations targeting an unrecognized `workflowVersion` are flagged.
- `RTOS-LINT-002` — clusters with `severity ≥ acknowledge-required` lacking a corresponding ChangeImpactReport are flagged.
- `RTOS-LINT-003` — promoted PolicyObjects without reviewer attestation are flagged.

### Runtime conformance fixtures (Stage 4–5; substantive testing in Phase 4)

- Observation with all required fields ingests successfully.
- Observation with redactions matches authority grants for the viewing reviewer.
- Cluster of severity `acknowledge-required` produces ChangeImpactReport.
- Replay of a happy-path observation produces zero divergence points.
- Replay of a manual-override observation flags the override path correctly.
- Promotion of an observation to a Scenario goes through review pipeline.

### Current limitations

- Cluster heuristics (which patterns rise to which severity thresholds) are workspace-policy-configurable; the default policy is sketched in `workspace.md` but not specified here.
- Replay against parallel-region workflows requires the DurableRuntime adapter to support deterministic parallel replay; the constraint is reflected in `crates/wos-runtime` companion notes.
- Cross-workspace observation sharing is deferred (federation; §1.34).

## WOS mappings

RuntimeObservation, Cluster, ReplayResult, and seam-side promotion artifacts are **`authoringOnly`** as a whole — they are Studio-internal concerns and never appear directly in `$wosWorkflow`.

| Studio object | Mapping state | WOS path |
|---|---|---|
| RuntimeObservation | `authoringOnly` | — |
| Cluster | `authoringOnly` | — (triggers ChangeImpactReport per `change-impact.md`) |
| ReplayResult | `authoringOnly` | — |
| Promoted Scenario | `mapsToWos` | `wos-tooling.scenarios[*]` (per `scenario-authoring.md`) |
| Promoted PolicyObject claim | (depends on the kind it becomes) | (per `policy-object-model.md` mapping table) |

## Examples

### Example 1: Bottleneck cluster triggering ChangeImpactReport

A SNAP redetermination workflow has been published for 6 months. The runtime emits 47 observations in 14 days where caseworkers flag "applicant did not respond within 30 days" but proceed to deny without the workflow's required notice-of-missing-response step (which the workflow has, but caseworkers are bypassing because the notice template fails to send for non-English-speaking applicants).

1. Cluster detector identifies 47 observations sharing `unmodeled-divergence` at the missing-response step.
2. Severity = `acknowledge-required` (workspace policy threshold: 25+ in 14 days).
3. ChangeImpactReport produced with `triggerKind = runtime-observation-cluster`; affected PolicyObjects: NoticeRequirement (the missing-response notice). Suggested action: reviewer investigates, identifies notice-template failure for non-English speakers, opens an ExtractedClaim with `originClass = runtime-observed` that becomes a new NoticeRequirement covering the language gap.
4. The original NoticeRequirement and the new one both progress through the review pipeline; the new one ships in the next workflow version.

### Example 2: Replay of a manual-override observation

A caseworker manually overrides a denial decision for a borderline-eligible applicant. The runtime emits an observation including the override.

1. Studio replays the observation against the currently-published WorkflowIntent.
2. ReplayResult shows: at the decision element, the designed path was "deny"; the observed path was "approve via manual-override."
3. The override is permitted (the workflow has a `manual-override` element), so the divergence is noted but is NOT a violation.
4. Reviewer reviewing the cluster of similar overrides decides whether to: (a) tighten the DecisionTable to make these cases auto-approve, or (b) keep the override path and add a notice-explaining-override ExplanationRequirement, or (c) accept-as-known-pattern and do nothing.

### Example 3: Phase-4 deferral

In the current Stage-2 spec set, this spec is **seam-only**. No runtime emits these observations yet. No cluster detector exists. No replay engine exists. The wire format and contracts above are the **forward-compatible promise** — when Phase 4 begins, implementers write the importer/cluster-detector/replayer to satisfy these contracts, and Studio's existing wiring (in `change-impact.md`, `authoring-provenance.md`, `scenario-authoring.md`, `policy-object-model.md`) just lights up.

## Open issues

- **Cluster heuristic catalog.** Phase 4 will need a starter catalog of cluster patterns (`stuck-at-step`, `unmodeled-divergence`, `bottleneck-spike`, etc.). The catalog is a research project blending production traffic patterns with reviewer-driven taxonomy.
- **Privacy de-identification before ingest.** When the runtime emits PII-bearing observations, what's the de-identification contract? Today this is "redact per DPV class + workspace authority grants"; the actual transformation pipeline (k-anonymity, differential privacy, structural suppression) is Phase 4.
- **Cross-workspace observation sharing.** A federal-level workspace might benefit from state-workspace observations; cross-workspace (federation) is deferred per §1.34.
- **Real-time vs. batch tradeoffs.** Three ingest paths are reserved; which is the default? Workspace-policy-configurable; default sketched in `workspace.md`.
- **Auto-detection of promoted-Scenario regressions.** When a promoted Scenario is added and then a future workflow version regresses it, the regression should surface as a tier-S5 ValidationFinding. The cross-spec coupling exists; the rule is sketched in `readiness-validation.md` but not yet exercised.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.17 (entity placeholder), §1.26 (this seam).
- Closes dangling references in: [`change-impact.md`](change-impact.md) (`triggerKind = runtime-observation-cluster`), [`authoring-provenance.md`](authoring-provenance.md) (`originClass = runtime-observed`).
- Downstream: [`scenario-authoring.md`](scenario-authoring.md) (runtime-observation-replay scenario type), [`policy-object-model.md`](policy-object-model.md) (runtime-observed PolicyObject candidates), [`binding-and-integration.md`](binding-and-integration.md) (EventBinding-based subscription path).
- Parent repo: [`../../crates/wos-runtime/`](../../crates/wos-runtime/) for the DurableRuntime adapter that runtime emitters and replay engines target.
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
