"""Closed native due-process data stays linked to its owning runtime schemas."""
import json
from pathlib import Path
from jsonschema import Draft202012Validator
from tests.schemas.conftest import SCHEMAS_ROOT, _REGISTRY

PROFILE = json.loads((SCHEMAS_ROOT / "wos-reference-due-process.schema.json").read_text())

def validator(name):
    return Draft202012Validator({"$ref": PROFILE["$id"] + "#/$defs/" + name}, registry=_REGISTRY)

def test_closed_command_rejects_unknown_fields_and_null():
    good = {"kind": "fileAppeal", "channel": "phone", "continuationChoice": "elect"}
    assert validator("GovernanceCommand").is_valid(good)
    assert not validator("GovernanceCommand").is_valid({**good, "filedAt": "2026-01-01T00:00:00Z"})
    assert not validator("GovernanceCommand").is_valid({**good, "continuationChoice": None})

def test_state_empty_arrays_and_optional_null_rejection():
    state = dict(determinations=[], notices=[], appeals=[], extensions=[], operations=[], replays=[])
    assert validator("DueProcessState").is_valid(state)
    assert not validator("DueProcessState").is_valid({**state, "notices": None})
    assert not validator("DueProcessState").is_valid({**state, "signed": True})

def test_lexical_snapshot_digest_and_native_timestamp_meanings():
    snapshot = {"recordId": "actual-native-record", "sha256": "a" * 64}
    assert validator("SnapshotRef").is_valid(snapshot)
    assert not validator("SnapshotRef").is_valid({**snapshot, "sha256": "sha256:" + "a" * 64})
    for valid in ["2026-01-01T00:00:00Z", "2026-01-01T00:00:00.1Z", "2026-01-01T00:00:00.001Z"]:
        assert validator("Timestamp").is_valid(valid)

def test_d26_native_events_use_exact_closed_data_schema():
    log = json.loads((SCHEMAS_ROOT / "wos-provenance-log.schema.json").read_text())
    assert log["$defs"]["DeterminationReasoningDataRecord"]["if"]["properties"]["event"]["const"] == "wos.governance.reasoning_recorded"
    assert log["$defs"]["DeterminationCounterfactualDataRecord"]["if"]["properties"]["event"]["const"] == "wos.governance.counterfactual_recorded"
    for name in ["DeterminationReasoningData", "DeterminationCounterfactualData", "DueProcessExecution", "DueProcessState"]:
        assert PROFILE["$defs"][name]["additionalProperties"] is False
    Draft202012Validator.check_schema(PROFILE)


def test_every_local_reference_resolves_and_scalar_types_keep_their_meaning():
    def walk(value):
        if isinstance(value, dict):
            reference = value.get("$ref", "")
            if reference.startswith("#/"):
                target = PROFILE
                for part in reference[2:].split("/"):
                    target = target[part.replace("~1", "/").replace("~0", "~")]
                assert isinstance(target, dict)
            for child in value.values():
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)
    walk(PROFILE)
    for kind, value in [("string", "text"), ("number", 50), ("boolean", True)]:
        assert validator("Scalar").is_valid({"kind": kind, "value": value})
        assert not validator("Scalar").is_valid({"kind": kind, "value": None})
    command = {"kind": "holdHearing", "appealId": "actual-appeal", "evidenceRefs": [{
        "sourceVersion": "hearing", "sectionAnchor": "exhibit-1", "retainedSha256": "sha256:" + "a" * 64,
    }]}
    assert validator("GovernanceCommand").is_valid(command)
    assert not validator("GovernanceCommand").is_valid({**command, "evidenceRefs": [None]})
    assert validator("GovernanceOutcome").is_valid({"kind": "hearingHeld", "appealId": "actual-appeal", "heldAt": "2026-01-01T00:00:00Z"})
    assert not validator("GovernanceOutcome").is_valid({"kind": "hearingHeld", "appealId": "actual-appeal", "heldAt": "not-a-clock"})


def test_transport_claims_keep_lexical_validation_separate_from_relationships():
    acknowledgement = dict(kind="accepted", channel="portal", noticeId="another-notice", transportKey="saved-key", correlation="wrong-correlation", artifactSha256="sha256:" + "b" * 64, adapterResultId="actual-result", acceptedAt="2026-01-02T00:00:00Z")
    receipt = {**acknowledgement, "qualifiedAt": acknowledgement["acceptedAt"], "kind": "qualifiedReceipt", "evidenceClass": "fixtureQualifiedReceipt"}
    receipt.pop("acceptedAt")
    result = dict(kind="qualified", acknowledgement=acknowledgement, receipt=receipt)
    assert validator("TransportResult").is_valid(result)
    for field, invalid in [("noticeId", ""), ("correlation", "invalid token"), ("artifactSha256", "sha256:ABC"), ("qualifiedAt", "2026-01-02T00:00:00.000Z")]:
        assert not validator("TransportResult").is_valid({**result, "receipt": {**receipt, field: invalid}})
    assert not validator("TransportResult").is_valid({**result, "extra": True})
    assert not validator("TransportResult").is_valid({**result, "receipt": None})
    state = PROFILE["$defs"]["NoticeState"]
    assert "transportResults" in state["required"]
    assert state["properties"]["transportResults"]["uniqueItems"] is True
    assert "default" not in state["properties"]["transportResults"]
