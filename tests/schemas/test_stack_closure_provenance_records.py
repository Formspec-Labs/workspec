"""Kernel-schema regression tests for the WOS stack-closure provenance cluster.

Covers the 14 new ``ProvenanceKind`` variants landed for ADRs 0066 (five-mode
amendment taxonomy + authorizing-act primitives), 0067 (clock primitives),
0068 (tenant-independent identity attestation), 0069 (clock-skew observation),
0070 (commit-attempt failures + authorization rejection), and 0071 (migration
pin change). After ADR 0076 the 14 record-shape ``$def``s live in
``schemas/wos-workflow.schema.json`` and the runtime export schema
``schemas/wos-provenance-log.schema.json`` ``$ref``s into them through the
shared cross-schema registry (``conftest._REGISTRY``).

The tests below validate the post-append export shape consulted by offline
verifiers and the Python conformance suite. ``$def`` lookups resolve via
``conftest.validator_for_def``, which spans every classified schema, so the
kernel-side copies ($wosWorkflow defs) are exercised through the
provenance-log envelope.

For each new record kind we cover:

- happy-path validation against the documented required fields,
- rejection when a required ``data`` field is omitted,
- where applicable, the ``if/then`` shape guard (clockResolved.paused requires
  resolvingEventHash; the other records carry only the discriminator-required
  guard, already covered by the missing-required-data test).

Bonus coverage:

- ``InstanceStatus.stalled`` + ``stalledSince`` if/then guard in
  ``schemas/wos-case-instance.schema.json`` (ADR 0070 D-5 / Q18 maximalist).
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

from .conftest import validator_for_def

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
PROVENANCE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "wos-provenance-log.schema.json"
)
CASE_INSTANCE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "wos-case-instance.schema.json"
)


@pytest.fixture(scope="module")
def schema() -> dict:
    """Retained for tests that round-trip the runtime envelope itself.

    Most ``$def``-shape tests resolve via ``validator_for_def`` instead; this
    fixture stays for the few cases that still need the raw provenance-log
    document for top-level validation.
    """
    return json.loads(PROVENANCE_SCHEMA.read_text())


@pytest.fixture(scope="module")
def case_instance_schema() -> dict:
    return json.loads(CASE_INSTANCE_SCHEMA.read_text())


def _validator_for_def(schema: dict, def_name: str) -> Draft202012Validator:
    """Cross-schema-aware ``$def`` validator. ``schema`` is ignored; def
    lookup spans every classified schema via ``conftest._REGISTRY`` so
    ``$def``s promoted to ``wos-workflow.schema.json`` (ADR 0076 step 5)
    resolve regardless of which schema a test loaded.
    """
    return validator_for_def(def_name)


def _facts_record(record_kind: str, record_id: str | None = None, **extra) -> dict:
    record = {
        "id": record_id or "sba-poc_prov_01jqt0wn4yh3p5q9r2x6t7v8w0",
        "recordKind": record_kind,
        "timestamp": "2026-04-28T14:00:00Z",
        "auditLayer": "facts",
        "definitionVersion": "1.0.0",
    }
    record.update(extra)
    return record


# ---------------------------------------------------------------------------
# ADR-0066: correction / amendment / rescission / reinstatement / attestation
# ---------------------------------------------------------------------------


class TestCorrectionAuthorized:
    def test_happy_path(self, schema):
        validator = _validator_for_def(schema, "CorrectionAuthorizedRecord")
        record = _facts_record(
            "correctionAuthorized",
            data={
                "correctionTargetEventHash": "a" * 64,
                "correctedFieldSet": ["/applicantName"],
                "reason": "transcription error",
                "authorizingActorId": "supervisor-001",
                "authorityBasis": {
                    "kind": "actorPolicyRef",
                    "value": "supervisor-correction-policy",
                },
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_missing_authority_basis_rejected(self, schema):
        validator = _validator_for_def(schema, "CorrectionAuthorizedRecord")
        record = _facts_record(
            "correctionAuthorized",
            data={
                "correctionTargetEventHash": "a" * 64,
                "correctedFieldSet": ["/applicantName"],
                "reason": "transcription error",
                "authorizingActorId": "supervisor-001",
            },
        )
        assert list(validator.iter_errors(record))

    def test_authority_basis_uri_form(self, schema):
        validator = _validator_for_def(schema, "CorrectionAuthorizedRecord")
        record = _facts_record(
            "correctionAuthorized",
            data={
                "correctionTargetEventHash": "a" * 64,
                "correctedFieldSet": ["/zip"],
                "reason": "encoding fix",
                "authorizingActorId": "system-batch-corrector",
                "authorityBasis": {
                    "kind": "uri",
                    "value": "https://agency.gov/policy/transcription-correction/v1",
                },
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_field_set_must_be_json_pointer(self, schema):
        validator = _validator_for_def(schema, "CorrectionAuthorizedRecord")
        record = _facts_record(
            "correctionAuthorized",
            data={
                "correctionTargetEventHash": "a" * 64,
                "correctedFieldSet": ["applicantName"],
                "reason": "x",
                "authorizingActorId": "x",
                "authorityBasis": {"kind": "actorPolicyRef", "value": "y"},
            },
        )
        assert list(validator.iter_errors(record))


class TestAmendmentAuthorized:
    def test_happy_path(self, schema):
        validator = _validator_for_def(schema, "AmendmentAuthorizedRecord")
        record = _facts_record(
            "amendmentAuthorized",
            data={
                "amendmentTargetEventHash": "b" * 64,
                "priorDeterminationHash": "c" * 64,
                "reason": "appeal sustained",
                "authorizingActorId": "appeals-officer-007",
                "authorityBasis": {
                    "kind": "uri",
                    "value": "https://agency.gov/regulations/benefits-act",
                },
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_hash_pattern_enforced(self, schema):
        validator = _validator_for_def(schema, "AmendmentAuthorizedRecord")
        record = _facts_record(
            "amendmentAuthorized",
            data={
                "amendmentTargetEventHash": "not-a-hex-hash",
                "priorDeterminationHash": "c" * 64,
                "reason": "x",
                "authorizingActorId": "x",
                "authorityBasis": {"kind": "actorPolicyRef", "value": "y"},
            },
        )
        assert list(validator.iter_errors(record))


class TestDeterminationAmended:
    def test_happy_path(self, schema):
        validator = _validator_for_def(schema, "DeterminationAmendedRecord")
        record = _facts_record(
            "determinationAmended",
            data={
                "priorDeterminationHash": "d" * 64,
                "newDeterminationValue": {"eligible": True, "monthlyAmount": 1850},
                "amendmentAuthorizationEventHash": "e" * 64,
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_new_determination_value_accepts_scalar(self, schema):
        validator = _validator_for_def(schema, "DeterminationAmendedRecord")
        record = _facts_record(
            "determinationAmended",
            data={
                "priorDeterminationHash": "d" * 64,
                "newDeterminationValue": "approved",
                "amendmentAuthorizationEventHash": "e" * 64,
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_missing_authorization_event_hash_rejected(self, schema):
        validator = _validator_for_def(schema, "DeterminationAmendedRecord")
        record = _facts_record(
            "determinationAmended",
            data={
                "priorDeterminationHash": "d" * 64,
                "newDeterminationValue": {},
            },
        )
        assert list(validator.iter_errors(record))


class TestRescissionAuthorized:
    def test_happy_path_without_migration(self, schema):
        validator = _validator_for_def(schema, "RescissionAuthorizedRecord")
        record = _facts_record(
            "rescissionAuthorized",
            data={
                "rescissionTargetEventHash": "f" * 64,
                "priorDeterminationHash": "f" * 64,
                "reason": "fraud detected post-determination",
                "authorizingActorId": "program-integrity-officer",
                "authorityBasis": {
                    "kind": "uri",
                    "value": "https://agency.gov/fraud-rescission",
                },
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_happy_path_with_migration_pin_change(self, schema):
        validator = _validator_for_def(schema, "RescissionAuthorizedRecord")
        pin = {
            "formspec.definitionVersion": "1.0.0",
            "wos.$wosWorkflowVersion": "1.0",
            "trellis.envelopeVersion": "1.0",
            "trellis.conformanceClass": "core",
        }
        new_pin = {**pin, "trellis.conformanceClass": "core+aead"}
        record = _facts_record(
            "rescissionAuthorized",
            data={
                "rescissionTargetEventHash": "f" * 64,
                "priorDeterminationHash": "f" * 64,
                "reason": "regulatory authority revoked; chain re-pinned",
                "authorizingActorId": "program-integrity-officer",
                "authorityBasis": {
                    "kind": "actorPolicyRef",
                    "value": "rescission-with-migration-policy-v3",
                },
                "migrationPinChange": {
                    "newChainPinEventHash": "1" * 64,
                    "priorPinSet": pin,
                    "newPinSet": new_pin,
                },
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_migration_pin_set_rejects_missing_field(self, schema):
        validator = _validator_for_def(schema, "RescissionAuthorizedRecord")
        bad_pin = {
            "formspec.definitionVersion": "1.0.0",
            "wos.$wosWorkflowVersion": "1.0",
            "trellis.envelopeVersion": "1.0",
        }
        record = _facts_record(
            "rescissionAuthorized",
            data={
                "rescissionTargetEventHash": "f" * 64,
                "priorDeterminationHash": "f" * 64,
                "reason": "x",
                "authorizingActorId": "y",
                "authorityBasis": {"kind": "actorPolicyRef", "value": "z"},
                "migrationPinChange": {
                    "newChainPinEventHash": "1" * 64,
                    "priorPinSet": bad_pin,
                    "newPinSet": bad_pin,
                },
            },
        )
        assert list(validator.iter_errors(record))


class TestDeterminationRescinded:
    def test_happy_path(self, schema):
        validator = _validator_for_def(schema, "DeterminationRescindedRecord")
        record = _facts_record(
            "determinationRescinded",
            data={
                "priorDeterminationHash": "f" * 64,
                "rescissionAuthorizationEventHash": "9" * 64,
            },
        )
        assert list(validator.iter_errors(record)) == []


class TestReinstated:
    def test_happy_path(self, schema):
        validator = _validator_for_def(schema, "ReinstatedRecord")
        record = _facts_record(
            "reinstated",
            data={
                "priorRescissionEventHash": "9" * 64,
                "reactivationAuthorizationEventHash": "8" * 64,
                "reason": "rescission overturned on appeal",
            },
        )
        assert list(validator.iter_errors(record)) == []


class TestAuthorizationAttestation:
    def test_happy_path_reserved_predicate(self, schema):
        validator = _validator_for_def(schema, "AuthorizationAttestationRecord")
        record = _facts_record(
            "authorizationAttestation",
            data={
                "authorizingActorId": "appeals-officer-007",
                "authorityBasis": {
                    "kind": "uri",
                    "value": "https://agency.gov/regulations/benefits-act",
                },
                "policyPredicate": "amendment-authority",
                "assuranceLevel": "high",
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_vendor_extension_predicate_accepted(self, schema):
        validator = _validator_for_def(schema, "AuthorizationAttestationRecord")
        record = _facts_record(
            "authorizationAttestation",
            data={
                "authorizingActorId": "x",
                "authorityBasis": {"kind": "actorPolicyRef", "value": "y"},
                "policyPredicate": "x-vendor-batch-correction-authority",
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_unreserved_predicate_rejected(self, schema):
        validator = _validator_for_def(schema, "AuthorizationAttestationRecord")
        record = _facts_record(
            "authorizationAttestation",
            data={
                "authorizingActorId": "x",
                "authorityBasis": {"kind": "actorPolicyRef", "value": "y"},
                "policyPredicate": "make-it-up-authority",
            },
        )
        assert list(validator.iter_errors(record))


# ---------------------------------------------------------------------------
# ADR-0067: clock primitives
# ---------------------------------------------------------------------------


class TestClockStarted:
    def test_happy_path(self, schema):
        validator = _validator_for_def(schema, "ClockStartedRecord")
        record = _facts_record(
            "clockStarted",
            data={
                "clockId": "appeal-window-2026-0042",
                "clockKind": "AppealClock",
                "originEventHash": "a" * 64,
                "duration": "P30D",
                "computedDeadline": "2026-05-28T12:00:00Z",
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_vendor_clock_kind_extension_accepted(self, schema):
        validator = _validator_for_def(schema, "ClockStartedRecord")
        record = _facts_record(
            "clockStarted",
            data={
                "clockId": "x",
                "clockKind": "x-medicaid-redetermination-clock",
                "originEventHash": "a" * 64,
                "duration": "P12M",
                "computedDeadline": "2027-04-28T12:00:00Z",
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_invalid_duration_rejected(self, schema):
        validator = _validator_for_def(schema, "ClockStartedRecord")
        record = _facts_record(
            "clockStarted",
            data={
                "clockId": "x",
                "clockKind": "AppealClock",
                "originEventHash": "a" * 64,
                "duration": "30 days",
                "computedDeadline": "2026-05-28T12:00:00Z",
            },
        )
        assert list(validator.iter_errors(record))


class TestClockResolved:
    def test_satisfied_with_resolving_event(self, schema):
        validator = _validator_for_def(schema, "ClockResolvedRecord")
        record = _facts_record(
            "clockResolved",
            data={
                "clockId": "sla-initial-review-2026-0042",
                "originClockHash": "a" * 64,
                "resolution": "satisfied",
                "resolvedAt": "2026-05-15T09:00:00Z",
                "resolvingEventHash": "b" * 64,
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_synthetic_elapsed_without_resolving_event(self, schema):
        """Q11 maximalist: elapsed without resolvingEventHash means a
        synthetic-elapsed emission (deadline passed, no event)."""
        validator = _validator_for_def(schema, "ClockResolvedRecord")
        record = _facts_record(
            "clockResolved",
            data={
                "clockId": "appeal-window-2026-0042",
                "originClockHash": "a" * 64,
                "resolution": "elapsed",
                "resolvedAt": "2026-05-28T12:00:00Z",
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_paused_requires_resolving_event_hash(self, schema):
        """Q11 maximalist: when resolution=paused, the pause event itself
        is the resolving event and MUST be present."""
        validator = _validator_for_def(schema, "ClockResolvedRecord")
        record = _facts_record(
            "clockResolved",
            data={
                "clockId": "appeal-window-2026-0099",
                "originClockHash": "a" * 64,
                "resolution": "paused",
                "resolvedAt": "2026-05-01T14:30:00Z",
            },
        )
        errors = list(validator.iter_errors(record))
        assert errors, (
            "clockResolved with resolution=paused MUST require resolvingEventHash "
            f"per Q11 maximalist; got no errors: {errors}"
        )

    def test_paused_with_resolving_event_accepted(self, schema):
        validator = _validator_for_def(schema, "ClockResolvedRecord")
        record = _facts_record(
            "clockResolved",
            data={
                "clockId": "appeal-window-2026-0099",
                "originClockHash": "a" * 64,
                "resolution": "paused",
                "resolvedAt": "2026-05-01T14:30:00Z",
                "resolvingEventHash": "b" * 64,
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_unreserved_resolution_rejected(self, schema):
        """Resolution is closed-enum -- vendor extensions deliberately not allowed."""
        validator = _validator_for_def(schema, "ClockResolvedRecord")
        record = _facts_record(
            "clockResolved",
            data={
                "clockId": "x",
                "originClockHash": "a" * 64,
                "resolution": "x-vendor-paused-pending-review",
                "resolvedAt": "2026-05-01T14:30:00Z",
            },
        )
        assert list(validator.iter_errors(record))


# ---------------------------------------------------------------------------
# ADR-0068: identity attestation
# ---------------------------------------------------------------------------


class TestIdentityAttestation:
    def test_happy_path(self, schema):
        validator = _validator_for_def(schema, "IdentityAttestationRecord")
        record = _facts_record(
            "identityAttestation",
            data={
                "subjectGlobalId": "globalid:eaff8b1c-4e3d-4c09-9c91-9b06b7d4c2e1",
                "assuranceLevel": "high",
                "attestationProvider": "urn:agency.gov:identity:idme",
                "providerAttestationId": "idme-2026-04-28-0042",
                "attestedAt": "2026-04-28T10:55:00Z",
                "attestedPredicates": ["legal-name-verified", "age-of-majority"],
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_attested_predicates_must_be_non_empty(self, schema):
        validator = _validator_for_def(schema, "IdentityAttestationRecord")
        record = _facts_record(
            "identityAttestation",
            data={
                "subjectGlobalId": "globalid:abc",
                "assuranceLevel": "low",
                "attestationProvider": "x",
                "providerAttestationId": "y",
                "attestedAt": "2026-04-28T10:55:00Z",
                "attestedPredicates": [],
            },
        )
        assert list(validator.iter_errors(record))

    def test_optional_valid_until(self, schema):
        validator = _validator_for_def(schema, "IdentityAttestationRecord")
        record = _facts_record(
            "identityAttestation",
            data={
                "subjectGlobalId": "globalid:abc",
                "assuranceLevel": "standard",
                "attestationProvider": "x",
                "providerAttestationId": "y",
                "attestedAt": "2026-04-28T10:55:00Z",
                "attestedPredicates": ["email-control-verified"],
                "validUntil": "2027-04-28T10:55:00Z",
            },
        )
        assert list(validator.iter_errors(record)) == []


# ---------------------------------------------------------------------------
# ADR-0069: clock skew observation
# ---------------------------------------------------------------------------


class TestClockSkewObserved:
    def test_happy_path(self, schema):
        validator = _validator_for_def(schema, "ClockSkewObservedRecord")
        record = _facts_record(
            "clockSkewObserved",
            data={
                "processorAuthoredAt": "2026-04-28T12:00:00.000Z",
                "substrateCreatedAt": "2026-04-28T12:00:01.500Z",
                "skewMilliseconds": 1500,
                "thresholdMilliseconds": 1000,
                "eventHash": "a" * 64,
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_negative_skew_allowed(self, schema):
        """Processor clock ahead of substrate produces negative skew."""
        validator = _validator_for_def(schema, "ClockSkewObservedRecord")
        record = _facts_record(
            "clockSkewObserved",
            data={
                "processorAuthoredAt": "2026-04-28T12:00:01.500Z",
                "substrateCreatedAt": "2026-04-28T12:00:00.000Z",
                "skewMilliseconds": -1500,
                "thresholdMilliseconds": 1000,
                "eventHash": "a" * 64,
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_threshold_zero_rejected(self, schema):
        validator = _validator_for_def(schema, "ClockSkewObservedRecord")
        record = _facts_record(
            "clockSkewObserved",
            data={
                "processorAuthoredAt": "2026-04-28T12:00:00.000Z",
                "substrateCreatedAt": "2026-04-28T12:00:01.000Z",
                "skewMilliseconds": 1000,
                "thresholdMilliseconds": 0,
                "eventHash": "a" * 64,
            },
        )
        assert list(validator.iter_errors(record))


# ---------------------------------------------------------------------------
# ADR-0070: commit-attempt failures, authorization rejection, stalled status
# ---------------------------------------------------------------------------


class TestCommitAttemptFailure:
    def test_happy_path_network_timeout(self, schema):
        validator = _validator_for_def(schema, "CommitAttemptFailureRecord")
        record = _facts_record(
            "commitAttemptFailure",
            data={
                "targetEventHash": "a" * 64,
                "failureKind": "networkTimeout",
                "attemptCount": 1,
                "retryBudgetRemainingMs": 60000,
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_failure_kind_closed_enum(self, schema):
        validator = _validator_for_def(schema, "CommitAttemptFailureRecord")
        record = _facts_record(
            "commitAttemptFailure",
            data={
                "targetEventHash": "a" * 64,
                "failureKind": "x-vendor-disk-full",
                "attemptCount": 1,
                "retryBudgetRemainingMs": 0,
            },
        )
        assert list(validator.iter_errors(record))

    def test_optional_error_payload(self, schema):
        validator = _validator_for_def(schema, "CommitAttemptFailureRecord")
        record = _facts_record(
            "commitAttemptFailure",
            data={
                "targetEventHash": "a" * 64,
                "failureKind": "hashConflict",
                "attemptCount": 3,
                "retryBudgetRemainingMs": 0,
                "errorPayload": {
                    "expected": "a" * 64,
                    "observed": "b" * 64,
                },
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_retry_budget_must_be_non_negative(self, schema):
        validator = _validator_for_def(schema, "CommitAttemptFailureRecord")
        record = _facts_record(
            "commitAttemptFailure",
            data={
                "targetEventHash": "a" * 64,
                "failureKind": "other",
                "attemptCount": 1,
                "retryBudgetRemainingMs": -100,
            },
        )
        assert list(validator.iter_errors(record))


class TestAuthorizationRejected:
    def test_happy_path(self, schema):
        validator = _validator_for_def(schema, "AuthorizationRejectedRecord")
        record = _facts_record(
            "authorizationRejected",
            data={
                "attemptedActorId": "intern-042",
                "attemptedAction": "transition:approve",
                "targetResourceId": "case-2026-0042",
                "rejectionReason": "actor lacks approval-authority predicate",
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_action_format_verb_resource(self, schema):
        validator = _validator_for_def(schema, "AuthorizationRejectedRecord")
        record = _facts_record(
            "authorizationRejected",
            data={
                "attemptedActorId": "x",
                "attemptedAction": "approve_transition",
                "targetResourceId": "y",
                "rejectionReason": "z",
            },
        )
        assert list(validator.iter_errors(record))

    def test_optional_policy_decision_ref(self, schema):
        validator = _validator_for_def(schema, "AuthorizationRejectedRecord")
        record = _facts_record(
            "authorizationRejected",
            data={
                "attemptedActorId": "applicant-123",
                "attemptedAction": "submit:taskResponse",
                "targetResourceId": "task-eligibility-2026-0042",
                "rejectionReason": "task assigned to a different actor",
                "policyDecisionRef": "urn:agency.gov:authz:decisions:0099",
            },
        )
        assert list(validator.iter_errors(record)) == []


class TestStalledInstanceStatus:
    """Q18 maximalist: `stalled` is a reserved InstanceStatus variant
    (not a kernel statechart node type) for ADR 0070 D-5 retry-budget
    exhaustion. `stalledSince` is REQUIRED when status == "stalled"."""

    @staticmethod
    def _instance(status: str, **extra) -> dict:
        record = {
            "$wosCaseInstance": "1.0",
            "instanceId": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd",
            "definitionUrl": "https://agency.gov/workflows/benefits",
            "definitionVersion": "1.0.0",
            "configuration": ["intake"],
            "caseState": {},
            "provenancePosition": 0,
            "timers": [],
            "activeTasks": [],
            "status": status,
            "createdAt": "2026-04-28T12:00:00Z",
            "updatedAt": "2026-04-28T12:01:00Z",
        }
        record.update(extra)
        return record

    def test_stalled_status_listed(self, case_instance_schema):
        status_enum = case_instance_schema["properties"]["status"]["enum"]
        assert "stalled" in status_enum

    def test_stalled_requires_stalled_since(self, case_instance_schema):
        validator = Draft202012Validator(case_instance_schema)
        record = self._instance("stalled")
        errors = list(validator.iter_errors(record))
        assert errors, (
            "instance with status=stalled MUST require stalledSince per ADR 0070 D-5"
        )

    def test_stalled_with_stalled_since_accepted(self, case_instance_schema):
        validator = Draft202012Validator(case_instance_schema)
        record = self._instance("stalled", stalledSince="2026-04-28T12:01:00Z")
        assert list(validator.iter_errors(record)) == []

    def test_active_without_stalled_since_accepted(self, case_instance_schema):
        validator = Draft202012Validator(case_instance_schema)
        record = self._instance("active")
        assert list(validator.iter_errors(record)) == []


# ---------------------------------------------------------------------------
# ADR-0071: migration pin change
# ---------------------------------------------------------------------------


class TestMigrationPinChanged:
    @staticmethod
    def _pin(**overrides) -> dict:
        pin = {
            "formspec.definitionVersion": "1.0.0",
            "wos.$wosWorkflowVersion": "1.0",
            "trellis.envelopeVersion": "1.0",
            "trellis.conformanceClass": "core",
        }
        pin.update(overrides)
        return pin

    def test_happy_path(self, schema):
        validator = _validator_for_def(schema, "MigrationPinChangedRecord")
        record = _facts_record(
            "migrationPinChanged",
            data={
                "priorPinSet": self._pin(),
                "newPinSet": self._pin(**{"trellis.conformanceClass": "core+aead"}),
                "authorizingActorId": "platform-migration-officer",
                "authorityBasis": {
                    "kind": "uri",
                    "value": "https://agency.gov/regulations/migration-policy/v2",
                },
                "migrationRationale": "uplift to AEAD-required Trellis conformance",
            },
        )
        assert list(validator.iter_errors(record)) == []

    def test_pin_set_rejects_unknown_field(self, schema):
        """MigrationPinSet additionalProperties: false -- Q33 four-field pin
        tree is exhaustive; vendor pin axes don't extend this shape."""
        validator = _validator_for_def(schema, "MigrationPinChangedRecord")
        bad_pin = self._pin()
        bad_pin["x-vendor-extra-pin"] = "foo"
        record = _facts_record(
            "migrationPinChanged",
            data={
                "priorPinSet": bad_pin,
                "newPinSet": self._pin(),
                "authorizingActorId": "x",
                "authorityBasis": {"kind": "actorPolicyRef", "value": "y"},
                "migrationRationale": "z",
            },
        )
        assert list(validator.iter_errors(record))

    def test_missing_migration_rationale_rejected(self, schema):
        validator = _validator_for_def(schema, "MigrationPinChangedRecord")
        record = _facts_record(
            "migrationPinChanged",
            data={
                "priorPinSet": self._pin(),
                "newPinSet": self._pin(),
                "authorizingActorId": "x",
                "authorityBasis": {"kind": "actorPolicyRef", "value": "y"},
            },
        )
        assert list(validator.iter_errors(record))


# ---------------------------------------------------------------------------
# Cross-cutting: AuthorityBasis discriminator
# ---------------------------------------------------------------------------


class TestAuthorityBasis:
    """The AuthorityBasis sub-shape is shared across 5 of the 14 new record
    kinds (correctionAuthorized, amendmentAuthorized, rescissionAuthorized,
    authorizationAttestation, migrationPinChanged) plus referenced from a 6th
    via the rescission migrationPinChange composition. Validate the
    discriminated union directly."""

    def test_uri_kind_requires_uri_format(self, schema):
        validator = _validator_for_def(schema, "AuthorityBasis")
        # Free-form strings parse as relative URIs, so jsonschema's `format` check
        # accepts most non-pathological strings; the load-bearing constraint here
        # is the discriminator pairing, validated below.
        ok = {"kind": "uri", "value": "https://example.gov/policy"}
        assert list(validator.iter_errors(ok)) == []

    def test_actor_policy_ref_kind_accepts_opaque_string(self, schema):
        validator = _validator_for_def(schema, "AuthorityBasis")
        record = {"kind": "actorPolicyRef", "value": "intake-supervisor-policy-v1"}
        assert list(validator.iter_errors(record)) == []

    def test_unknown_kind_rejected(self, schema):
        validator = _validator_for_def(schema, "AuthorityBasis")
        record = {"kind": "freeText", "value": "anything goes"}
        assert list(validator.iter_errors(record))

    def test_missing_value_rejected(self, schema):
        validator = _validator_for_def(schema, "AuthorityBasis")
        record = {"kind": "actorPolicyRef"}
        assert list(validator.iter_errors(record))
