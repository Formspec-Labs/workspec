# WOS Public API Governance

**Status:** Draft (ADR 0082 D-15 step 5 — landed 2026-05-05)
**Schema:** [`api/governance.schema.json`](../../schemas/api/governance.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/governance/v1`
**Common definitions:** [`api/_common.schema.json`](../../schemas/api/_common.schema.json) (`https://schemas.formspec.io/wos-api/_common/v1`) — canonical `ActorRef` and `WosResourceUrn` home (ADR 0082 D-4, D-9, D-14).
**Authority:** [ADR 0082 — Stack Public REST API Contract and Schema Discipline](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) (D-3 subresource decomposition; D-4 URN identifiers; D-7 cursor pagination; D-12 closed taxonomies; D-14 no redefining; D-15 step 5)
**Related ADRs:** [ADR 0064 — Agent as first-class `ActorKind`; `AgentInvoker` port](../../thoughts/adr/0064-agent-actor-kind-and-invoker-port.md); [ADR 0068 — Stack Tenant and Scope Composition](../../../thoughts/adr/0068-stack-tenant-and-scope-composition.md) (Proposed)
**Spec authority:** [`work-spec/specs/governance/workflow-governance.md`](../governance/workflow-governance.md) — §S2 (agents block content), §S6 (structured audit + escalation), §S10.3/§S10.4 (SLA + escalation chains), §S11 (delegation), §S12 (typed hold policies), §S13 (temporal parameter resolution).

## Purpose

This domain authors the cross-case authoritative shapes for WOS governance — the records that span case instances and outlive any one case. Four resource families ratify under this spec:

- **`AgentView`** — agent declaration view (model identity, autonomy level, deontic constraints summary, confidence floor, drift status, capabilities). Per ADR 0064, agents are first-class actors; this view projects the workflow `agents[]` block on top of the underlying `Actor` resource.
- **`Delegation`** — full cross-case delegation record (delegator URN, delegatee URN, authority scope, period, conditions, audit). Distinct from `instance.schema.json#/$defs/DelegationEntry`, which is the case-scoped summary projection.
- **`PolicyVersion`** — pinned version of a workflow's policy parameters with effective period, parameter hash, and source citation.
- **`EscalationEvent`** — wire shape for the `escalation` audit-event class (governance §S6, §S10.3, §S12.4).

Greenfield projection per ADR 0082 D-15: kernel governance `$defs` (`Delegation`, `DelegationScope`, `DeonticConstraint`, `EscalationStep`, `AgentDeclaration`) are prior art; this contract is the projection consumers code against. The `case-portal/src/ports/types.ts` and `workspec-server/src/domain/` shapes are not preserved.

## Relationship to per-case governance projection

The per-case projection lives on `instance.schema.json` (`CaseInstanceGovernance`, `DelegationEntry`, `ReviewState`, `HoldEntry`). That subresource summarizes the in-effect governance state for a single case instance — minimum fields necessary for case-portal rendering — and is reachable through `GET /api/v1/instances/{id}/governance` and the `?include=governance` aggregation seam (ADR 0082 D-3). This domain authors the cross-case authoritative shapes those summaries project from:

| Per-case (instance.schema.json) | Cross-case (this domain) | Relationship |
|---|---|---|
| `DelegationEntry` (case-scoped projection) | `Delegation` (cross-case authoritative) | `DelegationEntry` summarizes the fields a case view needs (`id`, `delegator`, `delegatee` (kernel calls it `delegate`), `authority`, `effectiveDate`, `expirationDate`, `revoked`); `Delegation` carries the full record (legal instrument, sub-delegation flag, revocation audit). |
| `ReviewState` (closed posture enum) | (no cross-case shape) | Review posture is per-case by construction; it has no cross-case authoritative analog. |
| `HoldEntry` (case-scoped hold projection) | (no cross-case shape) | Holds are case-scoped by construction; the hold policy declarations live on the workflow's `governance.holds` block, not as cross-case records. |
| (no per-case projection) | `AgentView` | Agents are workflow-scoped, not case-scoped — the same agent declaration governs every case under the workflow. The case view does not surface agents directly. |
| (no per-case projection) | `PolicyVersion` | Policy versions are workflow-scoped; the version effective for a case is resolved at runtime from the case's resolution-date field (governance §S13). |
| (no per-case projection) | `EscalationEvent` | Escalation events are emitted into the audit timeline; they are surfaced via the audit domain (ADR 0082 D-15 audit, deferred), not the case-instance governance subresource. |

The two-shape decomposition matches ADR 0082 D-3 — pagination scales per resource, and the per-case view stays bounded for case-portal rendering while consumers needing the cross-case authoritative record (audit, governance dashboards, AI-oversight) reach this domain directly.

## Resource Shape

### `AgentView`

Carries `id` (URN), `actorRef` (the underlying `Actor` URN), `workflowUrl`/`workflowVersion` (governing declaration), `modelIdentifier`/`modelVersion` (deployed model), `autonomyLevel` (closed `AutonomyLevel`), `invokerKind` REQUIRED (closed-with-vendor-extension `anthropic | claudeAgentSdk | mcp | a2a | http | stub` per ADR 0064 §2.2-§2.3), `lifecycleState` REQUIRED (closed-with-vendor-extension `active | degraded | suspended | retired` per advanced-governance.md §7.1), `agentType` (closed-with-vendor-extension `deterministic | statistical | generative` per ai-integration.md §3.1; structural discriminator gating downstream calibration / validation requirements per §3.2), `deonticConstraints` (`DeonticConstraintSummary[]`), `confidenceFloor`, `confidenceDecay` (closed `ConfidenceDecaySpec` — half-life and floor projection per §7.5), `driftStatus` (closed `AgentDriftStatus`), `cascadingInvocations` (closed `CascadingInvocationsSpec` per §3.7.3 — present iff the agent is declared `autonomous` and authorised to invoke other autonomous agents), `volumeConstraints` (closed `VolumeConstraintsSpec` per §11.1), `shadowMode` (closed `ShadowModeSpec` per advanced-governance.md:370-385), `circuitBreaker` (closed `CircuitBreakerSpec` per advanced-governance.md:387-400), `activeSessions` (`MultiStepSessionState[]` per advanced-governance.md §5 multi-step sessions), `capabilities`, `fallbackChain`. The underlying actor identity is referenced via `actorRef` — display name, principal class, and lifecycle status live on the `Actor` resource; this view carries only the agent-specific operational state the kernel `agents` block holds.

`invokerKind` is REQUIRED so operability tooling (telemetry, capability planning, multi-agent dispatch) can branch on the underlying adapter without inspecting the workflow document — this is the cross-spec observability commitment per ADR 0064. `lifecycleState` is REQUIRED so portals can render the live four-state machine (advanced-governance.md §7.1) without reconstructing from the per-event `agentStateTransition` provenance stream; the runtime event literal records transitions, this declarative field carries the current state.

`MultiStepSessionState` (declared in this domain, projected onto `AgentView.activeSessions`) carries `sessionId`, `currentStep`, `totalSteps`, `actorRef`, `startedAt`, optional `instanceId`, optional `atCheckpoint`. Lets reviewers query "which agent is parked awaiting human review at a multi-step checkpoint?" via `activeSessions[].atCheckpoint == true` (advanced-governance.md §5.4 cumulative-confidence pause). Closes the residual gap from the original advanced-sidecars audit fix #2.

Per ADR 0064, the agent's principal class on the underlying `Actor` is `service-account` or `workload` — never `human`.

`shadowMode` and `circuitBreaker` are paired with cross-spec consumers: shadow-mode divergence surfaces as the `agent-shadow-mode-divergence` report type on `reports.schema.json`; circuit-breaker state changes are mirrored by Agent B's `circuitBreakerTripped` and `circuitBreakerReset` Facts record kinds on `provenance.schema.json`.

Operational provenance obligations:

- Shadow-mode divergence means the shadow agent's output crossed the configured divergence threshold against its declared baseline. The output remains non-authoritative; the `shadowModeDivergence` Facts record is audit/report evidence only.
- Circuit breakers emit `circuitBreakerTripped` when observed failure rate or guarded error count opens the breaker, and `circuitBreakerReset` when governed cooldown/recovery closes or half-opens it.
- Drift monitoring emits `driftAlert` when an alert threshold crosses and records the resulting `AgentDriftStatus`.
- Legal holds emit `legalHoldPlaced` and `legalHoldReleased` for hold lifecycle changes. Any destruction attempt while a legal hold applies MUST emit `legalHoldDestructionRejected` and leave the protected material intact.
- Quorum reductions are explicit policy transitions. Governance §S4.9 forbids silently lowering `requiredCount`; any reduction MUST produce a Facts record carrying `quorumReduction`.

### `Delegation`

Carries `id` (URN), `delegator`/`delegatee` (`ActorRef` URNs), `authority` (closed `DelegationAuthority`), `scope` (`DelegationScope` — `impactLevels`, `caseTypes`, `maxDollarThreshold`, `conditions`), `legalInstrument`, `effectiveDate`, `expirationDate` (or sentinel `"never"` per ADR 0082 D-10), `revocable`, `revokedAt`/`revokedBy`/`revokedReason`, `allowsSubDelegation`, `quorum?: QuorumRequirement` (governance §S4.9 N-of-M; workflow-governance.md:667-677 — present iff this delegation requires an N-of-M assembly), `independenceVerified?: IndependenceVerification` (governance §3.5 item 1; workflow-governance.md:148-152 — present iff this delegation routes appeal review and the runtime has verified the delegatee is independent of the original determination), `createdAt`. The kernel `Delegation.delegate` field is renamed `delegatee` at the API layer for symmetry with `delegator`.

`QuorumRequirement` carries `requiredCount: integer ≥ 1` (N), `totalEligible: integer ≥ 1` (M, MUST be ≥ N), `distinctPrincipalRequired: boolean` (collusion-resistance MUST per workflow-governance.md:673), and an optional array of `DeonticConstraintSummary` further bounding the deontic envelope. The §S4.9 MUST against silent reduction is enforceable from this shape — a regulator querying "show me every adverse decision under a 3-of-5 quorum and prove distinct-principal" can read it directly.

`IndependenceVerification` carries `verifiedActorRef: ActorRef`, `verificationMethod` (closed-with-vendor-extension — `separation-of-duties-policy | delegation-graph-walk | manual-attestation | organizational-unit-disjoint | supervisor-attestation`), `verifiedAt: timestamp`, optional `appealCaseId: WosResourceUrn`. The §3.5 MUST that an appeal MUST be reviewed by a human adjudicator independent of the original determination (workflow-governance.md:150) is enforceable from this shape — a regulator or appeals-routing consumer queries `Delegation.independenceVerified.verificationMethod` directly.

`DelegationCreateRequest` accepts the optional `quorum` field; the server validates that `delegatee` references identify a quorum pool of size ≥ `quorum.totalEligible`.

### `PolicyVersion`

Carries `policyUrl` (workflow URL), `version` (stable label, SemVer recommended), `effectiveFrom`/`effectiveUntil` (with `"never"` sentinel for the current version), `parameters` (`PolicyParameterSummary[]` — `key`/`value`/`resolutionDateRef`), `parameterHash` (`sha256:` over RFC 8785 JCS canonical JSON of `parameters`), `sourceCitation`, `createdAt`/`createdBy`. The hash lets clients detect silent parameter drift between identical-looking version labels and lets audit compare against the Trellis-anchored value.

### `EscalationEvent`

Carries `id` (provenance-record URN), `instanceId` (case URN), `taskId?` (task URN when task-scoped), `reason` (closed `EscalationReason`), `level` (integer ≥ 1 — kernel `EscalationStep.level`), `stepId?` (kernel `EscalationStep.id`), `escalatedAt`, `escalatedBy`/`escalatedTo` (actor URNs), `rationale` (when `reason == manual`), `gracePeriod` (ISO 8601 duration, kernel `EscalationStep.gracePeriod`).

## Endpoints

```
GET   /api/v1/governance/agents                                         -> AgentPage
GET   /api/v1/governance/agents/{urn}                                   -> AgentView
GET   /api/v1/governance/delegations                                    -> DelegationPage
GET   /api/v1/governance/delegations/{urn}                              -> Delegation
POST  /api/v1/governance/delegations                                    -> Delegation             (Idempotency-Key REQUIRED)
POST  /api/v1/governance/delegations/{urn}/revoke                       -> Delegation             (Idempotency-Key REQUIRED)
GET   /api/v1/governance/policies/{policyUrl}/versions                  -> PolicyVersionPage
GET   /api/v1/governance/policies/{policyUrl}/versions/{version}        -> PolicyVersion
```

`{policyUrl}` is the URL-encoded governing workflow URL.

Per-case scoped delegations remain accessible via the case-instance subresource per ADR 0082 D-3:

```
GET   /api/v1/instances/{id}/governance                                 -> CaseInstanceGovernance  (instance.schema.json)
GET   /api/v1/instances/{id}?include=governance                         -> CaseInstanceWithIncludes (instance.schema.json)
```

`GET /api/v1/governance/agents` accepts `AgentListOptions`: `workflowUrl?`, `autonomyLevel?`, `driftStatus?`, `cursor?`, `limit?`. Returns `AgentPage` (cursor envelope per `pagination.schema.json`, ADR 0082 D-7). Default ordering is `id` ascending.

`GET /api/v1/governance/delegations` accepts `DelegationListOptions`: `delegatorUrn?`, `delegateeUrn?`, `authority?`, `active?`, `workflowUrl?`, `cursor?`, `limit?`. Returns `DelegationPage`. Default ordering is `createdAt` ascending. The `active` flag combines effective-date, expiration-date (including the `"never"` sentinel), and revocation state into a single derived predicate; servers MAY treat `workflowUrl` as best-effort filtering when scope predicates require runtime evaluation.

`POST /api/v1/governance/delegations` accepts `DelegationCreateRequest { delegator, delegatee, authority, scope, legalInstrument?, effectiveDate?, expirationDate?, revocable?, allowsSubDelegation? }`. The `Idempotency-Key` HTTP header is REQUIRED per ADR 0082 D-16; a repeat request within the retention window returns the original `Delegation` unchanged. Server assigns the URN, `createdAt`, and applies kernel-default `revocable: true` / `allowsSubDelegation: false` when the request omits those fields.

`POST /api/v1/governance/delegations/{urn}/revoke` accepts `DelegationRevokeRequest { reason }` and the `Idempotency-Key` header. Server records `revokedAt`, `revokedBy` (the calling actor), and the supplied `reason`. Revoked delegations remain visible for audit but do not authorize new actions; repeated revoke requests on an already-revoked delegation return the original revoked-state representation.

`GET /api/v1/governance/policies/{policyUrl}/versions` accepts `PolicyVersionListOptions`: `active?`, `cursor?`, `limit?`. Returns `PolicyVersionPage`. Default ordering is `effectiveFrom` descending (newest first) to match typical version-history rendering.

## Identifier Scheme

URNs follow ADR 0082 D-4: `urn:wos:<entity-type>:<workflow-or-scope-id>:<date>:<short-hash>`. This domain uses the `agent`, `delegation`, and `provenance-record` entity-type literals (the last for `EscalationEvent`, since escalation events are tier-typed provenance records per governance §S6). All three literals are already part of the closed taxonomy ratified at `_common.schema.json#/$defs/WosResourceUrn`.

`PolicyVersion` is identified by the `(policyUrl, version)` pair, not a URN — policy versions are workflow-scoped and the `version` label is the stable identifier consumers cite. The `policyUrl` segment is URL-encoded in the path.

`ActorRef` (the URN-form principal reference at `_common.schema.json#/$defs/ActorRef`) is the canonical reference shape for every actor field on this domain — `delegator`, `delegatee`, `revokedBy`, `escalatedBy`, `escalatedTo`, `actorRef` on `AgentView`. Inline actor objects are forbidden per ADR 0082 D-9 / D-14.

## Pagination

`GET /api/v1/governance/agents`, `GET /api/v1/governance/delegations`, and `GET /api/v1/governance/policies/{policyUrl}/versions` use cursor pagination per `api/pagination.schema.json` (ADR 0082 D-7). Cursors are opaque, single-use within the issuing deploy; cursor expiry returns `410 Gone` with `WOS-1410`. No `total`, `page`, or `pageSize` echo.

## Idempotency

`POST /api/v1/governance/delegations` and `POST /api/v1/governance/delegations/{urn}/revoke` require `Idempotency-Key` per ADR 0082 D-16. Server retains the request/response pair for at least 24 hours keyed by `(idempotency-key, route, scope)`. Repeated identical requests return the original response unchanged.

`GET` endpoints are idempotent by construction.

## Errors

All non-2xx responses use `application/problem+json` per ADR 0082 D-8 and `api/error.schema.json`. Domain-relevant codes from the registry:

- `WOS-1404`: agent / delegation / policy version does not exist or is not in the caller's scope.
- `WOS-1409`: revoke applied to an already-revoked delegation that has settled into a terminal state at a different `revokedAt` (idempotency-key mismatch case).
- `WOS-1410`: cursor expired.
- `WOS-1422`: request failed schema validation, including `delegator`/`delegatee` URN principal-class mismatch with policy expectations or `scope.maxDollarThreshold` below 0.
- `WOS-1503`: durable runtime backend unavailable.

## Closed Taxonomies

This domain introduces or projects the following closed taxonomies (ADR 0082 D-12). Vendor extensions use the `^x-[a-z]+-` enum-extension pattern.

| Taxonomy | Reserved literals | Vendor extension |
|---|---|---|
| `AutonomyLevel` | `none \| suggest \| recommend \| act-with-review \| autonomous` | `^x-[a-z]+-` |
| `AgentDriftStatus` | `nominal \| watch \| recalibration-required \| demoted \| unknown` | `^x-[a-z]+-` |
| `AgentType` | `deterministic \| statistical \| generative` (ai-integration.md §3.1) | `^x-[a-z]+-` |
| `DelegationAuthority` | `signing \| determination \| review \| override` | `^x-[a-z]+-` |
| `EscalationReason` | `sla-breach \| hold-timeout \| manual \| policy-violation \| agent-drift \| quorum-shortfall \| appeal-filed \| agent-fallback-terminated \| confidence-floor-breach \| agent-volume-exceeded` (last three: ai-integration.md §7.4 / §8 / §11.1) | `^x-[a-z]+-` |
| `InvokerKind` | `anthropic \| claudeAgentSdk \| mcp \| a2a \| http \| stub` (ADR 0064 §2.2-§2.3) | `^x-[a-z]+-` |
| `AgentLifecycleState` | `active \| degraded \| suspended \| retired` (advanced-governance.md §7.1) | `^x-[a-z]+-` |
| `IndependenceVerification.verificationMethod` | `separation-of-duties-policy \| delegation-graph-walk \| manual-attestation \| organizational-unit-disjoint \| supervisor-attestation` (governance §3.5) | `^x-[a-z]+-` |
| `DeonticKind` | `permission \| prohibition \| obligation \| right` (OASIS LegalRuleML §3) | none — closed; mirrored from kernel |
| `DelegationScope.impactLevels[]` | `rights-impacting \| safety-impacting \| operational \| informational` (Kernel §6) | none — closed; mirrored from kernel |
| `DcrConstraintZoneState.currentLevel` (lives on `instance.schema.json`) | `none \| caution \| breach` (advanced-governance.md §1.2) | none — closed |
| `ShadowModeSpec` / `CircuitBreakerSpec.currentStatus` | `closed \| open \| half-open` (advanced-governance.md:387-400) | none — closed |

`AutonomyLevel` reserved literals draw from the kernel `AgentDeclaration.autonomy` enum (`readOnly | assistive | supervised | autonomous`) plus the `none` sentinel. Names diverge intentionally — the kernel uses operator-facing labels; this view uses outcome-facing labels (`suggest`/`recommend`/`act-with-review`) closer to the AI-oversight prose in governance §S2 and the case-portal review-protocol UI. The kernel labels remain authoritative for the workflow declaration; this view is the projection.

## Kernel Mirrors

The contract projects (does not redefine) the kernel governance model. `x-wos.mirror` annotations enable the Gate 6 byte-parity check (`work-spec/scripts/check-api-mirror-parity.py`). Annotated mirrors:

| API definition | Kernel source | Mirror annotation |
|---|---|---|
| `DeonticKind` | `wos-workflow.schema.json#/$defs/DeonticConstraint/properties/kind` | `{"source": "wos-workflow.schema.json", "path": "$defs/DeonticConstraint/properties/kind"}` |

Other kernel relationships are documentary (described in $def `description` prose) but not byte-pinned because the API projection diverges intentionally on closed-with-vendor-extension shape vs. closed-only kernel enums:

- `AutonomyLevel` reserved literals mirror the conceptual content of `wos-workflow.schema.json#/$defs/AgentDeclaration/properties/autonomy` (kernel `readOnly|assistive|supervised|autonomous`) but with the renamed outcome-facing labels above and the `none` sentinel; not byte-pinned.
- `DelegationAuthority` reserved literals mirror `wos-workflow.schema.json#/$defs/Delegation/properties/authority` (kernel `signing|determination|review|override`) but with vendor-extension support; not byte-pinned because the kernel form is closed-only.
- `Delegation` properties (`delegator`, `delegatee`, `authority`, `scope`) project from `wos-workflow.schema.json#/$defs/Delegation`; the kernel-side identifiers are plain strings (workflow-scoped actor ids) while this projection uses `ActorRef` URNs (ADR 0082 D-9). Byte-parity is structurally inexpressible — the URN-vs-bare-id divergence is the projection.
- `DelegationScope` (`impactLevels`, `caseTypes`, `maxDollarThreshold`, `conditions`) projects from `wos-workflow.schema.json#/$defs/DelegationScope`; the kernel form carries `patternProperties: {^x-: ...}` for arbitrary `x-` properties, which the public API layer does not project (public-API shapes use the `^x-[a-z]+-` enum-extension pattern at enum sites, not arbitrary `x-` properties on object types). Not byte-pinned.
- `DeonticConstraintSummary` projects from `wos-workflow.schema.json#/$defs/DeonticConstraint` — `id`, `kind`, `actor`, `expression`, `condition`, `defeasible`, `citationRefs` carry through; the kernel-only `x-legalruleml-iri` mapping is not projected. Not byte-pinned at the object level; the closed `DeonticKind` enum it nests IS byte-pinned through `DeonticKind`.
- `EscalationEvent` projects from the kernel runtime concept of an escalation step firing (governance §S6, §S10.4.4); the kernel `EscalationStep` is the *declaration* shape, while this is the *event* shape (one event per step the runtime walks). Not byte-pinned because the shapes serve different purposes.

## Schema Cross-References

Per ADR 0082 D-14, the schema `$ref`s the canonical definitions in `_common.schema.json` and `pagination.schema.json` instead of redefining:

- `https://schemas.formspec.io/wos-api/_common/v1#/$defs/ActorRef`
- `https://schemas.formspec.io/wos-api/_common/v1#/$defs/WosResourceUrn`
- `https://schemas.formspec.io/wos-api/pagination/v1#/$defs/CursorToken`
- `https://schemas.formspec.io/wos-api/pagination/v1#/$defs/PageLimit`

The API layer is a projection, not an alternative reality (ADR 0082 D-14).

## Non-Goals

- Authoring agents, delegations, or policy versions through workflow document edits — that surface lives on the WOS authoring tools, not the public API.
- Sub-delegation creation through the public API — sub-delegation chains beyond depth 1 require workflow-level governance ratification per governance §S11.5; this domain only surfaces the `allowsSubDelegation` flag on existing delegations.
- Drift recalibration triggers — `AgentDriftStatus` is read-only at this layer; recalibration events are emitted by the runtime and surfaced via the audit domain (deferred).
- Quorum-based delegation construction (governance §S4.9) — the `Delegation.scope` shape does not surface `quorumCount`/`quorumPool` because quorum delegations cross multiple delegator records; quorum is a governance authoring concern, not a public-API CRUD concern.
- Escalation event submission — `EscalationEvent` is read-only at this layer (the runtime emits them); manual escalation is performed through case-action endpoints in the instance/task domains.
- Policy version creation through the public API — `PolicyVersion` records are committed by governance authoring tools; this domain surfaces them read-only.
- Streaming or push notifications — out of scope per ADR 0082 D-16.
