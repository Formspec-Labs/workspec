"""Completion export request and result selectors retain exact process identity."""
import itertools
import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR, api_referencing_registry, load_openapi, load_schema,
    validator_for_api_def,
)

PROCESS = "default_process_01hw7rm71vfay8vvw14d2pf2db"
PROCESS_URN = "urn:wos:" + PROCESS
REQUEST_ID = "a" * 32


def test_result_variants_are_complete_closed_objects_for_maintained_generators():
    result = load_schema(API_SCHEMA_DIR / "completion-exports.schema.json")["$defs"]["CompletionExportResult"]
    assert "properties" not in result
    for arm, status in zip(result["oneOf"], ("pending", "ready"), strict=True):
        assert arm["type"] == "object"
        assert arm["additionalProperties"] is False
        assert set(arm["required"]) == {"status", "processId", "requestId", "checkpointRef"}
        assert set(arm["properties"]) == set(arm["required"])
        assert arm["properties"]["status"]["const"] == status


def test_complete_result_arms_preserve_prior_wire_validation(checks):
    prior = json.loads((Path(__file__).parent / "fixtures" /
                        "completion-export-result-before-complete-arms.schema.json").read_text())
    old = Draft202012Validator(prior, registry=api_referencing_registry())
    new = checks["CompletionExportResult"]
    candidates = itertools.product(
        ("pending", "ready", "invented", None),
        (PROCESS_URN, PROCESS, PROCESS_URN.replace("_process_", "_case_"), None),
        (REQUEST_ID, "A" * 32, "", None),
        (None, "opaque-checkpoint", "", 42),
    )
    accepted = 0
    for status, process, request_id, checkpoint in candidates:
        value = {"status": status, "processId": process,
                 "requestId": request_id, "checkpointRef": checkpoint}
        old_valid = old.is_valid(value)
        assert new.is_valid(value) == old_valid, value
        accepted += old_valid
    assert accepted == 2
    for status, checkpoint in (("pending", None), ("ready", "opaque-checkpoint")):
        value = {"status": status, "processId": PROCESS_URN,
                 "requestId": REQUEST_ID, "checkpointRef": checkpoint}
        for field in value:
            missing = dict(value)
            del missing[field]
            assert not old.is_valid(missing)
            assert not new.is_valid(missing)
        extra = {**value, "invented": "refuse"}
        assert not old.is_valid(extra)
        assert not new.is_valid(extra)


@pytest.fixture
def checks():
    schema = load_schema(API_SCHEMA_DIR / "completion-exports.schema.json")
    registry = api_referencing_registry()
    return {name: validator_for_api_def(schema, name, registry) for name in
            ("CompletionExportRequest", "CompletionExportResult")}


@pytest.mark.parametrize("process", [PROCESS, PROCESS_URN])
def test_request_accepts_only_actual_process_selector(checks, process):
    check = checks["CompletionExportRequest"]
    check.validate({"processId": process})
    for field in ("requestId", "checkpointRef", "actor", "scope", "token", "Idempotency-Key"):
        assert not check.is_valid({"processId": process, field: "invented"})


@pytest.mark.parametrize("value", [{}, {"processId": None}, {"processId": "invented"},
                                   {"processId": "urn:wos:" + PROCESS.replace("_process_", "_case_")},
                                   {"processId": PROCESS_URN + "/tasks/respond"}])
def test_request_refuses_missing_or_wrong_resource_identity(checks, value):
    assert not checks["CompletionExportRequest"].is_valid(value)


@pytest.mark.parametrize("status,checkpoint", [("pending", None), ("ready", "retained-checkpoint")])
def test_result_requires_complete_state_and_canonical_process_identity(checks, status, checkpoint):
    check = checks["CompletionExportResult"]
    value = {"status": status, "processId": PROCESS_URN,
             "requestId": REQUEST_ID, "checkpointRef": checkpoint}
    check.validate(value)
    for field in value:
        missing = dict(value)
        del missing[field]
        assert not check.is_valid(missing)
    assert not check.is_valid({**value, "processId": PROCESS})
    assert not check.is_valid({**value, "downloadUrl": "invented"})


@pytest.mark.parametrize("status,checkpoint", [("pending", "invented"), ("ready", None),
                                               ("ready", ""), ("accepted", None), ("failed", None)])
def test_result_refuses_inconsistent_or_invented_states(checks, status, checkpoint):
    assert not checks["CompletionExportResult"].is_valid({
        "status": status, "processId": PROCESS_URN,
        "requestId": REQUEST_ID, "checkpointRef": checkpoint})


@pytest.mark.parametrize("request_id", [None, "a" * 31, "A" * 32, "g" * 32, "../request"])
def test_result_request_identity_is_exact_lowercase_hex(checks, request_id):
    assert not checks["CompletionExportResult"].is_valid({
        "status": "pending", "processId": PROCESS_URN,
        "requestId": request_id, "checkpointRef": None})


def test_status_and_download_routes_accept_same_process_and_request_selectors():
    paths = load_openapi()["paths"]
    for suffix in ("", "/download"):
        parameters = paths["/processes/{process_id}/completion-exports/{request_id}" + suffix]["get"]["parameters"]
        selectors = {item["name"]: item for item in parameters}
        assert selectors["process_id"]["schema"]["$ref"] == (
            "https://schemas.formspec.io/wos-api/_common/v1#/$defs/ProcessPathId")
        assert selectors["request_id"]["schema"]["pattern"] == "^[0-9a-f]{32}$"
        assert all(item["in"] == "path" and item["required"] for item in selectors.values())
    post = paths["/completion-exports"]["post"]
    assert post["requestBody"]["content"]["application/json"]["schema"]["$ref"].endswith(
        "#/$defs/CompletionExportRequest")
    assert post["parameters"][0]["name"] == "Idempotency-Key"
    assert post["parameters"][0]["in"] == "header"
