"""Completion export request and job selectors retain exact process identity."""
import pytest
from jsonschema import Draft202012Validator

from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR, api_referencing_registry, load_openapi, load_schema,
    validator_for_api_def,
)

PROCESS = "default_process_01hw7rm71vfay8vvw14d2pf2db"
PROCESS_URN = "urn:wos:" + PROCESS
JOB_ID = "a" * 32
STATES = {
    "running": {},
    "succeeded": {"checkpointRef": "retained-checkpoint"},
    "failed": {"failureCode": "export-retries-exhausted"},
}


def job(status, **overrides):
    return {"status": status, "processId": PROCESS_URN, "jobId": JOB_ID,
            **STATES[status], **overrides}


@pytest.fixture
def checks():
    schema = load_schema(API_SCHEMA_DIR / "completion-exports.schema.json")
    registry = api_referencing_registry()
    return {name: validator_for_api_def(schema, name, registry) for name in
            ("CompletionExportRequest", "CompletionExportJob", "CompletionExportJobList")}


def test_job_variants_are_complete_closed_objects_for_maintained_generators():
    result = load_schema(API_SCHEMA_DIR / "completion-exports.schema.json")["$defs"]["CompletionExportJob"]
    assert "properties" not in result
    for arm, status in zip(result["oneOf"], STATES, strict=True):
        assert arm["type"] == "object"
        assert arm["additionalProperties"] is False
        assert set(arm["required"]) == set(arm["properties"])
        assert arm["properties"]["status"]["const"] == status


@pytest.mark.parametrize("process", [PROCESS, PROCESS_URN])
def test_request_accepts_only_actual_process_selector(checks, process):
    check = checks["CompletionExportRequest"]
    check.validate({"processId": process})
    for field in ("jobId", "checkpointRef", "actor", "scope", "token", "Idempotency-Key"):
        assert not check.is_valid({"processId": process, field: "invented"})


@pytest.mark.parametrize("value", [{}, {"processId": None}, {"processId": "invented"},
                                   {"processId": "urn:wos:" + PROCESS.replace("_process_", "_case_")},
                                   {"processId": PROCESS_URN + "/tasks/respond"}])
def test_request_refuses_missing_or_wrong_resource_identity(checks, value):
    assert not checks["CompletionExportRequest"].is_valid(value)


@pytest.mark.parametrize("status", list(STATES))
def test_job_requires_complete_state_and_canonical_process_identity(checks, status):
    check = checks["CompletionExportJob"]
    value = job(status)
    check.validate(value)
    for field in value:
        missing = dict(value)
        del missing[field]
        assert not check.is_valid(missing), field
    assert not check.is_valid({**value, "processId": PROCESS})
    assert not check.is_valid({**value, "downloadUrl": "invented"})


@pytest.mark.parametrize("value", [
    {"status": "queued"},
    {"status": "running", "stage": "source_plan"},
    {"status": "running", "failureCode": "export-retries-exhausted"},
    {"status": "succeeded", "checkpointRef": ""},
    {"status": "succeeded", "checkpointRef": None},
    {"status": "failed", "failureCode": "invented"},
    {"status": "pending"},
    {"status": "ready"},
])
def test_job_refuses_inconsistent_or_invented_states(checks, value):
    base = {"processId": PROCESS_URN, "jobId": JOB_ID}
    assert not checks["CompletionExportJob"].is_valid({**base, **value})


@pytest.mark.parametrize("job_id", [None, "a" * 31, "A" * 32, "g" * 32, "../request"])
def test_job_identity_is_exact_lowercase_hex(checks, job_id):
    assert not checks["CompletionExportJob"].is_valid(job("running", jobId=job_id))


def test_job_list_is_bounded_closed_array_of_jobs(checks):
    check = checks["CompletionExportJobList"]
    check.validate({"jobs": []})
    check.validate({"jobs": [job(status) for status in STATES]})
    assert not check.is_valid({"jobs": [job("running")], "extra": 1})
    assert not check.is_valid({"jobs": [job("running")] * 51})


def test_routes_declare_job_acceptance_without_a_client_retry_header():
    paths = load_openapi()["paths"]
    for suffix in ("", "/download"):
        parameters = paths["/api/v1/processes/{process_id}/completion-exports/{job_id}" + suffix]["get"]["parameters"]
        selectors = {item["name"]: item for item in parameters}
        assert selectors["process_id"]["schema"]["$ref"] == "#/components/schemas/ProcessPathId"
        assert selectors["job_id"]["schema"]["pattern"] == "^[0-9a-f]{32}$"
        assert all(item["in"] == "path" and item["required"] for item in selectors.values())
    post = paths["/api/v1/completion-exports"]["post"]
    assert post["requestBody"]["content"]["application/json"]["schema"]["$ref"].endswith(
        "#/components/schemas/CompletionExportRequest")
    assert "parameters" not in post
    assert set(post["responses"]) >= {"202"} and "200" not in post["responses"]
    listing = paths["/api/v1/processes/{process_id}/completion-exports"]["get"]
    assert listing["responses"]["200"]["content"]["application/json"]["schema"]["$ref"].endswith(
        "#/components/schemas/CompletionExportJobList")


@pytest.mark.parametrize("code", ["export-forbidden", "export-refused", "export-conflict",
                                  "export-retries-exhausted", "export-receipt-missing"])
def test_failure_codes_are_the_closed_stable_set(checks, code):
    checks["CompletionExportJob"].validate(job("failed", failureCode=code))
    assert not checks["CompletionExportJob"].is_valid(job("failed", failureCode="export-invented"))
    assert not checks["CompletionExportJob"].is_valid(job("failed", failureCode="export-abandoned"))
    assert not checks["CompletionExportJob"].is_valid(job("running", attempt=1))
