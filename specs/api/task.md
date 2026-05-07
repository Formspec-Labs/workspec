# WOS Public API Task

**Status:** Draft
**ADR:** [`thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md`](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) (D-15 step 4)
**Schema:** [`api/task.schema.json`](../../schemas/api/task.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/task/v1`

## Purpose

`Task` is the public projection of a single unit of governed actor work. Tasks are created by kernel `createTask` actions (Kernel S9.2) and governed by the workflow-governance task-management contract (governance S10). Greenfield per ADR 0082 D-15: the kernel runtime artifact (`wos-case-instance.schema.json#/$defs/ActiveTask`) and the prior `case-portal/src/ports/types.ts` `TaskView` are prior art, not this contract.

The contract carries identity, workflow binding, assignment, contract-binding context (form definition or JSON Schema), the closed user-facing lifecycle, and the optional draft response. Submission splits into a draft (idempotent partial save) and a final response (require `Idempotency-Key`).

## Resource Shape

`Task` carries:
- `id`: `urn:wos:<typeid>` URN per ADR 0092 D-1. Tasks are sub-resources identified by URL path context; their wire identity uses the parent case TypeID.

- `instanceId`: `urn:wos:<typeid>` URN of the owning case.
- `workflowUrl`: governing Workflow Document.
- `kind`: closed-with-vendor-extension task family (see "Closed taxonomies" below).
- `status`: closed user-facing posture (no vendor-extension seam — see "Closed taxonomies").
- `impactLevel`: closed-with-vendor-extension impact level (Kernel S6) mirroring `wos-workflow.schema.json#/$defs/ImpactLevel`.
- `title`, `description`: actor-facing prose.
- `assignedActor`, `delegatedFrom`: `ActorRef` URNs hoisted to `_common.schema.json`.
- `binding`: closed-with-vendor-extension contract-binding kind (`formspec | jsonSchema | freeform`) mirroring `wos-case-instance.schema.json#/$defs/ActiveTask/properties/binding`.
- `contractRef`, `definitionUrl`, `definitionVersion`: pinned contract references; REQUIRED when `binding` is `formspec` or `jsonSchema`.
- `deadlines`: full set of named SLA / statutory / internal deadlines (governance §10.4.1; workflow-governance.md:548-560 — `slaDefinitions`). Each entry is `TaskDeadline { kind: sla | statutory | internal, at: ExpirableTimestamp, severity: advisory | enforcing, id? }`. Multi-SLA workflows expose both `firstResponse` and `fullResolution` here so the applicant can see both clocks and the staff portal can prioritize by which window is closer to breach.
- `reviewProtocol`: closed-with-vendor-extension review-protocol classification (governance §4.1-§4.2; workflow-governance.md:248-260) — `independentFirst | considerOpposite | calibratedConfidence | dualBlind | unassisted`. Present iff `kind == review` and the workflow declares a review protocol on this task pattern.
- `mustCompleteBeforeContinuationEnds`: when true, this task MUST be completed before the case's `continuationOfServicesEndsAt` window closes (governance §3.6 cross-cut). Applicant- and staff-facing UIs surface this flag to prioritize work that gates continued service delivery during an appeal window.
- `assignmentRoles`: `AssignmentRoles` projection (governance §7.2 + §10.2; workflow-governance.md:502-514). Closed object with optional `excludedOwner: ActorRef`, `requiredOwner: ActorRef`, `eligiblePool: ActorRef[]`, `currentAssignee: ActorRef`, `delegatedFrom: ActorRef`, `businessAdministrator: ActorRef`. Surfaces the kernel's five-role table so a staff portal can answer "who is excluded?", "who is in the pool?", "who is the business administrator?" without re-deriving from delegation graphs. The §10.2 precedence rule (`excludedOwner` overrides all other roles, workflow-governance.md:514) is enforced at the runtime, not in the projection.
- `draftResponse`, `draftedAt`: present only when `status == drafted`.
- `lastValidationOutcome`: most recent WOS `ValidationOutcome` wrapper from Kernel §13.6 / `wos-case-instance.schema.json#/$defs/ValidationOutcome`.
- `createdAt`, `updatedAt`: RFC 3339 UTC timestamps per ADR 0082 D-10.

`TaskListItem` is a lighter projection for inbox-style list views — same identity, classification, status, deadline, assignment. Detail navigation fetches the full `Task`.

## Closed taxonomies

`TaskStatus` is closed-no-extension by design: `pending | drafted | submitted | dismissed | expired`. The richer kernel runtime states (`created | assigned | claimed | delegated | escalated`; governance §10.1) live on the `wos-case-instance.schema.json#/$defs/ActiveTask.status` enum and are processor-internal — the public API collapses them into the user-visible posture. Richer kernel-side outcomes surface on `TaskOutcomeEvent.outcome` and in provenance, not on the public-list `status`. Adding a new status requires a schema major bump (ADR 0082 D-12).

`TaskKind` is closed-with-vendor-extension. Reserved literals at v1: `intake | review | determination | signature | correspondence-response | hold-resolution | escalation | verification`. Vendor extensions MUST use an `^x-[a-z]+-` prefix (ADR 0082 D-12).

`TaskBinding` is closed-with-vendor-extension. Reserved literals: `formspec | jsonSchema | freeform`. Mirrors `wos-case-instance.schema.json#/$defs/ActiveTask/properties/binding` plus an explicit `freeform` literal for tasks with no machine-validated payload (typical of dismissals or simple acknowledgements). Vendor extensions MUST use an `^x-[a-z]+-` prefix.

`TaskOutcomeKind` is closed-no-extension (matching `TaskStatus`): `submitted | dismissed | expired | delegated | escalated | drafted`.

`TaskDismissal.reason` is closed-with-vendor-extension: `not-applicable | delegated-to-other-actor | duplicate | in-error | out-of-scope`.

`ReviewProtocolKind` is closed-with-vendor-extension (governance §4.1-§4.2): `independentFirst | considerOpposite | calibratedConfidence | dualBlind | unassisted`. The `independentFirst` literal is the structural enforcement seam for the §4.2 obligation: a staff portal observing `Task { kind: review, reviewProtocol: independentFirst }` MUST lock recommendation visibility until the reviewer's independent assessment is recorded.

`TaskDeadline.kind` is closed-no-extension: `sla | statutory | internal`. `TaskDeadline.severity` is closed-no-extension: `advisory | enforcing`. `enforcing` deadlines fire the workflow's breach-policy escalation chain on expiry.

## Endpoints

```
GET   /api/v1/tasks                                  -> TaskPage           (cursor-paginated)
GET   /api/v1/tasks/{id}                             -> Task
POST  /api/v1/tasks/{id}/draft                       -> Task               (idempotent on body)
POST  /api/v1/tasks/{id}/response                    -> Task               (Idempotency-Key REQUIRED)
POST  /api/v1/tasks/{id}/dismiss                     -> Task               (idempotent on (taskId, reason))
GET   /api/v1/instances/{instanceId}/tasks           -> TaskPage           (subresource of instance, D-3)
```

`GET /api/v1/tasks` accepts `TaskListOptions`: `assignedActor` (defaults to the calling actor), `status`, `kind`, `instanceId`, `workflowUrl`, `deadlineBefore`, `cursor`, `limit` (max 200). Returns `TaskPage` (cursor envelope per `pagination.schema.json`, ADR 0082 D-7). Default scope is the calling actor's inbox; admins MAY widen via `assignedActor` subject to ReBAC checks.

`GET /api/v1/tasks/{id}` returns the full `Task` projection. While the workflow runs `lastValidationOutcome` on each draft save, clients MAY refresh by re-fetching this endpoint.

`ValidationOutcome` reports three independent axes: `envelopeValid` (the Formspec response envelope validates), `pinMatch` (`definitionUrl` / `definitionVersion` match the task pin), and `definitionValid` (Formspec Definition validation over `response.data` passes). `errors[]` is REQUIRED and carries WOS-level envelope / pin / aggregate validation failures; `validationResults[]` is OPTIONAL and carries Formspec-shaped validation results when definition validation ran. The API shape intentionally omits derived fields such as `valid`, `validatedAt`, `errorCount`, and `summary`; clients derive aggregate validity from the three axes and read timestamps from the surrounding `Task.updatedAt` / event stream.

`POST /api/v1/tasks/{id}/draft` accepts `TaskDraft { definitionUrl, definitionVersion, response, draftedAt? }`. Saves a partial response without committing it; the server runs the workflow's validation pipeline against the draft and returns the resulting `Task` projection (with `status == drafted`, `draftResponse` populated, `lastValidationOutcome` refreshed). Idempotent on `(taskId, definitionUrl, definitionVersion, response)` — repeated identical drafts return the same drafted-state representation.

`POST /api/v1/tasks/{id}/response` accepts `TaskSubmission { definitionUrl, definitionVersion, response, submittedAt?, signatureRef? }`. Commits the final response. The `Idempotency-Key` HTTP header is REQUIRED (ADR 0082 D-16). Successful submission transitions the task to `submitted`, fires the kernel `taskCompleted` event, and projects the response into case state via the workflow's `responseMappingRef`. Submission against a non-`pending`/`drafted` task is rejected with `WOS-1409`. `signatureRef` is REQUIRED when `kind == signature`; the value is the URN of a `SignatureAffirmation` provenance record produced by the signature profile.

`POST /api/v1/tasks/{id}/dismiss` accepts `TaskDismissal { reason, detail?, dismissedAt? }`. Declines the task without submitting a response; the server transitions the task to `dismissed` and fires the kernel `taskSkipped` event with the supplied rationale (governance §10.1 `skipped` requires structured rationale). Dismissal against a non-`pending`/`drafted` task is rejected with `WOS-1409`. Idempotent on `(taskId, reason)`.

`GET /api/v1/instances/{instanceId}/tasks` is the case-scoped task subresource per ADR 0082 D-3. Same `TaskListOptions` query surface; `instanceId` is implied from the path. Cursor-paginated. The same shape is also embedded inline (lighter `TaskListItem`) when `GET /api/v1/instances/{instanceId}?include=tasks` is requested.

## Identifiers

- `Task.id`: `urn:wos:<typeid>` URN per ADR 0092 D-1. Tasks are sub-resources identified by URL path context; their wire identity uses the parent case TypeID.
- `actorRef` fields use the canonical `ActorRef` URN.
- `signatureRef`, `provenanceRecordRef`: `WosResourceUrn` references to provenance / signature records.

## Pagination

`GET /api/v1/tasks` and `GET /api/v1/instances/{instanceId}/tasks` use cursor pagination per `api/pagination.schema.json`. Cursors are opaque, single-use within the issuing deploy. Cursor expiry returns `410 Gone` with `WOS-1410` (ADR 0082 D-7). No `total`, `page`, or `pageSize` echo.

## Idempotency

`POST /api/v1/tasks/{id}/response` requires `Idempotency-Key` per ADR 0082 D-16 — submission is the only externally-visible side-effecting endpoint that is not naturally idempotent on its body.

`POST /api/v1/tasks/{id}/draft` is idempotent on `(taskId, definitionUrl, definitionVersion, response)` so callers can retry without `Idempotency-Key`.

`POST /api/v1/tasks/{id}/dismiss` is idempotent on `(taskId, reason)`.

## Errors

All non-2xx responses use `application/problem+json` per ADR 0082 D-8 and `api/error.schema.json`. Domain-relevant codes from the registry:

- `WOS-1404`: task does not exist or is not in the caller's scope.
- `WOS-1409`: state mismatch — submission against a non-`pending`/`drafted` task; `definitionUrl`/`definitionVersion` mismatch with the task's pinned values; `signatureRef` absent on a `kind == signature` submission.
- `WOS-1410`: cursor expired.
- `WOS-1422`: request failed schema or contract validation, including response payload not matching the workflow's task contract; vendor-extension `kind` literal not accepted by the deployment.
- `WOS-1503`: durable runtime backend unavailable.

## Outcome events

`TaskOutcomeEvent` is the wire shape of a task state change emitted to subscribed observers (notifications domain projections, server-side downstream consumers). Carries the task URN, the new posture, the responsible actor, the closed `TaskOutcomeKind`, and an optional `provenanceRecordRef` URN linking to the kernel S8 provenance record. The structurally-identical `Task` resource is also retrievable via the GET endpoint; this event is the projection seam for incremental consumers.

## Kernel mirrors

The contract projects (does not redefine) the kernel runtime model. `x-wos.mirror` annotations enable the Gate 6 mirror-parity check:

| API definition | Kernel source | Mirror annotation |
|---|---|---|
| `TaskBinding` | `wos-case-instance.schema.json#/$defs/ActiveTask/properties/binding` | `x-wos.mirror = "wos-case-instance.schema.json#/$defs/ActiveTask/properties/binding"` |
| `ImpactLevel` | `wos-workflow.schema.json#/$defs/ImpactLevel` | `x-wos.mirror = "wos-workflow.schema.json#/$defs/ImpactLevel"` |

`TaskStatus`, `TaskKind`, `TaskOutcomeKind`, and `TaskDismissal.reason` are wire-projection taxonomies without an exact kernel `$def` — the kernel `ActiveTask.status` carries different (richer, processor-internal) semantics, and the `kind` / outcome / dismissal-reason taxonomies are API-introduced. They do not carry the mirror annotation; cross-spec drift is caught by the spec text and the closed-with-vendor-extension discipline.

## Non-Goals

- Form rendering — clients fetch the form definition by resolving `definitionUrl`/`definitionVersion` against the Formspec subsystem; this contract carries the references only.
- Task creation — tasks are created by kernel `createTask` actions, not by API callers. There is no `POST /api/v1/tasks` create endpoint.
- Task delegation as an explicit API action — delegation is a governance concern that updates `assignedActor` server-side; the wire signal is `TaskOutcomeEvent { outcome: "delegated" }`. A future delegation-domain spec (D-15 step 5) carries the explicit delegation API.
- SLA breach policies — the `expired` status is observable on the wire; configuration of breach behavior lives on the governance authoring surface (`wos-workflow.schema.json` task patterns + governance §10.4).
- Streaming task feeds — out of scope per ADR 0082 D-16.
