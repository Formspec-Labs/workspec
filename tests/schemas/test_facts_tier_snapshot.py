"""Facts-tier case-file snapshot schema regression tests.

Validates the runtime provenance-log shape ``wos-provenance-log.schema.json``.
Per ADR 0076 step 5, ``FactsTierRecord`` and ``CaseFileSnapshot`` were
promoted into ``wos-workflow.schema.json``'s ``$defs``; the runtime log
``$ref``s them across schemas. The cross-schema registry in
``conftest`` resolves both the in-document ``$ref`` and the bare ``$def``
lookups used here, so kernel-side promotion is invisible to the tests.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

from .conftest import _REGISTRY, validator_for_def
from .test_record_kind_registry import event_literal_mappings

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
PROVENANCE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "wos-provenance-log.schema.json"
)


@pytest.fixture(scope="module")
def schema() -> dict:
    return json.loads(PROVENANCE_SCHEMA.read_text())


def _validator_for_def(schema: dict, def_name: str) -> Draft202012Validator:
    """Registry-aware ``$def`` lookup. ``schema`` is ignored; def lookup
    spans every classified schema via ``conftest._REGISTRY``.
    """
    return validator_for_def(def_name)


def _document_validator(schema: dict) -> Draft202012Validator:
    """Validate the top-level provenance log document. This is the canonical
    shape the runtime emits, so it -- not the bare ``FactsTierRecord`` $def --
    is what production exports flow through. Registry-aware so cross-schema
    ``$ref``s into ``wos-workflow.schema.json`` resolve.
    """
    from jsonschema import FormatChecker
    return Draft202012Validator(
        schema,
        registry=_REGISTRY,
        format_checker=FormatChecker(),
    )


def _snapshot() -> dict:
    return {
        "value": {"eligible": True, "income": 17500},
        "jcsCanonical": '{"eligible":true,"income":17500}',
        "sha256": "b19f000c0cd497b52c4a78e50641651e4b1e96931a1b1558984d69e722f73f5e",
    }


def _facts_record(record_kind: str, **extra) -> dict:
    event = event_literal_mappings().get(record_kind, f"x-test.{record_kind}")
    record = {
        "id": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
        "event": event,
        "timestamp": "2026-04-19T12:00:00Z",
        "auditLayer": "facts",
        "definitionVersion": "1.0.0",
    }
    if record_kind == "stateTransition" and "data" not in extra:
        record["data"] = {"transitionEvent": "submit"}
    record.update(extra)
    return record


_VALID_SEED_DATA_BY_KIND = {
    "correctionAuthorized": {
        "correctionTargetEventHash": "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c",
        "correctedFieldSet": ["/applicantName"],
        "reason": "transcription correction",
        "authorizingActorId": "supervisor-001",
        "authorityBasis": {
            "kind": "actorPolicyRef",
            "value": "intake-supervisor-correction-policy",
        },
    },
    "amendmentAuthorized": {
        "amendmentTargetEventHash": "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c",
        "priorDeterminationHash": "8ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090b",
        "reason": "appeal sustained",
        "authorizingActorId": "appeals-officer-007",
        "authorityBasis": {
            "kind": "uri",
            "value": "https://agency.gov/regulations/benefits-act/section-7.3",
        },
    },
    "determinationAmended": {
        "priorDeterminationHash": "8ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090b",
        "newDeterminationValue": {"eligible": True},
        "amendmentAuthorizationEventHash": "5ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5092f",
    },
    "rescissionAuthorized": {
        "rescissionTargetEventHash": "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c",
        "priorDeterminationHash": "8ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090b",
        "reason": "determination issued under wrong workflow version",
        "authorizingActorId": "program-director-002",
        "authorityBasis": {
            "kind": "uri",
            "value": "https://agency.gov/policies/rescission/v1",
        },
    },
    "determinationRescinded": {
        "priorDeterminationHash": "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c",
        "rescissionAuthorizationEventHash": "6ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5093f",
    },
    "reinstated": {
        "priorRescissionEventHash": "6ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5093f",
        "reactivationAuthorizationEventHash": "8ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5095e",
        "reason": "rescission overturned on appeal",
    },
    "clockStarted": {
        "clockId": "appeal-window-2026-0042",
        "clockKind": "AppealClock",
        "originEventHash": "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c",
        "duration": "P30D",
        "computedDeadline": "2026-05-28T12:00:00Z",
    },
    "clockResolved": {
        "clockId": "appeal-window-2026-0042",
        "originClockHash": "2ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5092a",
        "resolution": "elapsed",
        "resolvedAt": "2026-05-28T12:00:00Z",
    },
    "identityAttestation": {
        "subjectGlobalId": "globalid:eaff8b1c-4e3d-4c09-9c91-9b06b7d4c2e1",
        "assuranceLevel": "high",
        "attestationProvider": "urn:agency.gov:identity:idme",
        "providerAttestationId": "idme-attestation-2026-04-28-0042",
        "attestedAt": "2026-04-28T10:55:00Z",
        "attestedPredicates": ["legal-name-verified"],
    },
    "keyRebind": {
        "priorKid": "00112233445566778899aabbccddeeff",
        "newKid": "ffeeddccbbaa99887766554433221100",
        "priorAssurance": "standard",
        "newAssurance": "high",
        "rebindAttestationRef": "urn:agency.gov:identity:attestations:rebind-2026-0001",
    },
    "authorizationAttestation": {
        "authorizingActorId": "supervisor-001",
        "authorityBasis": {
            "kind": "actorPolicyRef",
            "value": "approval-authority-policy",
        },
        "policyPredicate": "amendment-authority",
    },
    "clockSkewObserved": {
        "processorAuthoredAt": "2026-04-28T12:00:00.000Z",
        "substrateCreatedAt": "2026-04-28T12:00:01.500Z",
        "skewMilliseconds": 1500,
        "thresholdMilliseconds": 1000,
        "eventHash": "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c",
    },
    "commitAttemptFailure": {
        "targetEventHash": "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c",
        "failureKind": "networkTimeout",
        "attemptCount": 1,
        "retryBudgetRemainingMs": 60000,
    },
    "authorizationRejected": {
        "attemptedActorId": "intern-042",
        "attemptedAction": "transition:approve",
        "targetResourceId": "case-2026-0042",
        "rejectionReason": "actor lacks approval-authority predicate",
    },
    "migrationPinChanged": {
        "priorPinSet": {
            "formspec.definitionVersion": "1.0.0",
            "wos.$wosWorkflowVersion": "1.0",
            "trellis.envelopeVersion": "1.0",
            "trellis.conformanceClass": "core",
        },
        "newPinSet": {
            "formspec.definitionVersion": "1.1.0",
            "wos.$wosWorkflowVersion": "1.0",
            "trellis.envelopeVersion": "1.0",
            "trellis.conformanceClass": "core+aead",
        },
        "authorizingActorId": "platform-migration-officer",
        "authorityBasis": {
            "kind": "uri",
            "value": "https://agency.gov/regulations/migration-policy/v2",
        },
        "migrationRationale": "uplift Trellis conformance class",
    },
}


def _valid_seed_record(record_kind: str, **extra) -> dict:
    if record_kind in _VALID_SEED_DATA_BY_KIND and "data" not in extra:
        extra = {**extra, "data": _VALID_SEED_DATA_BY_KIND[record_kind]}
    return _facts_record(record_kind, **extra)


@pytest.mark.parametrize(
    ("record_kind", "event"),
    sorted(event_literal_mappings().items()),
)
def test_d26_seed_record_kind_uses_event_and_rejects_legacy_record_kind(
    schema, record_kind, event
):
    validator = _validator_for_def(schema, "FactsTierRecord")

    valid = _valid_seed_record(record_kind)
    assert valid["event"] == event
    assert list(
        validator.iter_errors(valid)
    ) == []

    legacy = dict(valid)
    legacy["recordKind"] = record_kind
    assert list(validator.iter_errors(legacy)), (
        f"{record_kind} must reject legacy inner recordKind"
    )


@pytest.mark.parametrize(
    "record_kind",
    sorted(event_literal_mappings()),
)
def test_d26_seed_record_kind_requires_event_on_base_facts_def(
    schema, record_kind
):
    validator = _validator_for_def(schema, "FactsTierRecord")

    record = _valid_seed_record(record_kind)
    record.pop("event")
    errors = list(validator.iter_errors(record))

    assert errors, (
        f"{record_kind} fragments without event must fail against "
        "the base FactsTierRecord $def after D26"
    )


def test_key_rebind_facts_tier_envelope_requires_typed_payload(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "keyRebind",
        event="wos.assurance.key_rebind",
        data={
            "priorKid": "00112233445566778899aabbccddeeff",
            "newKid": "ffeeddccbbaa99887766554433221100",
            "priorAssurance": "standard",
            "newAssurance": "high",
            "rebindAttestationRef": "urn:agency.gov:identity:attestations:rebind-2026-0001",
        },
    )
    missing_payload = _facts_record(
        "keyRebind",
        event="wos.assurance.key_rebind",
    )

    assert list(validator.iter_errors(record)) == []
    assert list(validator.iter_errors(missing_payload))


def test_determination_transition_without_snapshot_is_rejected(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        transitionTags=["determination"],
    )

    errors = list(validator.iter_errors(record))

    assert errors, "determination-tagged StateTransition must require caseFileSnapshot"


def test_determination_transition_with_snapshot_is_accepted(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        transitionTags=["determination"],
        caseFileSnapshot=_snapshot(),
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], f"valid determination snapshot rejected: {errors}"


def test_non_determination_transition_without_snapshot_is_accepted(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        transitionTags=["review"],
    )

    errors = list(validator.iter_errors(record))

    assert errors == [], f"non-determination snapshot should remain optional: {errors}"


def test_state_transition_without_transition_event_is_rejected(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        transitionTags=["review"],
    )
    del record["data"]

    errors = list(validator.iter_errors(record))

    assert errors, "stateTransition must preserve the workflow trigger in data.transitionEvent"


def test_state_transition_with_empty_transition_event_is_rejected(schema):
    validator = _validator_for_def(schema, "FactsTierRecord")
    record = _facts_record(
        "stateTransition",
        data={"transitionEvent": ""},
        transitionTags=["review"],
    )

    errors = list(validator.iter_errors(record))

    assert errors, "stateTransition data.transitionEvent must be non-empty"


def test_snapshot_rejects_malformed_sha256(schema):
    validator = _validator_for_def(schema, "CaseFileSnapshot")
    snapshot = _snapshot()
    snapshot["sha256"] = "not-a-sha"

    errors = list(validator.iter_errors(snapshot))

    assert errors, "caseFileSnapshot.sha256 must be a lowercase 64-character hex digest"


def test_full_document_rejects_determination_record_missing_snapshot(schema):
    """Full-document validation must reject a determination-tagged
    stateTransition that lacks ``caseFileSnapshot``. This is the structural
    bite Finding 2 asks for: the $def must produce errors on realistic
    provenance log exports, not only on the bare $def.
    """
    validator = _document_validator(schema)
    document = {
        "$wosProvenanceLog": "1.0",
        "provenanceLog": [
            {
                **_facts_record("stateTransition"),
                "transitionTags": ["determination"],
            }
        ]
    }

    errors = list(validator.iter_errors(document))

    assert errors, (
        "Full-document validation must reject a determination-tagged "
        "stateTransition without caseFileSnapshot"
    )


def test_full_document_accepts_determination_record_with_snapshot(schema):
    validator = _document_validator(schema)
    document = {
        "$wosProvenanceLog": "1.0",
        "provenanceLog": [
            {
                **_facts_record("stateTransition"),
                "event": "wos.kernel.state_transition",
                "transitionTags": ["determination"],
                "caseFileSnapshot": _snapshot(),
            },
            {
                **_facts_record(
                    "caseStateMutation",
                    id="sba-poc_prov_01hw7rm71vfay8vvw14d2pf2db",
                ),
                "transitionTags": [],
            },
        ]
    }

    errors = list(validator.iter_errors(document))

    assert errors == [], (
        f"well-formed provenance log rejected by FactsTierRecord: {errors}"
    )
