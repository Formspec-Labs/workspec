# Native due-process execution

This specification defines the bounded native execution surface for Governance
§3. It attaches through the existing `lifecycleHook`, `contractHook` and
`provenanceLayer`. Declaring this configuration does not qualify a hosted
transport, establish legal compliance or approve publication.

## Normative Contract

### Immutable Formspec selections

Scenario replay MUST execute selected Formspec documents through
`wos-formspec-binding::FormspecProcessor`, using the maintained Formspec
Definition evaluator and Mapping implementation. Each selection retains
`role`, `url`, `version` and original `bytes`. A Mapping additionally retains
`definition: {url, version}` selecting an exact Definition; its authored
Definition version range remains independently enforced. A Mapping has no body
URL, so its selected URL is its immutable resolution locator.

Repeated `definition` and `mapping` roles are permitted. Exact duplicate
identities, ambiguous task references, missing selections, incompatible pins or
Mapping directions MUST refuse before process creation. A task's prefill uses
reverse mapping; its response uses forward mapping. A selected Definition and
Mapping MUST agree with the actual task's Definition pin and Mapping reference.
Execution MUST NOT discover or fetch replacement documents.

Selected execution MUST validate the complete Response, run Definition evaluation
with the Submit trigger at the controlled runtime clock, and retain actual
blocking validations. Mapping MUST consume the admitted Response and preserve
native Mapping diagnostics. This reference route supports self-contained flat fields and display items.
Nested/repeated/prePopulate reconstruction, instances, modules/extensions and
unavailable host context MUST refuse. A processor that cannot reconstruct a
Response changed by evaluation MUST refuse that behavior. Mapping ambient time
and provider functions MUST refuse because the maintained Mapping API does not
accept the qualified runtime context; no second interpreter may supply them. Named instances, external
lookups and extension providers MUST refuse unless their owning qualification
path is supplied. Canned fixture validation or mapping MUST NOT replace a
selected native operation. Mixed canned outcomes and selected artifacts MUST
refuse. Unselected conformance fixtures retain their separately labeled fixture
processor.

Replay identity MUST retain original and canonical document digests, explicit
Mapping pairing, processor mode and the declared source scope of the native
implementations actually used. A partial source fingerprint MUST identify its
scope and MUST NOT claim a complete compiled dependency graph.

The additive authored recipe, service fixture and staged comparison protocol is
defined in [Native reference replay](reference-replay.md).

### Closed reference profile and notation

The remainder defines profile `referenceDueProcess/1`. It proves selected
software-fixture behavior through the maintained native owners. A processor MUST
refuse unsupported combinations before activating the workflow. It MUST retain
the original illustrative source bytes and MUST NOT substitute live law.

The notation below defines JSON objects: every listed field is REQUIRED unless
followed by `?`; `T[]` is an array; `A | B` is a tagged union; quoted values are
literal strings. Objects are closed: additional fields and explicit null MUST
refuse. Arrays of references MUST be nonempty and unique unless stated otherwise.
Numbers MUST be finite. No command or transport object accepts arbitrary JSON.
Existing canonical types named below retain their owning schema, rather than a
second definition here.

| Type | Exact meaning |
|---|---|
| `Id` | String matching `^[A-Za-z][A-Za-z0-9._-]*$`; at most 128 UTF-8 bytes. Configuration IDs are unique within their collection. |
| `Token` | String matching `^[A-Za-z0-9._-]{1,128}$`. |
| `SnapshotDigest` | Exactly 64 lowercase hexadecimal digits, preserving existing CaseFileSnapshot.sha256 and snapshotSha256 representation. |
| `Digest` | `sha256:` followed by 64 lowercase hexadecimal digits, computed with maintained canonical primitives. |
| `Timestamp` | Exact UTC RFC 3339 output of the maintained native timestamp formatter for nonnegative representable Unix milliseconds: whole seconds use `YYYY-MM-DDTHH:mm:ssZ`; fractional milliseconds use one through three digits with trailing zeroes removed. Parse-and-format roundtrip MUST equal the original string. Calendar/range validity and representability are native semantic checks; client timestamps never establish operation time. |
| `Days` | Integer from 1 through 36500. |
| `DayDuration` | Canonical ISO 8601 `P<n>D`, where `n` is `Days`. |
| `Pointer` | Nonempty RFC 6901 JSON pointer; each escape MUST be valid. Resolution MUST yield exactly one required node of the specified type. |
| `Human` | Canonical `actor:human:` identity accepted by Kernel §3.4's retained `HumanActorBinding`; no local role alias. |
| `ActorRef` | Exactly one declared workflow actor ID. |
| `RecordId` | ID of an actual saved native provenance record in this process; MUST resolve exactly once. |
| `Channel` | `phone | in-person | written`. |
| `DeliveryChannel` | `mail | portal`. These values select this profile's fixture adapter channels; they do not qualify postal or portal delivery. |
| `SourceRef` | `{ sourceVersion: Id, sectionAnchor: string, retainedSha256: Digest }`; sectionAnchor is nonblank, at most 1024 bytes. Digest identifies retained selected source bytes. |
| `ReviewRef` | `{ id: Id, sha256: Digest }`; identifies retained review bytes. Execution does not create or approve the review. |
| `Scalar` | `{kind:"string", value:string}` or `{kind:"number", value:number}` or `{kind:"boolean", value:boolean}`. String values have at most 4096 UTF-8 bytes. |

All references are process-local unless their type explicitly names selected
source bytes, selected templates or workflow locations. `definitionSha256` is
the canonical digest of the immutable governing workflow; it is not its URL or
version. Stored generated IDs use the owning native ID family where one exists.

### Exact locations and causal explanation

```text
TransitionLocation = { statePointer: Pointer, transitionIndex: integer >= 0 }
FiringRef = {
  definitionSha256: Digest, location: TransitionLocation,
  provenanceRecordId: RecordId
}
CaseAlternative = {
  id: Id, changes: CaseChange[], description: string, sourceRefs: SourceRef[]
}
CaseChange = { casePointer: Pointer, value: Scalar }
ReasonBinding = {
  tableRef: Id, rowId: string, reasonCode: Id,
  sourceAuthority:"statute" | "regulation" | "policy" | "guideline",
  description: string, sourceRefs: SourceRef[]
}
ExplanationBinding = {
  tableRef: Id, outputColumn: Id, beneficialOutput: Scalar,
  reasons: ReasonBinding[], positive: CaseAlternative[], negative: CaseAlternative[]
}
```

`TransitionLocation` points to the owning state and the zero-based index in its
`transitions` array. The native evaluator MUST capture it at selection, through
ordinary, parallel, compound, continuous-rescan, timer and ForEach firing paths.
It MUST NOT rediscover it from equal bodies, event names, tags or source/target
pairs. A transition firing retains its actual provenance record ID. Repeated
firings at one location have distinct `FiringRef` values. False guard tests retain
the tested location but MUST NOT become determination firings.

The determining transition MUST use the selected native decision-table guard.
The owner MUST retain actual table inputs, matched row IDs, selected row ID,
output values and guard result. `tableRef` and `outputColumn` MUST resolve to that
actual table/output. Only a reason binding for the actually selected row can
produce factual reasoning. Descriptions are nonblank selected policy text, at
most 4096 bytes; actual input/output values accompany them in native provenance.
Event reason payloads and expected assertions MUST NOT supply reasoning facts.

Before notice assembly, the native owner MUST evaluate every selected positive
and negative alternative against a private clone of the retained determination
snapshot, using the same native table evaluator, input bindings, time and
selected dependencies. Each casePointer MUST address an existing scalar case
field; duplicate/ancestor-overlapping changes, instance/provider access or
unsupported native evaluation MUST refuse. No alternative mutates actual case
state. ReasonBinding.rowId is the exact nonblank native table row ID, at most
128 bytes; it does not acquire configuration-ID syntax. At least one positive alternative MUST actually select the configured
beneficial output, different from the original adverse output. Every negative
alternative MUST actually preserve the original output and adverse guard result.
No match, ambiguous match, evaluation diagnostic, unsupported value or failed
comparison MUST produce a passing counterfactual. Alternatives and results retain
their own snapshot digests, actual rows and outputs.

Reasoning and successful alternatives MUST be related to the actual determining
`FiringRef.provenanceRecordId`. The owner MUST use the existing
`assemble_explanation` algorithm over those native records. Required reasoning,
positive and negative counterfactual families MUST be nonempty. Each negative
claim describes only its executed alternatives; it does not establish universal
fairness or legal compliance. Unsupported causal explanation remains refused;
processors MUST NOT disable a rights-impacting requirement to pass the fixture.

### Native provenance discriminants

OperationState.recordId and GovernanceResult.operationRecordId MUST be the
actual bound operation transition's StateTransition provenance ID, equal to
OperationState.firing.provenanceRecordId. This profile introduces no generic
GovernanceOperation record kind. NoticeState.id is its actual issue-operation
StateTransition ID; staging does not invent NoticeSent. AppealState.id resolves
to the actual existing AppealFiled provenance record. ExtensionState retains the
actual existing OverrideRecorded record and its canonical OverrideRecord.
Held/scheduled/disposition facts reside in the typed state linked to their actual
bound transition IDs. A fabricated extra StateTransition is forbidden.

Causal explanation requires two narrowly defined new provenance discriminants:
`determinationReasoning` and `determinationCounterfactual`, serialized through the owning canonical event literals `wos.governance.reasoning_recorded` and `wos.governance.counterfactual_recorded`. Their actor/process/
time fields use the existing native provenance shape and actual attribution;
their data fields are exactly the closed types below. These discriminants and
payload schemas MUST receive their owning schema/type update before activation.
They MUST NOT be disguised as unrelated existing record kinds.

```text
NamedScalar = {name:Id, value:Scalar}
NativeDecisionEvaluation = {
  location:TransitionLocation, tableRef:Id, inputs:NamedScalar[],
  matchedRowIds:string[], selectedRowId?:string, outputs:NamedScalar[],
  guardResult:boolean
}
DeterminationReasoningData = {
  tier:"reasoning", relatedTransition:RecordId, ruleId:Id,
  authority:"statute" | "regulation" | "policy" | "guideline",
  description:string, timestamp:Timestamp,
  evaluation:NativeDecisionEvaluation, sourceRefs:SourceRef[]
}
DeterminationCounterfactualData = {
  tier:"counterfactual", relatedTransition:RecordId,
  type:"positive" | "negative", factor:Id, description:string,
  alternative:CaseAlternative,
  originalSnapshotSha256:SnapshotDigest, alternateSnapshotSha256:SnapshotDigest,
  originalEvaluation:NativeDecisionEvaluation,
  alternateEvaluation:NativeDecisionEvaluation, sourceRefs:SourceRef[]
}
```

NativeDecisionEvaluation retains a tested native guard; only its matching actual
selected FiringRef can establish the original determination. Arrays of inputs,
rows and outputs reflect actual native evaluation and may be empty only where
the selected table permits it; original determination reasoning requires an
actual selected row. Alternative evaluations are explicitly hypothetical, not
new workflow firings. Explanation descriptions and authority come from the
selected reason/alternative binding. Actual evaluation facts, relatedTransition,
timestamps and snapshot digests come from native execution. In particular,
snapshotSha256 in notice content and signed determination association MUST retain
the original bare SnapshotDigest, not a newly prefixed or recomputed meaning.

### Selected policy configuration

```text
DueProcessExecution = {
  profile: "referenceDueProcess/1",
  determinations: DeterminationBinding[], operations: OperationBinding[],
  notice: NoticePolicy, appeal: AppealPolicy,
  preNoticeExtension?: ExtensionPolicy, limits: Limits
}
DeterminationBinding = {
  id: Id, transition: TransitionLocation, pendingStatePointer: Pointer,
  appliedStatePointers: Pointer[], servicePointers: Pointer[],
  signatureTaskRef: Id, explanation: ExplanationBinding, sourceRefs: SourceRef[]
}
OperationBinding = {
  id: Id, kind: OperationKind, determinationRef: Id,
  actors: ActorRef[], requiredStatePointers: Pointer[],
  triggerEvent: Id, transition: TransitionLocation, sourceRefs: SourceRef[]
}
OperationKind = "issueNotice" | "fileAppeal" | "scheduleHearing" |
  "holdHearing" | "recordDisposition" | "requestServiceTermination" |
  "extendPreNoticeWindow" | "applyAdverseEffect"
NoticePolicy = {
  recipientPointer: Pointer, deliveryChannel: DeliveryChannel,
  templates: NoticeTemplateSelection[], evidenceClass: "fixtureQualifiedReceipt",
  appealAnchor: "noticeIssued", graceAnchor: "qualifiedReceipt",
  sourceRefs: SourceRef[]
}
NoticeTemplateSelection = {
  locale: string, templateKey: Id, review: ReviewRef
}
AppealPolicy = {
  channels: Channel[], standingActors: ActorRef[], reviewerActors: ActorRef[],
  excludedDeterminationActors: "determinerAndSupervisor",
  hearingDeadline: { calendarDays: Days, purpose: "hearingHeld" },
  continuation: {
    choice: "electionRequired" | "defaultUnlessWaived",
    scope: "currentLevel", endsOn: "dispositionOrHouseholdTermination"
  }, sourceRefs: SourceRef[]
}
ExtensionPolicy = {
  timerId: Id, allowedStatePointers: Pointer[], maxCumulativeCalendarDays: Days,
  sourceRefs: SourceRef[]
}
Limits = {
  maxDeterminations: integer >= 1, maxNotices: integer >= 1,
  maxAppeals: integer >= 1, maxOperations: integer >= 1,
  maxOutstandingNoticeWork: integer >= 1, maxTransportAttempts: integer >= 1,
  maxNoticeBytes: integer >= 1
}
```

`governance.dueProcess.execution` has exactly the shape above. Limit integers
MUST be at most 100000, except maxNoticeBytes, which MUST be at most 10485760.
Existing `adverseDecisionPolicy` supplies `noticeRequired:true`,
`noticeTiming:"beforeEffective"`, `noticeGracePeriod:DayDuration`,
`noticeTemplateKey`, `explanationLevel:"individualized"`,
`counterfactualRequired:true` and an enabled appeal mechanism with
`appealWindow:DayDuration` and `continuationOfServices:true`. Missing required
policy, other timing/explanation/evidence/anchor combinations, conflicting values
or unresolved continuation scope MUST refuse. `continuationPolicyRef`, when
present, MUST resolve to the selected typed current-level policy with the same
end condition; silently discarding that reference MUST refuse. Policy durations
and choices come from selected reviewed material; native SNAP constants are
forbidden. Scoped due-process policies remain unsupported by this profile.

Each determination transition MUST carry `adverse-decision`; every reachable
such transition MUST have exactly one binding. Its immediate target is the bound
pending state, which MUST be disjoint from all applied states. Each servicePointer
addresses an existing case value in the pre-transition snapshot and is relative
to case state. State/transition pointers are relative to the selected workflow.
Bindings with unresolved nodes, repeated operation kind for one determination,
or overlapping protected service pointers MUST refuse. Each operation transition
MUST be unique across operation bindings, reachable, and declare the exact
literal triggerEvent. There is no inferred operation binding by event name.

Only `issueNotice` and `applyAdverseEffect` may execute on their actual bound
workflow-owned transition without a public GovernanceRequest. Every other kind
requires the trusted native request-dispatch path below. The configured actors
MUST include the transition's declared actor where one is declared. All actors
resolve through native action admission; fixture configuration does not grant
access. All human actors MUST have original retained canonical bindings.
Determiners, supervisors and reviewers MUST be humans. Every reviewer MUST be
canonically distinct from the determining human and the signing supervisor,
including when different declared roles map to one human.

`signatureTaskRef` MUST resolve to the actual maintained Signature Profile task
that signs this determination. The task's full signed Formspec Response.data MUST
contain the exact flat fields
`determinationId`, `determinationSnapshotSha256` and
`determinationFiringRecordId`, matching retained native facts. These fields MUST
be part of the full Response bytes covered by the maintained verified signed
receipt; copied unsigned fields cannot establish association. The owner MUST
check the stored accepted Response, exact task/profile, actual canonical signer,
full signed-payload digest and these associations. It MUST preserve the selected
SignatureProfile documentHash semantics; this profile neither mutates a static
profile documentHash into an individualized hash nor invents another signing
primitive. A required SigningContentCommitment/displayed-content association must
also be present and valid under its existing owner; unavailable qualification
MUST refuse. Notice issuance requires an accepted `SignatureAffirmation` for
that exact task/content,
with `primitiveVerification.status:"verified"`, accepted consent, current signer
authority and selected posture checks. The canonical signer comes from actual
native evidence. A record kind, `{signed:true}`, unrelated document, untrusted
receipt or `deferredPendingHelper` status is insufficient. Signature Profile and
its verification receipt owner retain their meanings.

The selected `noticeTemplateKey` MUST equal the first template selection's key.
Every locale MUST resolve to exactly one selected Delivery template variant;
locale strings use the selected template's exact locale. The notice includes all
listed variants. Each required review MUST resolve to retained selected bytes.
This profile does not infer language equivalence or accessibility certification
from locale presence. Evidence for such claims remains separately reviewed.

### Time facts and supported timing matrix

This profile accepts exactly one timing row:

| noticeTiming | appealAnchor | graceAnchor | evidenceClass | Required relationship |
|---|---|---|---|---|
| `beforeEffective` | `noticeIssued` | `qualifiedReceipt` | `fixtureQualifiedReceipt` | Actual qualifiedAt equals immutable issueAt. |

The native clock determines `determinedAt` when the determination fires and
`issueAt` when the first notice work is staged after signature. `acceptedAt` is
an actual transport acceptance fact. `qualifiedAt` is an actual fixture transport
receipt fact. These are distinct fields even when equal. Only successful receipt
qualification establishes `noticeIssuedAt=issueAt`. The fixture transport MUST
actually issue/qualify at that native time; it MUST NOT copy issueAt into a later
receipt to satisfy equality.

The immutable notice prints `appealDeadline=issueAt+appealWindow` and
`effectEligibleAt=issueAt+noticeGracePeriod`. Qualified receipt at that same
instant makes those dates correct for this supported matrix. Before
qualification those dates are proposed dates on pending bytes; there is no
successful issuance or active appeal-clock assertion. Delayed/mismatched receipt
remains blocked/pending with original bytes and dates retained. It cannot qualify
an effect or be backdated, and this profile provides no silent reissue or
supersession route. A workflow requiring delayed receipt, other anchors or
changed printed dates MUST refuse this profile.

A calendar day is exactly 86400000 milliseconds in UTC. Add whole calendar-day
durations to the exact retained timestamp; local time zones, daylight-saving
adjustments, business calendars and holiday exceptions are unsupported here.
Overflow MUST refuse before mutation. Filing is timely at or before
appealDeadline. Effects require now greater than or equal to effectEligibleAt.
Scheduling/holding must occur at or before the retained hearing deadline. Retry
and runtime recreation MUST preserve every anchor. No deadline derives from a
mutable current case field or a retry timestamp.

### Request dispatch and canonical actors

```text
GovernanceRequest = {
  operationRef: Id, determinationId?: RecordId, idempotencyToken: Token,
  actor: { declaredActor: ActorRef, actingActor: string }, command: GovernanceCommand
}
GovernanceCommand =
  { kind:"issueNotice" } |
  { kind:"fileAppeal", channel:Channel,
    continuationChoice:"elect" | "waive" | "unspecified" } |
  { kind:"scheduleHearing", appealId:RecordId,
    reviewerActor:ActorRef, scheduledFor:Timestamp } |
  { kind:"holdHearing", appealId:RecordId, evidenceRefs:SourceRef[] } |
  { kind:"recordDisposition", appealId:RecordId,
    outcome:"upheld" | "reversed", rationale:string, evidenceRefs:SourceRef[] } |
  { kind:"requestServiceTermination", appealId:RecordId, rationale:string } |
  { kind:"extendPreNoticeWindow", calendarDays:Days,
    rationale:string, evidenceRefs:SourceRef[] } |
  { kind:"applyAdverseEffect" }
```

`determinationId` is REQUIRED for every command except extendPreNoticeWindow,
where it MUST be absent. Where required, it is the actual determining
transition's saved provenance record ID, not a configured binding ID or optional
occurrence; it MUST resolve exactly one saved occurrence under operationRef's
configured determination binding. Extension operates on its configured actual
pre-notice timer and allowed state before a determination exists; operationRef
still resolves its selected policy binding, without inventing a determination. `actingActor` is the independently admitted
actual actor: a retained Human for a human role, or the exact declared nonhuman
actor otherwise. A request cannot authenticate its own actor. Native admission
MUST compare current caller facts, retained assignment and operation actors;
missing, foreign or conflicting facts refuse. Unsupported delegation/reassignment
qualification MUST refuse rather than infer authority from a role string.
The bounded native governance preflight MUST explicitly enforce the selected
transition.actor and OperationBinding.actors against the validated declared slot
and its canonical acting identity, and MUST invoke current
AccessControl.can_transition before any effects or success save. The maintained
evaluator receives the actual acting identity for provenance. It does not itself
enforce transition.actor in the inspected baseline; this profile MUST NOT claim
that such a check already ran or omit it for that reason. Missing declared/canonical
mapping or a denied transition refuses. This is bounded governed-operation
admission, not a claim that unrelated generic transition actor enforcement has
been implemented. Local aliases MUST NOT replace actual human provenance.
Rationales are nonblank and at most 4096 bytes. Required evidenceRefs MUST resolve
to retained selected evidence; authored locators alone do not prove existence.

Dispatch MUST load the selected operation, check kind equality and actor access,
and pass only its fixed triggerEvent to the maintained lifecycle evaluator.
The event data is the exact object `{governance:{operationRef,determinationId,
command}}`, with determinationId omitted only for extendPreNoticeWindow; it
contains no additional fields. A trusted native invocation marker
travels outside this JSON. Copying the object into a generic event does not
produce that marker. Native guards, actor admission and document-order selection
run normally. Success requires the actual selected location to equal the
configured transition and the required source-state membership to hold. A false
guard or competing transition refuses the request, saves no prospective mutation
and produces no success evidence. The owner MUST NOT directly edit configuration
to force the expected transition.

A workflow-owned issue/apply transition uses the same preflight and policy checks
and an internal replay identity derived from its actual FiringRef. For other
bound kinds, a generic event selecting the transition MUST refuse before effects;
it cannot create an appeal, hearing or override fact. A same-named event selecting
an unrelated unbound transition produces no governance operation.

For public requests, the idempotency scope is `(processId,idempotencyToken)`.
The owner MUST retain and compare the complete canonical request, including actor,
operationRef, target and command. After current caller identity is admitted, an
exact retry MUST return its original saved result before rechecking mutable
business preconditions changed by its first success. Changed content under a
used token MUST refuse. Refusals do not consume the token. Public and internal
workflow replay namespaces MUST remain disjoint.

### Native state, operation results and milestones

```text
DueProcessState = {
  determinations: DeterminationState[], notices: NoticeState[], appeals: AppealState[],
  extensions: ExtensionState[], operations: OperationState[], replays: ReplayState[]
}
DeterminationState = {
  id:RecordId, bindingRef:Id, firing:FiringRef, determinedAt:Timestamp,
  snapshot:{ recordId:RecordId, sha256:SnapshotDigest }, determiningHuman:Human,
  servicePointers:Pointer[], signature?:{
    taskId:string, affirmationRecordId:RecordId, supervisorHuman:Human,
    signedContentSha256:Digest
  }, effect?:{ operationRecordId:RecordId, appliedAt:Timestamp }
}
NoticeState = {
  id:RecordId, determinationId:RecordId, artifactSha256:Digest,
  artifactBytesBase64:string, issueAt:Timestamp, appealDeadline:Timestamp,
  effectEligibleAt:Timestamp, recipient:string, channel:DeliveryChannel,
  variants:NoticeVariant[], transportKey:Token, correlation:Token,
  status:"pending" | "qualified" | "blocked" | "exhausted",
  attempts:integer >= 0, acknowledgements:TransportAcknowledgement[],
  transportResults:TransportResult[],
  receipt?:QualifiedNoticeReceipt, failure?:TransportFailure
}
NoticeVariant = {
  locale:string, templateKey:Id, review:ReviewRef,
  body:string, bodySha256:Digest
}
AppealState = {
  id:RecordId, determinationId:RecordId, noticeId:RecordId,
  filedAt:Timestamp, filedBy:Human, channel:Channel,
  continuationChoice:"elect" | "waive" | "unspecified",
  continuation:"frozen" | "waived" | "released",
  hearingDeadline:Timestamp,
  schedule?:{ operationRecordId:RecordId, scheduledAt:Timestamp,
    scheduledFor:Timestamp, reviewerActor:ActorRef, reviewerHuman:Human },
  hearing?:{ operationRecordId:RecordId, heldAt:Timestamp,
    reviewerHuman:Human, evidenceRefs:SourceRef[] },
  disposition?:{ operationRecordId:RecordId, recordedAt:Timestamp,
    outcome:"upheld" | "reversed", rationale:string, evidenceRefs:SourceRef[] },
  termination?:{ operationRecordId:RecordId, requestedAt:Timestamp,
    requestedBy:Human, rationale:string }
}
ExtensionState = {
  operationRecordId:RecordId, determinationBindingRef:Id, timerId:Id,
  calendarDays:Days, cumulativeCalendarDays:Days,
  priorDeadline:Timestamp, newDeadline:Timestamp,
  overrideRecord:OverrideRecord
}
OperationState = {
  recordId:RecordId, operationRef:Id, determinationId?:RecordId,
  kind:OperationKind, performedAt:Timestamp, actor:GovernanceRequest.actor,
  firing:FiringRef, outcome:GovernanceOutcome
}
ReplayState = { request:GovernanceRequest, result:GovernanceResult }
GovernanceResult =
  { status:"committed", operationRecordId:RecordId, outcome:GovernanceOutcome } |
  { status:"refused", code:RefusalCode }
GovernanceOutcome =
  { kind:"noticeStaged", noticeId:RecordId } |
  { kind:"appealFiled", appealId:RecordId } |
  { kind:"hearingScheduled", appealId:RecordId, scheduledFor:Timestamp } |
  { kind:"hearingHeld", appealId:RecordId, heldAt:Timestamp } |
  { kind:"dispositionRecorded", appealId:RecordId, outcome:"upheld" | "reversed" } |
  { kind:"serviceTerminationRequested", appealId:RecordId } |
  { kind:"preNoticeWindowExtended", overrideRecordId:RecordId, newDeadline:Timestamp } |
  { kind:"adverseEffectApplied", appliedAt:Timestamp }
```

OperationState.determinationId MUST be absent only for extendPreNoticeWindow
and MUST be present for every other kind. All state arrays may be empty. Saved generated IDs MUST resolve to exactly one
actual owning provenance record and its correct process. Base64 fields MUST
encode canonical retained bytes within maxNoticeBytes. Recipient is the exact
nonblank string resolved from recipientPointer, at most 1024 bytes. State MUST
NOT accept caller-supplied success records. Actual pre-effect service values are
read from the immutable original snapshot via servicePointers; hashes or mutable
current fields alone cannot reconstruct those values.

`issueNotice` stages deterministic bytes only after the required signature.
Machine-readable content MUST retain kind `adverseDecisionNotice`, determination
ID/FiringRef, snapshot digest, actual assembled Explanation, selected source and
template/review identities, recipient, both printed dates, accepted appeal
channels and continuation instructions. Human-readable variants render those
same facts. Identical retained inputs and issueAt MUST produce byte-identical
content and prose. `noticeStaged` means saved local work; it is not delivery.

`fileAppeal` requires a qualified notice, standing human, accepted channel and
timely native filing time. Channels configured together use the same operation
and state semantics. The native filing sets hearingDeadline to filedAt plus the
selected hearingDeadline duration. In electionRequired mode, elect freezes
services; unspecified refuses. In defaultUnlessWaived mode, elect/unspecified
freeze services and waive records explicit household waiver. A waiver/termination
requires the affected household's admitted canonical identity, not agency staff.
An existing unresolved appeal or second filing for that determination refuses.

`scheduleHearing` requires a pending appeal and an eligible independent canonical
human reviewer. scheduledFor MUST be at or after now and no later than
hearingDeadline. `holdHearing` requires that saved schedule, the same independent
reviewer as admitted acting human, actual now at or after scheduledFor, and
nonempty retained evidenceRefs. It records heldAt from the native clock. Missing
or late held evidence does not satisfy purpose hearingHeld, even when scheduling
was timely. Late holdHearing MUST refuse; it MUST NOT backdate or pretend the
hearing occurred. No payload field may supply heldAt.

`recordDisposition` requires actual held-hearing evidence and the same admitted
reviewer. It records actual native time and the separate upheld/reversed result.
This profile proves no disposition-notification deadline or notification delivery;
policy requiring either MUST refuse this profile. A reversed disposition forbids
applying the retained adverse determination. An upheld disposition may release
continuation, subject to remaining grace/evidence checks. A disposition record
alone is not held-hearing evidence.

When continuation is required or elected, every bound adverse operation MUST
preserve original service values until disposition or permitted explicit
household termination. Before appeal filing, current services remain frozen
during the configured appeal window unless the selected household waiver path
has actually executed. After the window with no appeal, or after permitted
release, applyAdverseEffect still requires qualified receipt and grace.
Cancellation by generic event, mere schedule, pending disposition, timeout before
the selected window or staff request cannot end continuation.

`extendPreNoticeWindow` requires selected ExtensionPolicy, its allowed state,
actual bound timer, native actor authority and the existing OverrideAuthority
requirements. Accepted commands append the existing canonical OverrideRecord
with actual authorityVerification, rationale and supportingEvidence; they MUST
validate against that owning schema. Sum accepted extensions cumulatively; refuse
beyond the selected cap. Add days to the retained timer deadline, never now.
The operation cannot change notice grace, appeal/hearing clocks, sent bytes or
service rights. Repeated token returns original evidence without a second add.

`applyAdverseEffect` requires the exact current determination, saved accepted
signature, qualified receipt/digest/correlation, elapsed grace, no frozen
continuation, no reversal and no earlier effect. It records appliedAt only after
all prospective changes pass preflight. It cannot be enabled by expected output,
case fields resembling receipt data, changed impact level or disabling notice.

### Prospective preflight and record bounds

The native owner MUST inspect the actual complete prospective evaluation before
any task, service, delivery or other effect and before saving success. It MUST
cover initial creation, intermediate transitions, automatic firings and
exit/transition/entry service writes. Determination transitions cannot themselves
enter an applied state or change protected service values. Unbound entry to an
applied state or adverse write to any configured protected service value MUST
refuse. Effects require the exact bound apply operation. Checking only the final
configuration is insufficient. Preflight MUST use the evaluator's captured
locations, guard/row results and action history; a second custom selector is
forbidden.

A refused preflight leaves saved state, provenance implying success and external
effects unchanged. The owning reference store MUST atomically save the admitted
local state, operation result and corresponding provenance. It MUST validate on
initial save, replacement and load/restore that original attribution, snapshots,
artifacts, replay results and evidence have not been erased, changed, duplicated
or rebound. New statuses add facts without changing original bytes/anchors.

Check Limits before staging work or invoking effects. maxOperations counts saved
OperationState records; replays count at most that many public successes.
Determinations/notices/appeals each use their named bound. Outstanding work counts
pending/blocked items without a final qualified receipt. Attempts use the bound
below. No history is pruned to make room; reaching a limit refuses new work and
retains recoverable prior evidence. This profile supports one notice and at most
one appeal per determination; new determination firings use new IDs and limits.

### Fixture transport and recovery

```text
PendingNoticeWork = {
  processId:string, noticeId:RecordId, determinationId:RecordId,
  transportKey:Token, correlation:Token, artifactSha256:Digest,
  artifactBytesBase64:string, recipient:string, channel:DeliveryChannel,
  issueAt:Timestamp
}
TransportAcknowledgement = {
  kind:"accepted", noticeId:RecordId, transportKey:Token, correlation:Token,
  artifactSha256:Digest, channel:DeliveryChannel, acceptedAt:Timestamp,
  adapterResultId:Token
}
QualifiedNoticeReceipt = {
  kind:"qualifiedReceipt", noticeId:RecordId, transportKey:Token,
  correlation:Token, artifactSha256:Digest, channel:DeliveryChannel,
  evidenceClass:"fixtureQualifiedReceipt", qualifiedAt:Timestamp,
  adapterResultId:Token
}
TransportFailure = {
  kind:"noEffectFailure", code:"injectedFailure" | "attemptsExhausted"
}
TransportResult =
  { kind:"acceptedOnly", acknowledgement:TransportAcknowledgement } |
  { kind:"qualified", acknowledgement:TransportAcknowledgement,
    receipt:QualifiedNoticeReceipt } |
  { kind:"failed", failure:TransportFailure }
```

`NoticeTransport` MUST expose lookup of the retained result by complete immutable
PendingNoticeWork and execution of that same work. The selected fixture adapter
MUST retain actual results by `(processId,transportKey)` independently of runtime
recreation. Same key with changed bytes, digest, recipient, channel, correlation,
target or issueAt MUST refuse without invocation. A successful repeat recovers
the original accepted/qualified result and timestamps; it does not send again.
The implementation MUST distinguish an absent result from an actual failed
attempt. Failed attempts may be retried under the same immutable work; retained
successful results cannot be replaced by new attempts.

Save notice bytes/pending work before any invoke. Save the incremented attempt
count before invoking a new attempt; recovery lookup does not consume an
attempt. A pre-send save failure MUST produce zero transport calls. Each
noEffectFailure MUST mean the adapter actually performed no external effect; an
uncertain outcome is unsupported and MUST remain unqualified, never rewritten as
failure/receipt. On maxTransportAttempts with no retained success, mark exhausted
without invocation or clock reset. Work remains retained and effect stays blocked.

Transport success before local completion-save failure MUST be recoverable by
lookup after runtime recreation. Receipt admission MUST check exact saved work,
adapter result ID, evidence class, timestamp range and required equality with
issueAt. Save accepted evidence, qualification and pending-work consumption
atomically. Reordered/duplicate callbacks return existing evidence; differing
results for the same completed work refuse. An acknowledgement alone leaves the
notice pending and cannot emit noticeSent or satisfy delivery/grace. A delayed or
mismatched receipt records the anomaly as blocked without a qualification fact.
Retain each distinct actual typed transport result in transportResults, including
accepted-only and rejected results, in first runtime observation order. This is a
distinct-result diagnostic set, not a chronology or attempt audit log. Identical
failed results and recovery lookup of identical results add no duplicate. Saved
attempts count pre-invoke reservations, including crash windows before an actual
invoke, and therefore do not themselves prove an invocation. This diagnostic
array grants no delivery qualification; only the separately admitted receipt
does. Its exact prefix is immutable and its size cannot exceed saved attempts.
This synchronous profile permits only one stable non-failure result per work
key; a changed accepted or qualified result refuses as ReplayConflict, including
an accepted-only to qualified upgrade. Transport ports must return the declared
closed typed result with its complete lexical constraints before native result
admission; malformed wire data or malformed string fields cause a port refusal,
not a qualified result. Work, original dates and effect gates remain unchanged
after a blocked result.
No Scenario request, payload, expected assertion or generic event can construct
these evidence types.

Only an admitted qualified result emits `noticeSent`, with
`data.source:"deterministic"`, machineReadable.kind adverseDecisionNotice,
original snapshot digest, notice ID and actual transport evidence references.
Pending, accepted-only, blocked and exhausted work MUST have distinguishable
native observations and MUST NOT emit that success fact. A configured adapter
that cannot retain and recover original successful results MUST refuse this
profile. These guarantees apply to the selected fixture adapter and one-record
reference store; hosted transactions, SMTP, real recipient authentication and
legal delivery remain unqualified.

### Fixture replay inputs and native selectors

```text
ReferenceReplayAdapters = {
  scope:"softwareFixture", humanActorBindings:HumanActorBinding[],
  trustedReceiptSigners:FixtureReceiptSigner[], noticeTransport:FixtureNoticeTransport,
  retainedEvidence:EvidenceSelection[]
}
EvidenceSelection = {
  id:Id, role:"source" | "review" | "hearingEvidence" | "overrideEvidence",
  bytesBase64:string, sha256:Digest
}
FixtureReceiptSigner = {
  id:Id, suite:"ed25519", kidBase64:string,
  publicKeyBase64:string, methodUri:string
}
FixtureNoticeTransport = {
  id:Id, scope:"softwareFixture", mode:"sameInstantQualifiedReceipt",
  retainedResultsIdentity:Digest,
  failureSchedule:{ attempt:integer >= 1, outcome:"noEffectFailure" }[]
}
NativeSelector = {
  attempt:{ eventIndex:integer >= 0,
    kind:"event" | "taskSubmission" | "governanceOperation" | "clock" | "migration",
    occurrence:integer >= 0 },
  subject:NativeSubject
}
NativeSubject =
  { kind:"determination", bindingRef:Id, firingOccurrence:integer >= 0 } |
  { kind:"decision", transition:TransitionLocation, evaluationOccurrence:integer >= 0 } |
  { kind:"notice", determinationId:RecordId } |
  { kind:"appeal", determinationId:RecordId } |
  { kind:"operation", operationRef:Id, operationOccurrence:integer >= 0 } |
  { kind:"task", taskRef:Id, taskOccurrence:integer >= 0 } |
  { kind:"timer", timerId:Id, timerOccurrence:integer >= 0 } |
  { kind:"override", operationRef:Id, operationOccurrence:integer >= 0 }
```

The additive published entry point is
`prepare_reference_replay(request: ReplayRequest, adapters: ReferenceReplayAdapters)`;
it does not add fields to every existing ReplayRequest literal. EvidenceSelection
IDs MUST be unique, base64 MUST be canonical, and sha256 MUST match the actual
original decoded bytes. SourceRef.sourceVersion and ReviewRef.id resolve to
exact selected evidence IDs of the required role; evidence data is immutable and
included in replay identity. Each evidence item MUST be at most 10485760 bytes.

These adapters are an explicit native replay input, separate from workflow
configuration and event data. All fields, original signer key bytes, failure
schedule, human assignments and retained transport identity MUST bind replay
identity. Arrays may be empty only when the selected workflow does not need the
capability. Any required missing capability refuses. kidBase64 is nonempty
canonical base64, at most 256 decoded bytes; publicKeyBase64 is canonical base64
encoding exactly 32 bytes. methodUri is an absolute registered signature-method
URI accepted by the existing binding. Duplicate signer IDs or kid/method
combinations refuse. Key data configures the maintained
`TrustedReceiptSigner::ed25519` owner; it MUST NOT bypass native signed receipt,
Signature Profile or task-content verification. Synthetic fixture keys prove
only that qualified software path. Human assignments retain Kernel §3.4's exact
shape and immutable process-creation binding; they grant no resource authority.

The notice fixture mode obtains its actual acceptedAt/qualifiedAt from the
shared native clock at first successful execution. It cannot manufacture an
issue-time result after that instant. Retained result storage survives runtime
recreation; a fresh isolated replay may create a new explicitly identified
fixture store. Restoring/retrying one replay must reuse its selected retained
store and evidence identity. failureSchedule applies to actual numbered
invocations; attempts after its listed no-effect failures use the declared mode.
It cannot be changed inside Scenario events.

Native fixture EventEntry has optional `governance_operation:GovernanceRequest`;
Studio ScenarioEvent spells that field `governanceOperation`. Lowering changes
only the field name. It is mutually exclusive with task submission and generic
event data; the outer event label is descriptive and grants no operation. The
request's actor is authoritative only after native host admission; conflicting
outer actor fields refuse. A selected native request reports ReplayAttempt.kind
`governanceOperation`, its exact result and actual drains/provenance. RuntimeSnapshot
exposes the actual saved DueProcessState.

The published native functions are
`resolve_observation(&ReplayResult, &NativeSelector)` and
`compare_observation(&ReplayResult, &NativeSelector, &ObservationExpectation)`.
The resolver returns exactly `{status:"resolved", pointer:string}` or
`{status:"refused",code:"targetRejected"}`; pointer addresses the existing
serialized native result. The comparison returns exactly
`{status:"pass"}` or `{status:"fail",difference:ObservationDifference}` or
`{status:"refused",code:"targetRejected"}`. ObservationExpectation.pointer is
relative to the resolved subject; its existing expected JSON and
ObservationDifference meanings remain unchanged. These existing comparison types
are assertions, never command/evidence inputs. Provenance comparisons MUST call
the existing native partial matcher on the selected attempt's actual records;
Studio MUST NOT implement a second matcher or evaluate lifecycle behavior.

Selector occurrences are zero-based in native observation order within the
specified attempt. Each selector MUST resolve exactly one observed subject.
Missing subjects, ambiguous attempts/refs, an absent saved state or a guard test
masquerading as a firing MUST refuse. Expected values are compared using the
existing exact JSON-pointer or provenance partial-matching authorities; selectors
never populate evidence. Decision observations distinguish tested guards from
selected transitions. Notice, appeal, held hearing and override assertions name
actual native state/results, not event/result labels.

### Closed refusal matrix

`RefusalCode` is exactly the code column below. A refusal MUST retain its native
attempt/result for inspection while preserving the no-success/no-effects rules.

| Code | Required trigger |
|---|---|
| `unsupportedProfile` | Unsupported timing/evidence/anchor, provider, scoped policy, instance/context, deadline purpose or execution family. |
| `invalidConfiguration` | Unknown fields, wrong type/range, unresolved/duplicate/overlapping config references, conflicting selected policy or required source/review bytes missing. |
| `unboundTransition` | Actual tagged determination/applied-state/service effect lacks its exact supported binding. |
| `actorRejected` | Missing/current unauthorized actor, foreign binding, standing failure, nonhuman review or canonical independence violation. |
| `targetRejected` | Missing, ambiguous, stale or wrong-process saved determination/notice/appeal. |
| `selectionRejected` | False guard, competing winner, wrong source state or actual selected location differs from bound operation. |
| `invocationRequired` | Generic dispatch attempts a command-only operation. |
| `replayConflict` | Used public token or immutable transport key with changed content/actor/target. |
| `signatureRejected` | Absent, unverified, deferred, untrusted or mismatched actual task/signature/content. |
| `explanationRejected` | Missing causal row reasoning, failed/missing positive/negative alternatives or unsupported native evaluation. |
| `noticePending` | Receipt absent, accepted-only, wrong class/bytes/correlation or delayed anchor mismatch. |
| `clockRejected` | Invalid/future/unrepresentable timestamp, overflow, early effect, late appeal or missed selected held-hearing deadline. |
| `continuationFrozen` | Adverse effect would change protected service before permitted continuation release. |
| `operationStateRejected` | Duplicate notice/appeal/effect, missing schedule/held milestone, reversal, wrong reviewer or incompatible current operation state. |
| `overrideRejected` | Missing authority/rationale/evidence, unrelated timer/state, cumulative cap exceeded or attempt to shorten other rights. |
| `limitReached` | Selected collection/bytes/outstanding-work/attempt bound would be exceeded. |
| `storeRejected` | Atomic save/load/restore validation fails; no successful result or unpersisted dispatch may be claimed. |

## Composition

The profile implements Governance §3 through lifecycleHook and provenanceLayer,
using existing native lifecycle evaluation, Signature Profile, Formspec processing,
actor admission, Delivery templates, OverrideRecord and store owners. Other
review protocols, temporal policy tables, delegation, compensation, external
services and unsupported causal evidence remain refused. Changing impact level,
disabling required switches or supplying expected outcomes does not qualify them.

The retained illustrative sources define selected software-fixture policy only.
This profile MUST preserve their exact bytes and review limitations. It does not
approve source truth, publication, substantive language equivalence, accessibility,
legal sufficiency, production transport or hosted durability. Timing/evidence
qualification remains explicitly related to the owning fs-hltb review; fixture
implementation cannot close unrelated qualification obligations.

## Conformance

Owning schemas/types MUST agree with these closed shapes before activation.
Native admission MUST reject unsupported combinations rather than drop fields.
Focused tests MUST prove selected Definition/Mapping causality; changing time
constraints under the moving native clock; duplicate-shaped/indexed and repeated
firings; false/competing guards; all firing paths; no effect on refused preflight;
actual verified signature/task binding; causal individualized/positive/negative
explanation; deterministic localized artifact; accepted-only/delayed/forged
receipts; save/invoke/crash/restore boundaries; retained transport result recovery;
clock boundaries; canonical-human independence; equivalent filing channels;
timely schedule with missing/late held hearing; separate disposition; continuation
and explicit household release; duplicate effect; cumulative native override;
limits; exact replay results; and missing/ambiguous native selectors.

Deletion controls MUST fail when the owning validator, mapper, selector capture,
preflight, signature verification, native table evaluation, receipt qualification,
held-hearing operation or independent semantic comparison is omitted. Reference
store/fixture transport tests prove only those actual selected implementations.
Hosted/provider/legal/product qualification requires separately owned evidence.
