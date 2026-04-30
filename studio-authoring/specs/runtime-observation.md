# Studio Spec: Runtime Observation

**Status:** **future-track** — draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap)); the underlying capability is **Phase 4** in the Product Roadmap and is NOT in scope for any current MVP. This spec exists to anchor the data model so earlier-phase work does not paint Phase 4 into a corner.
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.17 RuntimeObservation.
**PRD anchor:** [`../VISION.md`](../VISION.md) §9.11 (Runtime Observation and Iteration), §16 Phase-4 (entire phase).
**Depends on:** [`change-impact.md`](change-impact.md), [`authoring-provenance.md`](authoring-provenance.md), [`scenario-authoring.md`](scenario-authoring.md).

## Scope

Runtime Observation is the **closed-loop** spec: it brings observed runtime behavior back into Studio as evidence for workflow iteration. The Phase-4 success question (PRD §16) is "Where does actual practice diverge from the approved workflow?"

Initial scope is **imported traces, not live orchestration** (PRD §9.11). Studio does not run cases; it imports case histories from external systems (state agency case-management systems, federal benefits adjudication systems, integration audit logs) and compares them against the designed `$wosWorkflow` to identify drift, bottlenecks, manual overrides, and stuck cases.

This spec defines:

- the RuntimeObservation record shape and the normalized-event format imports must produce;
- the **designed-vs-observed comparison** model;
- the **drift / bottleneck / override detection** taxonomy;
- the **scenario-promotion** path (an observed real case → a Studio Scenario for regression);
- the **improvement-proposal** path (clustered observations → ChangeImpactReport trigger via [`change-impact.md`](change-impact.md));
- composition with the rest of Studio.

## Out of scope

- Live runtime orchestration (e.g., Studio actively running cases). This is permanently out of scope; Studio is an authoring/review/observation layer, not a workflow engine.
- Direct integration with specific workflow engines (Restate, Temporal, Camunda, Step Functions). Engine-specific adapters live downstream.
- Process mining beyond the four detection categories (PRD §9.11 deferred capabilities).
- Bidirectional updates from Studio into the runtime (Phase 4 is read-only-from-runtime).

## Terminology

- **RuntimeObservation** — the durable Studio artifact representing one imported case's history (CM §1.17).
- **Normalized event** — an imported runtime event mapped into a Studio-comparable structure (`{kind, atStep, atTime, actor, payload}`).
- **Designed trace** — what the workflow says should have happened (derived from the `$wosWorkflow` by walking the lifecycle with the observed inputs).
- **Observed trace** — what actually happened (constructed from imported events).
- **Divergence** — a typed mismatch between designed and observed.
- **Cluster** — a set of RuntimeObservations sharing a common divergence pattern.
- **Promotion** — converting an observation into a Studio Scenario for use as regression test material.
- **Improvement proposal** — a reviewer-authored, evidence-backed candidate workflow change.

## Data model

### `RuntimeObservation` (CM §1.17, extended)

```text
RuntimeObservation {
  id, externalCaseId, externalCaseSystem,
  importedFrom (adapter id + import job id),
  workflowVersion (the PublishedWorkflowPackage version this case followed, if known),
  caseStartedAt, caseEndedAt?,
  rawEventsRef (raw imported events, for audit),
  normalizedEvents[],   // NormalizedEvent
  observedTrace,        // ObservedTrace
  designedTrace,        // DesignedTrace (derived at comparison time)
  divergences[],        // Divergence
  classifications[],    // {kind: bottleneck|drift|manual-override|stuck|exception, evidence}
  reviewState,          // imported | normalized | compared | reviewed | promoted
  workspaceId, version
}

NormalizedEvent {
  kind,                 // intake | decision | notice | task-assigned | task-completed | timer-fired | agent-invoked | manual-override | system-failure | …
  atStep,               // workflow step ref (resolved if possible; null if unmappable)
  atTime,
  actor,                // applicant | staff:role | system | agent
  payload,              // event-specific structured data
  externalEventId       // back-reference to the source system's event
}

ObservedTrace {
  path[],
  decisions[],
  notices[],
  tasks[],
  timers[],
  terminalOutcome?,     // if case ended; null if still active
  manualOverrides[]
}

Divergence {
  kind,                 // step-skipped | step-extra | decision-mismatch | notice-missing | timer-overshoot | actor-substituted | exception-undeclared
  designedRef,
  observedRef,
  description,
  severity (low|medium|high)
}

ClusterMembership {
  clusterId, observationId, sharedPattern
}
```

### Cluster

```text
RuntimeObservationCluster {
  id, patternKind,       // common-divergence | shared-stuck-step | shared-bottleneck | …
  patternDescription, members[], detectedAt,
  proposedChangeImpactReportRef? (when promoted to a CI trigger)
}
```

## Lifecycle

A RuntimeObservation:

```text
imported → normalized → compared → reviewed → { promoted | acknowledged | archived }
```

- `imported`: raw events arrived; not yet normalized.
- `normalized`: events mapped to NormalizedEvent shape; ObservedTrace constructed.
- `compared`: DesignedTrace derived; divergences computed.
- `reviewed`: a reviewer has examined the comparison and classified the case.
- `promoted`: case promoted to a Studio Scenario (typically when the case represents an interesting edge case worth keeping as regression material).
- `acknowledged`: reviewer noted the case; no follow-up action required.
- `archived`: case retired from active review (still queryable; no longer surfaces in dashboards).

A Cluster:

```text
detected → reviewed → { proposedAsImprovement | dismissed | merged }
```

- `detected`: clustering algorithm found a pattern.
- `reviewed`: a reviewer has examined the cluster.
- `proposedAsImprovement`: the cluster has been used to author an improvement proposal (and possibly trigger a ChangeImpactReport per [`change-impact.md`](change-impact.md)).
- `dismissed`: reviewer determined the cluster is not actionable.
- `merged`: combined with another cluster sharing the underlying cause.

## Normative Contract

Every MUST in this spec is **future-track** — none is enforceable today. Tracking IDs of the form `SA-MUST-ro-NNN`.

### Import and normalization

- **`SA-MUST-ro-001`** — Every RuntimeObservation MUST identify its source system (`externalCaseSystem`), the import job that produced it (`importedFrom`), and (if known) the published workflow version the case followed (`workflowVersion`). *(future-track: no import path implemented.)*
- **`SA-MUST-ro-002`** — Imported events MUST preserve raw form (`rawEventsRef`) for audit. Normalization MUST be a pure function from raw → NormalizedEvent; re-running the normalizer on the same raw events MUST produce identical NormalizedEvents. *(future-track.)*
- **`SA-MUST-ro-003`** — Normalization MUST identify, for each event, the `atStep` reference into the workflow when possible. Events that cannot be mapped to a workflow step MUST be retained as `unmappableEvent` and surface in the divergence list as `step-extra` (something happened that the workflow does not model). *(future-track.)*
- **`SA-MUST-ro-004`** — Every NormalizedEvent MUST carry an `actor` classification (applicant | staff:role | system | agent). Unknown-actor events are imported but flagged for reviewer attention. *(future-track.)*

### Comparison

- **`SA-MUST-ro-010`** — DesignedTrace MUST be derived deterministically from the observed inputs by walking the `$wosWorkflow` lifecycle. *(future-track.)*
- **`SA-MUST-ro-011`** — Divergences MUST be typed (per the Divergence enum: `step-skipped`, `step-extra`, `decision-mismatch`, `notice-missing`, `timer-overshoot`, `actor-substituted`, `exception-undeclared`) and carry plain-language descriptions. *(future-track.)*
- **`SA-MUST-ro-012`** — Comparison MUST be deterministic given `{observed events, $wosWorkflow version, comparator version}`. Re-running comparison produces identical divergences. *(future-track.)*
- **`SA-MUST-ro-013`** — A RuntimeObservation MUST NOT advance from `compared → reviewed` automatically; reviewer attention is required. *(future-track; cross-cutting with PRD §10 AI behavior.)*

### Detection categories

- **`SA-MUST-ro-020`** — Bottleneck detection MUST identify steps where observed-step-duration significantly exceeds designed-step-duration across a meaningful sample. The "significant" and "meaningful" thresholds are workspace-policy. *(future-track.)*
- **`SA-MUST-ro-021`** — Drift detection MUST identify cases that consistently diverge from designed paths in a structured way (e.g., 30% of denied cases route through an unmodeled pre-denial conversation step). *(future-track.)*
- **`SA-MUST-ro-022`** — Manual-override detection MUST identify cases where a staff actor explicitly overrode a default decision; the override frequency MUST be aggregable to identify whether the override is "rare-by-policy" or "the default is wrong." *(future-track.)*
- **`SA-MUST-ro-023`** — Stuck-case detection MUST identify cases that have been in a state beyond a reasonable threshold (workspace-policy). *(future-track.)*

### Cluster detection and promotion

- **`SA-MUST-ro-030`** — Cluster detection MUST be deterministic given the same set of RuntimeObservations and the same clustering version. *(future-track.)*
- **`SA-MUST-ro-031`** — A cluster MUST NOT auto-trigger a ChangeImpactReport. A reviewer-driven `proposedAsImprovement` transition (per Lifecycle) is the path. *(future-track; PRD §10 AI behavior — copilot proposes, humans approve.)*
- **`SA-MUST-ro-032`** — When a reviewer promotes a cluster to an improvement proposal, the resulting ChangeImpactReport MUST carry `triggerKind = runtime-observation-cluster` and `triggerRef = clusterId` (per [`change-impact.md`](change-impact.md) §Triggers). *(future-track.)*

### Scenario promotion

- **`SA-MUST-ro-040`** — When a reviewer promotes a single RuntimeObservation to a Studio Scenario, the resulting Scenario MUST: (a) use the case's inputs as `inputs`; (b) use the normalized events as the `eventSequence` (or a redacted equivalent if the raw events contain PII); (c) declare an `expectedTrace` derived from the observed trace OR from the designed trace, depending on whether the reviewer is capturing "what happened" or "what should happen"; (d) link the underlying PolicyObjects via `linkedPolicyObjects[]`; (e) declare an appropriate `scenarioType` from the canonical 12 (most often `manual-override`, `system-failure-fallback`, `agent-fallback`, or a divergence-mirror of the original). *(future-track; cross-cutting with [`scenario-authoring.md`](scenario-authoring.md).)*
- **`SA-MUST-ro-041`** — Promoted Scenarios MUST carry provenance back to the originating RuntimeObservation in their `provenance` field. *(future-track; cross-cutting with [`authoring-provenance.md`](authoring-provenance.md).)*
- **`SA-MUST-ro-042`** — When a promoted Scenario uses real applicant data, the implementation MUST scrub PII per workspace policy before the Scenario can be saved as a durable artifact. *(future-track; PRD §13 Security and permissions.)*

### Privacy and PII

- **`SA-MUST-ro-050`** — RuntimeObservations MAY contain PII; their access controls MUST be at least as strict as the host workspace's most-sensitive SourceDocument. *(future-track.)*
- **`SA-MUST-ro-051`** — Promotion to Scenario or sharing across workspaces MUST trigger PII scrubbing per workspace policy. *(future-track.)*
- **`SA-MUST-ro-052`** — Aggregations across observations (clusters, bottleneck statistics) MUST NOT expose individual case identifiers in summary form. *(future-track.)*

## Composition

### Attachment point

Runtime Observation attaches at the **workspace** layer. Imported observations are workspace-scoped. Cross-workspace pattern detection (e.g., the same drift appearing across multiple state agencies' workspaces) is Phase-4 work that depends on Phase-3 cross-workspace concepts ([`change-impact.md`](change-impact.md) Open Issues).

The observation engine **reads** the published `$wosWorkflow` to derive DesignedTraces. It **writes** RuntimeObservations and Clusters. It does **NOT** mutate any other Studio object directly — improvement proposals are reviewer-driven (PRD §10 AI behavior).

### Precedence

When a RuntimeObservation arrives that contradicts a Studio Scenario's `expectedTerminalOutcome` for a similar input, the observation does NOT override the scenario. The scenario remains the test; the observation is *evidence* that the scenario or the workflow may need updating. Reviewers decide.

When two clusters share members (an observation is part of more than one detected pattern), both clusters retain that observation; the reviewer decides which cluster's `proposedAsImprovement` (if any) drives the change.

### Conflict handling

Observations with damaged or incomplete event traces (e.g., the source system dropped events) are imported with explicit gaps marked; comparison is performed best-effort. The implementation MUST NOT silently fill gaps or fabricate events.

### Versioning / migration

- Adding a new Divergence kind, classification kind, or normalization-event kind is **schema-breaking**.
- The clustering algorithm version is recorded per cluster; algorithm upgrades produce new clusters rather than retroactively re-classifying old ones.
- Adapter changes (a new source system being supported) are non-breaking schema-wise but require workspace administrator opt-in.

## Conformance

### Schema validation (Stage 3)

Planned schema gates (all future-track, conditional on Phase-4 work):

- RuntimeObservation required fields and lifecycle enum.
- NormalizedEvent shape and `kind` enum.
- Divergence kind enum.
- ClusterMembership shape.

### Lint rules (Stage 4)

No tier-S1–S6 readiness rules from this spec are active in Phase 1–3. Phase-4 will add:

- `RO-LINT-001` — every promoted Scenario carries provenance to its originating RuntimeObservation (`SA-MUST-ro-041`).
- `RO-LINT-002` — every cluster `proposedAsImprovement` produces a ChangeImpactReport (`SA-MUST-ro-032`).
- `RO-LINT-003` — PII present in raw observations is scrubbed before promotion (`SA-MUST-ro-051`).

### Runtime conformance fixtures (Stage 4–6)

Future-track:

- Deterministic normalization: same raw events ⇒ same NormalizedEvents.
- Deterministic comparison: same `$wosWorkflow` + observed events ⇒ same divergences.
- Cluster determinism for the same inputs and clustering algorithm version.
- Promotion of an observation to a Scenario preserves provenance and scrubs PII.

### Current limitations

- The entire spec is **future-track**. No part of it is enforceable today.
- No reference adapter exists for any source system.
- The clustering algorithm is unspecified beyond "deterministic given inputs."
- The PII-scrubbing policy is workspace-policy-driven; no default is provided.
- The improvement-proposal authoring path is sketched but not pinned; it cross-couples with [`change-impact.md`](change-impact.md) Open Issues.

This spec exists primarily to **anchor the data model** so that earlier-phase work (Phase 1–3) does not paint Phase 4 into a corner. The CM §1.17 entity, the `runtime-observed` originClass in [`authoring-provenance.md`](authoring-provenance.md), the `runtime-observation-cluster` trigger in [`change-impact.md`](change-impact.md), and the optional Phase-4 promotion path in [`scenario-authoring.md`](scenario-authoring.md) all reference back here.

## WOS mappings

RuntimeObservations and Clusters are **`authoringOnly`** as a whole — runtime observation is a Studio-internal concern. Clusters that are promoted to ChangeImpactReports drive new published workflow versions, but the observations themselves never appear in `$wosWorkflow`.

| Studio object | Mapping state | WOS path |
|---|---|---|
| RuntimeObservation | `authoringOnly` | — (workspace state) |
| NormalizedEvent | `authoringOnly` | — |
| ObservedTrace, DesignedTrace, Divergence | `authoringOnly` | — |
| Cluster | `authoringOnly` | — |
| Promoted Scenario | (per [`scenario-authoring.md`](scenario-authoring.md)) | conformance trace projection in `wos-tooling.schema.json` |
| Improvement-proposal-derived ChangeImpactReport | (per [`change-impact.md`](change-impact.md)) | `authoringOnly` (release notes project compactly) |

The bridge between observed runtime and the published WOS artifact is **indirect**: observations evidence the need for a workflow update; reviewers author the update; a new workflow version publishes; the cycle continues.

## Examples

The following examples are illustrative; nothing in them is implemented yet.

### Example 1: Bottleneck cluster drives a workflow simplification

A state agency's SNAP-redetermination workflow has been in production for 18 months. 1,200 cases imported via an integration adapter. After comparison:

1. Cluster detected: `patternKind = bottleneck`. 47% of cases spent more than 14 days in the `evidence-verification` step (designed-step-duration: 5 days). Members: 564 observations.
2. Reviewer reviews; classifies the cluster as `proposedAsImprovement`.
3. Reviewer authors an improvement proposal: "Add a parallel auto-verification path for cases where federal-data-broker check returns clean results within 24h. Bypass manual evidence verification when broker confidence > 0.95."
4. ChangeImpactReport produced with `triggerKind = runtime-observation-cluster`.
5. Reviewer drafts a workflow change: a new system-check step + a new TransitionMapping. The change passes through normal review/approval; new workflow version `v2.0` publishes.
6. Subsequent observations show the bottleneck cleared — 80% of cases now route through the auto-verification path within 24h.

### Example 2: Manual override frequency reveals an incorrect default

Observations reveal that staff members are manually overriding the default `pending-info → denied` transition in 35% of cases — adding a courtesy outreach step that is not in the workflow.

1. Cluster detected: `patternKind = manual-override`. 35% of cases include a `manual-override` event at a specific step.
2. Reviewer concludes: "The 'courtesy outreach' is local practice that should be modeled. Authoring it explicitly as an `ExceptionRule` will eliminate the override and document what staff actually do."
3. Improvement proposal authored. ChangeImpactReport produced.
4. The new workflow version explicitly models courtesy outreach as an ExceptionRule with `originClass = local-practice` and a citation to the workspace's operational policy.

### Example 3: Stuck-case detection promotes a scenario

A reviewer notices a single observation: applicant submitted in January, the case has been in `awaiting-evidence` for 4 months. The reviewer promotes this observation to a Studio Scenario:

```text
Scenario: "Applicant submits but never returns evidence; deadline passes"
scenarioType: deadline-missed
linkedPolicyObjects: [evidence-deadline-Deadline, default-denial-Outcome, denial-NoticeRequirement]
inputs (PII-scrubbed): { householdSize: <redacted>, monthlyIncome: <redacted>, ... }
eventSequence: [
  { kind: applicantSubmits, at: T+0 },
  // No evidence submission
  { kind: timerFired, at: T+30d, timer: evidence-deadline },
  ... (no further events as observed)
]
expectedTrace: { ... per the workflow's intended behavior on timer fire ... }
```

The Scenario carries provenance back to the originating RuntimeObservation. It becomes a regression test in the workspace's scenario suite.

## Open issues

This spec has the **most extensive open-issues set** because it is future-track. Many decisions are deferred:

- **Adapter model.** No source-system adapter is specified. Each integration (state CMS, federal benefits system, agency log) needs its own adapter; the adapter contract (input format, normalization mapping) is open.
- **Clustering algorithm.** "Deterministic given inputs" is the bar; the algorithm itself is unspecified. Likely candidates: process-mining-derived (alpha-miner, heuristic miner), embedding-based, graph-based. Stage-4 (or Phase-4) work decides.
- **PII scrubbing policy.** Default scrubbing rules, configurability, and audit of scrubbing operations are unspecified.
- **Cross-workspace pattern detection.** Mentioned in Composition; not specified.
- **Live integration vs. batch import.** The spec assumes batch import; live event streaming is not addressed.
- **Quantitative thresholds.** Bottleneck "significantly exceeds," cluster "meaningful sample," stuck-case "reasonable threshold" — all workspace-policy. Recommended defaults are unspecified.
- **Active-case impact reporting.** PRD §9.8 chain ends at "potentially affected active cases" — observation-driven detection of which active cases were impacted by a workflow definition gap is unspecified.
- **Promotion-to-scenario UX.** The spec says reviewers can promote; the UX path is unspecified.
- **Runtime observation as a basis for `originClass = runtime-observed` in authoring provenance.** The provenance integration point is sketched in [`authoring-provenance.md`](authoring-provenance.md); the precise contract is unspecified beyond the originClass enum value.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.17.
- PRD: [`../VISION.md`](../VISION.md) §9.11, §16 Phase-4 (entire phase).
- Upstream (data flows in): observations come from external runtime systems; not a Studio-internal upstream.
- Downstream (drives action): [`change-impact.md`](change-impact.md) (cluster → trigger), [`scenario-authoring.md`](scenario-authoring.md) (observation → scenario), [`authoring-provenance.md`](authoring-provenance.md) (`runtime-observed` originClass).
- WOS: published `$wosWorkflow` artifact (read-only; supplies DesignedTrace).
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
