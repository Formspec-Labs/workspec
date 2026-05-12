# D26 Registry Agreement Audit — Task 5.1

**Date:** 2026-05-12
**Scope:** Read-only enumeration of `schemas/record-kind-registry.json` (132 kinds) against the
`schemas/wos-provenance-log.schema.json` overlay and `schemas/api/provenance.schema.json` API view.
**Pre-release framing:** No registry-version bump, no deprecation period. This audit precedes
Task 5.2 (inner `recordKind` drop), which depends on this enumeration.

## Summary

| Metric | Value |
|---|---|
| Total kinds enumerated | **132** |
| Schema-validated | 22 |
| With `eventLiteral` | **14 / 132** (10.6%); **14 / 22** (63.6% of schema-validated) |
| Overlay `event` const present | **14 / 132** (10.6%) |
| Overlay ↔ registry agreement (where both exist) | **14 / 14 = 100%** |
| API-view (`api/provenance.schema.json`) const agreement | **14 / 14 = 100%** (lines 1158–1340) |
| Mismatches found | **0** |
| Schema-validated kinds missing `eventLiteral` | **11** (see "Gaps to fix in 5.2") |
| Flat (non-validated) kinds without `eventLiteral` | **110** (correct posture; flat Facts-tier records do not require dispatch keys) |

**Verdict:** Every `eventLiteral` declared in the registry has a matching `event` const in both the
provenance-log overlay and the API view. There are **zero mismatches**. The audit is green for the
inner-`recordKind` drop in 5.2 for the 14 kinds that have ratified event literals. The 11
schema-validated-but-eventLiteral-less kinds are a separate, pre-existing gap (no F-13 event literal
ratified yet) and must be resolved before 5.2 can drop `recordKind` for those kinds without losing
their dispatch identity.

## Sources

- Registry: `work-spec/schemas/record-kind-registry.json` (206 lines; v1.0.0; lastAudited 2026-05-07)
- Overlay: `work-spec/schemas/wos-provenance-log.schema.json` (1797 lines)
- API view: `work-spec/schemas/api/provenance.schema.json` (2520 lines)
- Governance policy doc (`specs/provenance-registry.md`): not present at audit time; cited by
  registry `description` but absent from `work-spec/specs/`.

## Method

1. Loaded all 132 entries from `record-kind-registry.json#/recordKinds`.
2. Scanned `wos-provenance-log.schema.json#/$defs/*` for every `recordKind`/`event` const pair.
3. Cross-referenced the 14 `eventLiteral` declarations in the registry against the 14 overlay
   `$defs` and the 14 API-view if/then guards (`api/provenance.schema.json` lines 1149–1346).
4. Recorded `kind | eventLiteral | overlay_const | agreement` per row.

## Full enumeration (132 rows)

| # | kind | category | sv | eventLiteral | overlay $def line | agreement |
|---|------|----------|----|--------------|---------------------|-----------|
| 1 | `stateTransition` | foundation | ✓ | — | — | gap (sv, no eventLiteral) |
| 2 | `unmatchedEvent` | foundation |  | — | — | flat |
| 3 | `caseStateMutation` | foundation |  | — | — | flat |
| 4 | `caseCreated` | foundation | ✓ | `wos.kernel.case_created` | 104 | **MATCH** |
| 5 | `intakeAccepted` | foundation | ✓ | `wos.kernel.intake_accepted` | 183 | **MATCH** |
| 6 | `intakeRejected` | foundation | ✓ | `wos.kernel.intake_rejected` | 265 | **MATCH** |
| 7 | `intakeDeferred` | foundation | ✓ | `wos.kernel.intake_deferred` | 337 | **MATCH** |
| 8 | `timerCreated` | lifecycle |  | — | — | flat |
| 9 | `timerFired` | lifecycle |  | — | — | flat |
| 10 | `forEachIterationStarted` | lifecycle |  | `wos.kernel.for_each_iteration_started` | 571 | **MATCH** |
| 11 | `forEachIterationCompleted` | lifecycle |  | `wos.kernel.for_each_iteration_completed` | 599 | **MATCH** |
| 12 | `forEachCompleted` | lifecycle |  | `wos.kernel.for_each_completed` | 627 | **MATCH** |
| 13 | `timerCancelled` | lifecycle |  | — | — | flat |
| 14 | `onEntry` | internal |  | — | — | flat |
| 15 | `onExit` | internal |  | — | — | flat |
| 16 | `actionExecuted` | internal |  | — | — | flat |
| 17 | `invalidDuration` | internal |  | — | — | flat |
| 18 | `toleranceViolation` | internal |  | — | — | flat |
| 19 | `convergenceCapReached` | internal |  | — | — | flat |
| 20 | `capabilityInvocation` | ai | ✓ | — | — | gap (sv, no eventLiteral) |
| 21 | `deonticViolation` | ai |  | — | — | flat |
| 22 | `deonticEvaluation` | ai |  | — | — | flat |
| 23 | `deonticResolution` | ai |  | — | — | flat |
| 24 | `deonticBypass` | ai |  | — | — | flat |
| 25 | `rightsViolation` | ai |  | — | — | flat |
| 26 | `consistencyViolation` | ai |  | — | — | flat |
| 27 | `autonomyViolation` | ai |  | — | — | flat |
| 28 | `autonomyCapped` | ai |  | — | — | flat |
| 29 | `autonomyComputed` | ai |  | — | — | flat |
| 30 | `humanTaskCreated` | ai |  | — | — | flat |
| 31 | `toolViolation` | ai |  | — | — | flat |
| 32 | `escalationPending` | ai |  | — | — | flat |
| 33 | `autonomyDemotion` | ai |  | — | — | flat |
| 34 | `autonomyEscalation` | ai |  | — | — | flat |
| 35 | `confidenceViolation` | ai |  | — | — | flat |
| 36 | `confidenceDecay` | ai |  | — | — | flat |
| 37 | `cumulativeConfidenceViolation` | ai |  | — | — | flat |
| 38 | `sessionPaused` | ai |  | — | — | flat |
| 39 | `groundTruthLabel` | ai |  | — | — | flat |
| 40 | `agentOutput` | ai |  | — | — | flat |
| 41 | `actorTypeViolation` | ai |  | — | — | flat |
| 42 | `agentProvenanceAnnotation` | ai |  | — | — | flat |
| 43 | `agentVersionChange` | ai |  | — | — | flat |
| 44 | `narrativeTierRecorded` | ai |  | — | — | flat |
| 45 | `constraintTamperBlocked` | ai |  | — | — | flat |
| 46 | `driftReclassification` | ai |  | — | — | flat |
| 47 | `agentStateTransition` | ai |  | — | — | flat |
| 48 | `proxyInvocation` | ai |  | — | — | flat |
| 49 | `dispositiveViolation` | ai |  | — | — | flat |
| 50 | `fallbackTriggered` | ai |  | — | — | flat |
| 51 | `fallbackAttempt` | ai |  | — | — | flat |
| 52 | `fallbackTerminal` | ai |  | — | — | flat |
| 53 | `noticeSent` | governance |  | — | — | flat |
| 54 | `separationViolation` | governance |  | — | — | flat |
| 55 | `appealFiled` | governance |  | — | — | flat |
| 56 | `protocolViolation` | governance |  | — | — | flat |
| 57 | `independentFirstEnforced` | governance |  | — | — | flat |
| 58 | `samplingDecision` | governance |  | — | — | flat |
| 59 | `overrideViolation` | governance |  | — | — | flat |
| 60 | `overrideRecorded` | governance |  | — | — | flat |
| 61 | `legalHoldPlaced` | governance |  | — | — | flat |
| 62 | `legalHoldReleased` | governance |  | — | — | flat |
| 63 | `legalHoldDestructionRejected` | governance |  | — | — | flat |
| 64 | `continuationOfServicesActivated` | governance |  | — | — | flat |
| 65 | `pipelineStageCompleted` | governance |  | — | — | flat |
| 66 | `pipelineRiskProfile` | governance |  | — | — | flat |
| 67 | `pipelineRejection` | governance |  | — | — | flat |
| 68 | `taskCreated` | governance |  | — | — | flat |
| 69 | `taskPresented` | governance |  | — | — | flat |
| 70 | `taskDismissed` | governance |  | — | — | flat |
| 71 | `taskDraftPersisted` | governance |  | — | — | flat |
| 72 | `taskResponseSubmitted` | governance |  | — | — | flat |
| 73 | `taskResponseRejected` | governance |  | — | — | flat |
| 74 | `dataMapping` | governance |  | — | — | flat |
| 75 | `taskCompleted` | governance |  | — | — | flat |
| 76 | `taskFailed` | governance |  | — | — | flat |
| 77 | `taskSkipped` | governance |  | — | — | flat |
| 78 | `parameterResolved` | governance |  | — | — | flat |
| 79 | `compensationLogEntry` | compensation |  | — | — | flat |
| 80 | `compensationExecuted` | compensation |  | — | — | flat |
| 81 | `compensationScopeBoundary` | compensation |  | — | — | flat |
| 82 | `delegationViolation` | delegation |  | — | — | flat |
| 83 | `instanceSuspended` | lifecycle |  | — | — | flat |
| 84 | `instanceResumed` | lifecycle |  | — | — | flat |
| 85 | `instanceTerminated` | lifecycle |  | — | — | flat |
| 86 | `stepResultPersisted` | lifecycle |  | — | — | flat |
| 87 | `idempotencyDedup` | lifecycle |  | — | — | flat |
| 88 | `instanceMigrated` | lifecycle |  | — | — | flat |
| 89 | `contractValidation` | lifecycle |  | — | — | flat |
| 90 | `historyCleared` | internal |  | — | — | flat |
| 91 | `dcrActivityExecuted` | advanced |  | — | — | flat |
| 92 | `dcrRelationEvaluated` | advanced |  | — | — | flat |
| 93 | `dcrResolutionError` | advanced |  | — | — | flat |
| 94 | `zoneSatisfied` | advanced |  | — | — | flat |
| 95 | `dcrZoneViolation` | advanced |  | — | — | flat |
| 96 | `equityAlert` | advanced |  | — | — | flat |
| 97 | `circuitBreakerTripped` | advanced |  | — | — | flat |
| 98 | `circuitBreakerReset` | advanced |  | — | — | flat |
| 99 | `shadowModeDivergence` | advanced |  | — | — | flat |
| 100 | `driftAlert` | advanced |  | — | — | flat |
| 101 | `verificationReportProduced` | advanced |  | — | — | flat |
| 102 | `immutabilityViolation` | advanced |  | — | — | flat |
| 103 | `activationBlocked` | advanced |  | — | — | flat |
| 104 | `calendarIgnored` | sidecar |  | — | — | flat |
| 105 | `notificationSuppressed` | sidecar |  | — | — | flat |
| 106 | `reportTimedOut` | sidecar |  | — | — | flat |
| 107 | `configurationWarning` | config |  | — | — | flat |
| 108 | `relationshipChanged` | internal |  | — | — | flat |
| 109 | `milestoneFired` | foundation |  | — | — | flat |
| 110 | `eventEmitted` | integration |  | — | — | flat |
| 111 | `eventConsumed` | integration |  | — | — | flat |
| 112 | `callbackReceived` | integration |  | — | — | flat |
| 113 | `callbackPending` | integration |  | — | — | flat |
| 114 | `arazzoStep` | integration |  | — | — | flat |
| 115 | `toolInvoked` | integration |  | — | — | flat |
| 116 | `policyDecision` | integration |  | — | — | flat |
| 117 | `signatureAffirmation` | signature | ✓ | `wos.kernel.signature_affirmation` | 655 | **MATCH** |
| 118 | `signatureAdmissionFailed` | signature | ✓ | `wos.kernel.signature_admission_failed` | 800 | **MATCH** |
| 119 | `correctionAuthorized` | amendment | ✓ | — | — | gap (sv, no eventLiteral) |
| 120 | `amendmentAuthorized` | amendment | ✓ | — | — | gap (sv, no eventLiteral) |
| 121 | `determinationAmended` | amendment | ✓ | — | — | gap (sv, no eventLiteral) |
| 122 | `rescissionAuthorized` | amendment | ✓ | — | — | gap (sv, no eventLiteral) |
| 123 | `determinationRescinded` | amendment | ✓ | `wos.governance.determination_rescinded` | 905 | **MATCH** |
| 124 | `reinstated` | amendment | ✓ | `wos.governance.reinstated` | 933 | **MATCH** |
| 125 | `authorizationAttestation` | amendment | ✓ | — | — | gap (sv, no eventLiteral) |
| 126 | `clockStarted` | clock | ✓ | `wos.governance.clock_started` | 961 | **MATCH** |
| 127 | `clockResolved` | clock | ✓ | `wos.governance.clock_resolved` | 989 | **MATCH** |
| 128 | `identityAttestation` | identity | ✓ | `wos.assurance.identity_attestation` | 1017 | **MATCH** |
| 129 | `clockSkewObserved` | clock | ✓ | — | — | gap (sv, no eventLiteral) |
| 130 | `commitAttemptFailure` | failure | ✓ | — | — | gap (sv, no eventLiteral) |
| 131 | `authorizationRejected` | failure | ✓ | — | — | gap (sv, no eventLiteral) |
| 132 | `migrationPinChanged` | migration | ✓ | — | — | gap (sv, no eventLiteral) |

Legend:
- **sv** = `schemaValidated: true` in the registry.
- **MATCH** = `eventLiteral` and overlay `event` const are present and identical (and the API view
  carries the same const at the cited line in `api/provenance.schema.json`).
- **gap (sv, no eventLiteral)** = schema-validated kind missing an F-13 event literal in the
  registry; no overlay or API-view event const exists. This is a pre-existing condition.
- **flat** = non-schema-validated Facts-tier kind, dispatched on inner `recordKind` only.

## Gaps to fix in 5.2 (or before 5.2 can proceed on these kinds)

5.2 plans to drop the inner `recordKind` field outright (pre-release, no deprecation). Doing so on a
kind without a ratified outer `event` literal would erase the kind's only dispatch identity. The
following 11 kinds therefore block 5.2 unless one of two paths is taken per kind: (a) ratify an
`eventLiteral` and add an overlay `$def` guard + API-view if/then, or (b) leave inner `recordKind`
in place for these specific kinds (contradicting 5.2's "atomic drop" framing). Recommendation: (a)
on all 11, since 5.2 is gated on full atomic removal.

The 11 schema-validated kinds without an `eventLiteral`:

| # | kind | registry line | category | spec reference | required action |
|---|------|---------------|----------|----------------|-----------------|
| 1 | `stateTransition` | `record-kind-registry.json:73` | foundation | Kernel | Add `eventLiteral` (e.g. `wos.kernel.state_transition`) + overlay `$def` guard in `wos-provenance-log.schema.json` + API-view if/then in `api/provenance.schema.json`. |
| 2 | `capabilityInvocation` | `record-kind-registry.json:92` | ai | AI §3.3.1 | Add `eventLiteral` (e.g. `wos.ai.capability_invocation`) + overlay guard + API guard. |
| 3 | `correctionAuthorized` | `record-kind-registry.json:191` | amendment | ADR 0066 §1 | Add `eventLiteral` (e.g. `wos.governance.correction_authorized`) + overlay guard + API guard. |
| 4 | `amendmentAuthorized` | `record-kind-registry.json:192` | amendment | ADR 0066 §2 | Add `eventLiteral` (e.g. `wos.governance.amendment_authorized`) + overlay guard + API guard. |
| 5 | `determinationAmended` | `record-kind-registry.json:193` | amendment | ADR 0066 §2 | Add `eventLiteral` (e.g. `wos.governance.determination_amended`) + overlay guard + API guard. |
| 6 | `rescissionAuthorized` | `record-kind-registry.json:194` | amendment | ADR 0066 §3 | Add `eventLiteral` (e.g. `wos.governance.rescission_authorized`) + overlay guard + API guard. |
| 7 | `authorizationAttestation` | `record-kind-registry.json:197` | amendment | ADR 0066 §5 | Add `eventLiteral` (e.g. `wos.governance.authorization_attestation`) + overlay guard + API guard. |
| 8 | `clockSkewObserved` | `record-kind-registry.json:201` | clock | ADR 0069 §3 | Add `eventLiteral` (e.g. `wos.governance.clock_skew_observed`) + overlay guard + API guard. |
| 9 | `commitAttemptFailure` | `record-kind-registry.json:202` | failure | ADR 0070 §2 | Add `eventLiteral` (e.g. `wos.kernel.commit_attempt_failure`) + overlay guard + API guard. |
| 10 | `authorizationRejected` | `record-kind-registry.json:203` | failure | ADR 0070 §4 | Add `eventLiteral` (e.g. `wos.governance.authorization_rejected`) + overlay guard + API guard. Note: API view already references a typed `authorizationRejectedRecord` payload (`api/provenance.schema.json:1117`) discriminated on `recordKind`; that discriminator path must move to `event` in 5.2. |
| 11 | `migrationPinChanged` | `record-kind-registry.json:204` | migration | ADR 0071 §3 | Add `eventLiteral` (e.g. `wos.kernel.migration_pin_changed`) + overlay guard + API guard. |

Namespacing recommendations (kernel vs governance vs ai vs assurance) above are conservative
suggestions drawn from the existing 14 ratified literals; the actual namespace choice per kind is
ADR-territory and not load-bearing for this audit. The audit's load-bearing claim is the **list of
11**, not the namespace per row.

## What is NOT in this audit (deferred to 5.2)

- No schema edits. Files in `work-spec/schemas/` are read-only for this task.
- No inner `recordKind` field removal. That is Task 5.2's atomic replace-only operation across
  `wos-provenance-log.schema.json`, `wos-workflow.schema.json`, `api/provenance.schema.json`, and
  the Trellis-side parsers cited in `REFACTOR-TODO.md:411`.
- No fixture regeneration. That is Task 5.3.

## Files inspected (read-only)

- `work-spec/schemas/record-kind-registry.json` (lines 1–206)
- `work-spec/schemas/wos-provenance-log.schema.json` (lines 1–1797)
- `work-spec/schemas/api/provenance.schema.json` (lines 779–1346 for the Facts-tier envelope and
  D26 if/then guards)

## Acceptance

`python3 -m pytest tests/schemas -q` was run after this report was authored; no schemas were
modified, only this markdown report was written.
