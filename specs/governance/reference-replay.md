# Native reference replay recipes and comparisons

This profile completes the authored software-fixture route defined by
[reference-execution](reference-execution.md). It introduces no hosted, source-law,
publication, substantive-language or accessibility certification.

A canonical custody declaration with no behavior, exactly `{}` or
`{anchorRequirements:[]}`, is admissible and retains its original bytes and
identity. It claims no trust profile, anchor or export execution. Any nonempty
anchor requirement, trustProfileRef, exportBundleRef or other custody field
remains unsupported and refuses before activation.

## Normative Contract

### Execution-time references and signing

All objects below are closed. Existing Id, Scalar, SourceRef, ReviewRef, DayDuration,
GovernanceRequest and GovernanceCommand retain their owning meanings. Optional
fields omit their value; explicit null refuses.

```text
DeterminationTarget = {bindingRef:Id, firingOccurrence:integer>=0}
GovernanceRecipe = {
  operationRef:Id, target?:DeterminationTarget, idempotencyToken:Token,
  actor:GovernanceRequest.actor, command:GovernanceRecipeCommand
}
GovernanceRecipeCommand =
  {kind:"issueNotice"} |
  {kind:"fileAppeal", channel:Channel,
    continuationChoice:"elect"|"waive"|"unspecified"} |
  {kind:"scheduleHearing", reviewerActor:ActorRef,
    calendarDaysFromFiling:integer>=0 and <=36500} |
  {kind:"holdHearing", evidenceRefs:SourceRef[]} |
  {kind:"recordDisposition", outcome:"upheld"|"reversed",
    rationale:string,evidenceRefs:SourceRef[]} |
  {kind:"requestServiceTermination",rationale:string} |
  {kind:"extendPreNoticeWindow",calendarDays:Days,
    rationale:string,evidenceRefs:SourceRef[]} |
  {kind:"applyAdverseEffect"}
DocumentSigningKey = {
  id:Id,taskRef:Id,signerHuman:Human,kidBase64:string,
  publicKeyBase64:string,methodUri:"urn:formspec:sig-method:ed25519-cose-sign1@1"
}
FixtureSigningCapability = {
  id:Id,scope:"softwareFixture",documentKeyRef:Id,pkcs8Base64:string,
  receiptSignerRef:Id,receiptPkcs8Base64:string
}
SignatureRecipe = {
  target:DeterminationTarget,signingCapabilityRef:Id,
  signature:FormspecAuthoredSignatureTemplate
}
```

A native EventEntry MAY contain governance_recipe, mutually exclusive with data,
task_submission, governance_operation and migration. Its outer label grants no
capability. Native replay resolves target against actual prior saved determinations
in native order, verifies the selected operation's determination binding, and
constructs the exact GovernanceRequest. Extension alone omits target. An appeal
command resolves the one actual saved appeal under that determination; absent or
ambiguous targets refuse. Scheduling adds the selected whole UTC days to actual
filedAt using checked native time arithmetic. The native governor independently
admits and executes the resolved command normally. Retain original recipe,
resolved request, native result and observations; recipes never create facts.

A TaskSubmission MAY contain signature_recipe. Its original Response remains an
actual complete Formspec Response. The following fields MUST initially be absent:
data.determinationId, data.determinationSnapshotSha256,
data.determinationFiringRecordId, data.signatureTaskId and authoredSignatures. Native replay resolves the actual active task using the
existing taskRef route and target using actual saved state. It fills those four
reserved association fields with actual determination/snapshot/firing/task IDs;
The normal binding cross-checks signed data.signatureTaskId against its actual
ActiveTask and supplies that native taskId as SignatureEvidence.source_response_ref
to the unchanged Signature Profile owner. No top-level Response sourceResponseRef
is added; the closed owning Response schema remains unchanged. These fields are part of the complete
Response signed-payload bytes, except authoredSignatures itself as required by the
maintained canonical profile. Existing Formspec signed-payload and static
SignatureProfile documentHash meanings remain unchanged.

FormspecAuthoredSignatureTemplate is the maintained Formspec authored-signature
shape without signatureValue, verificationReceipt or signedPayload. It MUST carry
explicit consentAccepted:true, consent reference/version, affirmationText,
signingIntent, signatureId, documentId/documentHash/algorithm, signatureProvider,
ceremonyId and canonical signerId. Consent/action and source identity come from
these reviewed fixture inputs; native execution does not invent human consent.
Unknown/unsupported fields refuse. SignatureRecipe cannot override reserved
association fields or insert expected cryptographic verdicts.

Before canonicalization, native replay takes signedPayload.signedAt from its
admitted shared clock, fills all required normal SignedPayload metadata and
mirrors the explicit signing intent. Where the selected Definition requires
SigningContentCommitment, the recipe MUST supply its original draft/revision,
explicit actor/name/consent and exact selected Definition content. Native lowering
resolves that pinned content and sets the commitment's actual native signing time
before constructing canonical signed bytes. The maintained owner checks every
mirrored content/document/consent field. Unsupported section signatures or missing
required commitments refuse. No new signing profile or mutable static
SignatureProfile documentHash is introduced.

Native replay checks the fixture private keys match their selected public keys,
method and protected kid, resolves the exact selected task and canonical human,
uses the maintained canonical payload builder and Ed25519 COSE primitive, and
verifies the exact actual bytes through the maintained RingVerifier. Receipt keys
and source document keys remain separate selected trust roles. It constructs the
normal authored signature and signed receipt, then submits the resulting complete
Response through the normal Formspec binding and Signature Profile. Its successful
receipt cannot authenticate a different Response: the governor independently
reverifies actual signed bytes and all four data association fields at issuance
and after restore. Actual task creation, accepted artifact, normal affirmation,
canonical human and sourceSignatureId/sourceResponseRef must agree uniquely.

ReferenceReplayAdapters additionally requires documentSigningKeys and
fixtureSigningCapabilities arrays (empty only when unused). All canonical key
bytes, private fixture capabilities, templates and explicit choices bind replay
identity; these are synthetic local software capabilities, never production keys.
No key is selected from generic event data. Retain original recipe, resolved
unsigned and signed Response, controlled time, method/key identity, actual receipt
and task result. A missing/ambiguous/stale target or key, changed task/content,
transplanted receipt/COSE or conflicting actor refuses before task acceptance.

The adapters' templates use the maintained NotificationTemplateDocument DTO,
including its optional nullable metadata and named semantic templates. This input
is not a canonical Delivery document or proof of executing a Delivery envelope.
Unknown fields and nonempty template extensions refuse in this bounded profile;
native template validation still owns content, locale and variable checks.

NativeSubject notice/appeal MAY use target:DeterminationTarget instead of
actual determinationId; these alternatives are mutually exclusive. Resolution
uses actual native saved state and refuses absent/ambiguous matches. Stable
references do not predict record IDs and do not interpolate authored strings.

### Explicit fixture service and policy invocation

Canonical workflow bindings remains OutputBinding[] and is not reinterpreted as
a legacy named integration map. The existing extensions seam admits a distinct
selected declaration extensions.x-wos-reference-invocations:

```text
ReferenceInvocations = {
  profile:"referenceInvocations/1",bindings:ReferenceInvocationBinding[]
}
ReferenceInvocationBinding = {
  id:Id,kind:"requestResponse"|"opaPolicy",
  descriptor:{kind:"openapi",documentRef:string,operationRef:string} |
             {kind:"opa",endpoint:string},
  inputMapping:{[name:string]:string}, outputBinding:{[casePath:string]:string}
}
FixtureInvocation = {
  id:Id,serviceRef:Id,expectedInput:JSON,response:JSON,
  maxInvocations:integer>=1 and <=1000
}
```

Bindings are immutable selected workflow authority; FixtureInvocation entries are
separate native adapters, included in ReferenceReplayAdapters and replay identity.
They supply explicit synthetic external responses and expected actual request
bytes; they never grant a workflow action or bypass its guard. Every invoked
serviceRef must resolve exactly one selected declaration and one fixture input.
Native prep validates unique IDs, descriptor/kind agreement, all reachable and
unreachable invokeService actions, the maintained FEL inputMapping and supported
outputBinding JSONPath syntax. Unused/missing/ambiguous fixture entries refuse.
Actual inputs must equal the selected expectedInput exactly; mismatches refuse
without an output. Counts are bounded and actual invocation records retain input,
response and fixture identity. Identity-bound canned service_response cannot mix
with these selected adapters.

The maintained invocation owner MUST perform actual input/binding/fixture identity
checks before both provider invocation and persisted StepResult reuse. Qualified
StepResults retain their complete actual request, actual selected binding and
adapter identity. Reuse requires exact association equality and adapter validation
of the stored actual response. A preexisting result cannot bypass selected request
or response checks merely because serviceRef and idempotencyKey match.

Native replay lowers this closed declaration to the maintained request-response
or policy-engine runtime handler and passes a real ExternalService implementation
through InvokeServices. For OPA it supplies engineType:opa and the actual native
handler normalizes result:boolean, reasons and obligations. The handler's actual
StepResult, PolicyDecision and mapped case values remain observations. Native
replay must refuse incompatible source decision vocabulary; it cannot rename
permit/allow or invent response facts. Selected fixture input and response values
are visibly software-fixture data, with no actual OpenAPI/OPA endpoint invocation.
Original descriptor references and authored mappings remain retained.

This route does not implement ADR0080 canonical OutputBinding. Such bindings stay
explicitly refused. A configured invocation output may not target a due-process
protected service pointer, its ancestor or descendant; refuse before any provider
call. All selected invokeService actions remain subject to governance preflight.
No invocation or expected result may be replaced by an event with the same name.

### Reference observation and declared expectations

The additive reference route distinguishes absent expected_transitions from an
explicit empty array in the original retained fixture. Absence requests no
transition-order assertion; explicit [] asserts zero transitions. Every declared
nonempty transition, provenance, error and observation expectation remains active.
The existing generic prepare_replay route retains its existing behavior. This
presence rule does not waive native errors or refusals and does not classify a
cohort member: the metric independently requires successful complete execution
and exactly one declared favorable/adverse final outcome. Original presence and
exact bytes remain part of preparation identity.

### Clock and paired approval comparison

JSONScalar is an ordinary JSON string, boolean, or finite number. It is the
original case-file value, distinct from the tagged Scalar used by due-process
CaseAlternative declarations.

NativeTimeAdvance is {beforeEventIndex:integer>=0,duration:DayDuration}. Each
unique in-range declaration lowers natively to the actual event delay; an existing
competing delay refuses. Retain original declarations and exact lowered bytes in
identity. Existing general native sub-day delay behavior remains unchanged.

PairedApprovalRateRequest contains exactly 100 explicit pairs/200 unique member
IDs, protectedCasePointer, distinct leftValue/rightValue JSON scalars, selected source
thresholdPercentagePoints, sourceRefs, review, favorableStatePointer and
adverseStatePointer. Each member contains its complete ReplayRequest and
ReferenceReplayAdapters. Across all 200 members the selected candidate, dependencies and native runtime
configuration MUST agree exactly. Within each pair all original inputs/recipes/dependencies,
adapters and expectations must be identical except that one initial case scalar.
No generated actual IDs are compared as original inputs. Native replay runs each
member sequentially and retains every complete result. Each member must execute
without error/refusal/failed submission and finish in exactly one selected
favorable or adverse final state. Incomplete journeys, third outcomes and absent
or ambiguous state ownership refuse the metric; they cannot count adverse.

Only the selected favorable terminal outcome counts approved. Denominators remain
100 on each side. Compare abs(approvedLeft-approvedRight) strictly less than the
reviewed threshold in percentage points using integer/rational arithmetic. For
the maintained source's five-percentage-point threshold, five fails and four
passes. No threshold relaxation or sample shrinkage is permitted. Return all
200 native results and observed classifications. This is bounded software-fixture
evidence, not a population fairness or legal-compliance claim.

The selected metric review is a closed MetricReview:

```text
MetricReview = {
  scope:"softwareFixture",decision:"accepted"|"rejected"|"inconclusive",
  reviewer:EngineeringReviewer,reviewedAt:Timestamp,rationale:string,
  descriptor:MetricDescriptor
}
MetricDescriptor = {
  metric:"absoluteApprovalRateDifference",operator:"lt",
  thresholdPercentagePoints:integer>=1 and <=100,protectedCasePointer:Pointer,
  leftValue:JSONScalar,rightValue:JSONScalar,favorableStatePointer:Pointer,
  adverseStatePointer:Pointer,sourceRefs:SourceRef[],reviewId:Id,
  members:{id:Id,independentInputSha256:Digest}[200]
}
```

Native comparison recomputes the full descriptor and requires exact equality with
the retained selected review, an accepted decision, nonblank concrete rationale,
honest engineering model attribution and admitted canonical review time. Each
member digest covers its complete original independent request, parsed fixture,
adapters and expectations; it excludes only that exact selected metric review's
own bytes and digest to remove the cycle. No other source or adapter is omitted.
A changed threshold, outcome, protected value, member or source requires a new
applicable review. Review bytes alone are insufficient authority for a different
metric. A model engineering review grants no human/agency/legal qualification.

### Review the same retained execution

A RenderedNoticeReviewAssertion names the actual NativeSelector, full notice
artifact digest, all actual locale/body digests, nonempty requiredClaims set
(substantiveLanguageEquivalence, plainLanguage, section508Review), and retained
review bytes/digest. Its closed review document includes those exact digests,
claims, canonical reviewer and reviewedAt. Native comparison requires exact
coverage of every actual variant once and all required claims. It checks the
review time follows artifact availability and is not future relative to its
explicit admitted comparisonClockMs. Structured content/locale checks remain
separate actual observations. Native comparison does not manufacture or approve
human quality judgments.

An explicit engineeringFixture review is a separate alternative:

```text
EngineeringRenderedReview = {
  scope:"engineeringFixture",reviewer:{kind:"model",id:Id,method:string},
  reviewedAt:Timestamp,artifactSha256:Digest,
  variants:{locale:string,bodySha256:Digest}[],
  claims:("bilingualStructuredFacts"|"plainLanguageEngineeringReview"|
          "accessibilityStructureReview")[],
  observations:{[claim]:{verdict:"supported"|"refuted"|"inconclusive",observations:string}},
  limitations:string[]
}
```

Its nonempty requiredClaims MUST use that separate engineering set and be covered
exactly. Each required claim has one explicit structured verdict with nonblank
concrete observations; supported may pass, refuted fails and inconclusive refuses.
A nonblank paragraph or copied pass flag cannot imply a favorable judgment. Limitations MUST be nonempty and retain the original broad qualifications
as unqualified. Result scope is engineeringFixture with honestly attributed model
reviewer and exact content hashes; it MUST return original human language,
Section 508, legal-sufficiency and publication claims as unqualified. Mechanical
locale/content observations stay separate native comparisons. This alternative
cannot pass, replace or relabel the original broad human/FNS approval assertions;
any source correction to executable engineering assertions remains review-visible
with original assertions/history retained. It proves only those explicit fixture
engineering checks, not substantive human approval.

The additive staged owner retains a NativeReplaySession containing the original
PreparedReplay and immutable actual ReplayResult. begin_reference_replay executes
once and returns that session plus observable artifacts. compare_rendered_notice
accepts that exact session, a freshly prepared current request/adapters and the
explicit retained review assertion. It requires the current prepared identity and
runtime identity equal the original ones before checking the stored actual result.
It does not replay or replace the actual artifacts. A changed source, request,
selected dependency, adapter or native source fingerprint refuses stale-session
comparison. The session is an opaque native object, never accepted from authored
JSON and never reconstructed from a claimed pass. Studio may retain its native
session handle while attaching a review, then complete comparison on that same
session. A new process without the original session must execute a new run and
obtain review of its own artifacts. Native replay facts and post-run comparison
facts retain distinct identities and clocks.

## Composition

Recipes resolve dynamic native facts through existing task/lifecycle/governance
owners. Invocation fixtures attach below the existing service port. Comparisons
read immutable actual results and never grant commands or evidence. Legal source,
publication, real transport and substantive review limits remain explicit.

## Conformance

Causal controls must reject missing/ambiguous targets; altered signed association
fields; valid receipts/COSE transplanted across Response/task/determination;
request/result mismatch; service output targeting protected values; unsupported
OPA vocabulary; incomplete cohort members; exact threshold equality; stale or
mutated rendered variants; missing required review claims; future reviews; and
changed native/source/adapter identity on a retained session. Positive controls
must execute the actual native primitives and preserve complete reports.
