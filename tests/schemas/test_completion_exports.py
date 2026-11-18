"""Completion export requests, public jobs and problems."""
import pytest
from jsonschema import Draft202012Validator, FormatChecker

from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR, API_SPEC_DIR, api_referencing_registry, load_openapi, load_schema,
    validator_for_api_def,
)

PROCESS = "default_process_01hw7rm71vfay8vvw14d2pf2db"
PROCESS_URN = "urn:wos:" + PROCESS
JOB_ID = "a" * 32
DECLARATIONS = {"purpose": "quality-review", "recipientClass": "oversight"}
ACTIVE_JOB_CONFLICT = "WOS-2409"
PROBLEM_URI = "https://schemas.formspec.io/wos-api/errors/"
STATES = {
    "running": {},
    "succeeded": {"checkpointRef": "retained-checkpoint"},
    "failed": {"failureCode": "export-retries-exhausted"},
}


def job(status, **overrides):
    return {"status": status, "processId": PROCESS_URN, "jobId": JOB_ID,
            **DECLARATIONS, **STATES[status], **overrides}


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
def test_request_accepts_actual_process_selector_and_declarations_only(checks, process):
    check = checks["CompletionExportRequest"]
    # The former process-only request now requires both authorization declarations.
    request = {"processId": process, **DECLARATIONS}
    check.validate(request)
    for field in ("jobId", "checkpointRef", "actor", "scope", "token", "Idempotency-Key"):
        assert not check.is_valid({**request, field: "invented"})


@pytest.mark.parametrize("value", [{}, {"processId": None}, {"processId": "invented"},
                                   {"processId": "urn:wos:" + PROCESS.replace("_process_", "_case_")},
                                   {"processId": PROCESS_URN + "/tasks/respond"}])
def test_request_refuses_missing_or_wrong_resource_identity(checks, value):
    assert not checks["CompletionExportRequest"].is_valid({**DECLARATIONS, **value})


@pytest.mark.parametrize("field", DECLARATIONS)
def test_request_requires_each_declaration(checks, field):
    request = {"processId": PROCESS, **DECLARATIONS}
    del request[field]
    assert not checks["CompletionExportRequest"].is_valid(request)
    assert not checks["CompletionExportRequest"].is_valid({"processId": PROCESS})


@pytest.mark.parametrize("field", DECLARATIONS)
def test_declaration_syntax_applies_to_requests_and_jobs(checks, field):
    request = {"processId": PROCESS, **DECLARATIONS}
    variants = [(checks["CompletionExportRequest"], request)]
    variants.extend((checks["CompletionExportJob"], job(status)) for status in STATES)
    for check, body in variants:
        # Syntax is shared and the values are open to deployment vocabularies.
        for value in ("a", "a" * 64, "review-2"):
            check.validate({**body, field: value})
        for value in (None, 1, "", "a" * 65, "Quality-review", "quality review",
                      "quality/review", "quality--review", "-review", "review-",
                      "2-review", "réview", "review\n"):
            assert not check.is_valid({**body, field: value}), (field, value)


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
    base = {"processId": PROCESS_URN, "jobId": JOB_ID, **DECLARATIONS}
    assert not checks["CompletionExportJob"].is_valid({**base, **value})


@pytest.mark.parametrize("status", STATES)
@pytest.mark.parametrize("job_id", [None, "a" * 31, JOB_ID + "\n", "A" * 32, "g" * 32, "../request"])
def test_job_identity_is_exact_lowercase_hex(checks, status, job_id):
    assert not checks["CompletionExportJob"].is_valid(job(status, jobId=job_id))


def test_job_list_is_bounded_closed_array_of_jobs(checks):
    check = checks["CompletionExportJobList"]
    check.validate({"jobs": []})
    check.validate({"jobs": [job(status) for status in STATES]})
    assert not check.is_valid({"jobs": [job("running")], "extra": 1})
    assert not check.is_valid({"jobs": [job("running")] * 51})
    incomplete = job("running")
    del incomplete["recipientClass"]
    assert not check.is_valid({"jobs": [incomplete]})


def test_active_job_conflict_has_a_registered_http_problem_shape():
    schema = load_schema(API_SCHEMA_DIR / "completion-exports.schema.json")
    check = validator_for_api_def(schema, "CompletionExportRequestConflict", api_referencing_registry())
    problem = {
        "type": PROBLEM_URI + ACTIVE_JOB_CONFLICT,
        "title": "Active completion export conflict",
        "status": 409,
        "error_code": ACTIVE_JOB_CONFLICT,
    }
    check.validate(problem)
    assert not check.is_valid({**problem, "status": 403})
    assert not check.is_valid({**problem, "error_code": "WOS-1409"})
    assert not check.is_valid({**problem, "type": PROBLEM_URI + "WOS-1403"})
    assert not check.is_valid({**problem, "jobId": JOB_ID})
    assert not check.is_valid({key: value for key, value in problem.items() if key != "title"})
    registry = (API_SPEC_DIR / "error-registry.md").read_text()
    assert f"| `{ACTIVE_JOB_CONFLICT}` | 409 |" in registry


def test_public_root_exports_the_active_job_conflict():
    schema = load_schema(API_SCHEMA_DIR / "completion-exports.schema.json")
    check = Draft202012Validator(schema, registry=api_referencing_registry(),
                                format_checker=FormatChecker())
    example, = schema["$defs"]["CompletionExportRequestConflict"]["examples"]
    check.validate(example)


@pytest.mark.parametrize("status, code, title, reason", [
    (403, "WOS-1403", "Forbidden", "policy_forbidden"),
    (503, "WOS-1503", "Service unavailable", "facts_invalid"),
    (503, "WOS-1503", "Service unavailable", "package_unavailable"),
])
def test_public_authorization_problem_requires_a_safe_reason(status, code, title, reason):
    schema = load_schema(API_SCHEMA_DIR / "completion-exports.schema.json")
    check = Draft202012Validator(schema, registry=api_referencing_registry(),
                                format_checker=FormatChecker())
    problem = {"type": PROBLEM_URI + code, "title": title, "status": status, "error_code": code}
    check.validate({**problem, "context": {"reason_code": reason}})
    assert not check.is_valid(problem)
    for context in ({}, {"reason_code": "allowed"}, {"reason_code": "invented"},
                    {"reason_code": reason, "policy_id": "private-policy"}):
        assert not check.is_valid({**problem, "context": context})


def test_status_reads_tell_clients_to_keep_polling_during_a_provider_outage():
    spec = (API_SPEC_DIR / "completion-exports.md").read_text()
    status_section = spec.split("## Status and download\n", 1)[1]
    read_spec = status_section.split("`GET /api/v1/processes/{process_id}/completion-exports/{job_id}/download`", 1)[0]
    outage_rule = next((line for line in read_spec.splitlines()
                        if "provider outage" in line.lower()), "")
    assert "503" in outage_rule, "status reads must map provider outages to unavailability"
    assert "keep polling" in outage_rule, "clients need the recovery rule for a status outage"


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
