"""CaseInstance TypeID regression tests.

Guards the runtime-artifact CaseInstance schema after T1-3 tightened
`instanceId` to WOS TypeID families. The transitional bridge accepts the
legacy `case` family and the new workflow `process` family while the explicit
`caseLedgerId` and `processId` fields carry the split identity.
"""

from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
CASE_INSTANCE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "wos-case-instance.schema.json"
)


def _minimal_instance() -> dict:
    return {
        "$wosCaseInstance": "1.0",
        "instanceId": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd",
        "tenant": "sba-poc",
        "definitionUrl": "https://agency.gov/workflows/benefits-adjudication",
        "definitionVersion": "1.0.0",
        "configuration": ["intake"],
        "caseState": {},
        "provenancePosition": 0,
        "timers": [],
        "activeTasks": [],
        "status": "active",
        "createdAt": "2026-04-21T12:00:00Z",
        "updatedAt": "2026-04-21T12:00:00Z",
    }


def test_case_instance_accepts_case_typeid():
    schema = json.loads(CASE_INSTANCE_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    errors = list(validator.iter_errors(_minimal_instance()))
    assert errors == [], f"valid CaseInstance rejected: {errors}"


def test_case_instance_accepts_process_typeid():
    schema = json.loads(CASE_INSTANCE_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["instanceId"] = "sba-poc_process_01jqrpd32jf8xtx9qxkkv3rqsd"
    doc["caseLedgerId"] = "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd"
    errors = list(validator.iter_errors(doc))
    assert errors == [], f"valid process-backed CaseInstance rejected: {errors}"


def test_case_instance_accepts_explicit_dual_identity_fields():
    schema = json.loads(CASE_INSTANCE_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["processId"] = "sba-poc_process_01jqrpd32jf8xtx9qxkkv3rqsd"
    doc["caseLedgerId"] = "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd"
    errors = list(validator.iter_errors(doc))
    assert errors == [], f"valid dual-identity CaseInstance rejected: {errors}"


def test_case_instance_rejects_non_runtime_typeid():
    schema = json.loads(CASE_INSTANCE_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["instanceId"] = "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd"
    errors = list(validator.iter_errors(doc))
    assert errors, "CaseInstance.instanceId must use `case` or `process`"


def test_case_instance_rejects_swapped_dual_identity_families():
    schema = json.loads(CASE_INSTANCE_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["processId"] = "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd"
    doc["caseLedgerId"] = "sba-poc_process_01jqrpd32jf8xtx9qxkkv3rqsd"
    errors = list(validator.iter_errors(doc))
    assert errors, "processId and caseLedgerId must use their reserved families"


def test_formspec_task_context_accepts_process_instance_id():
    schema = json.loads(CASE_INSTANCE_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["activeTasks"] = [
        {
            "taskId": "task-1",
            "taskRef": "complete-intake",
            "status": "created",
            "context": {
                "taskId": "task-1",
                "instanceId": "sba-poc_process_01jqrpd32jf8xtx9qxkkv3rqsd",
                "contractRef": "intakeApplication",
                "definitionUrl": "urn:formspec:intake",
                "definitionVersion": "1.0.0",
                "binding": "formspec",
                "assignedActor": "applicant-123",
            },
            "createdAt": "2026-04-21T12:00:00Z",
            "updatedAt": "2026-04-21T12:00:00Z",
        }
    ]
    errors = list(validator.iter_errors(doc))
    assert errors == [], f"valid process task context rejected: {errors}"


def test_case_instance_omitted_tenant_still_validates():
    """Older persisted rows and minimal hand-authored fixtures may omit tenant."""
    schema = json.loads(CASE_INSTANCE_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    del doc["tenant"]
    errors = list(validator.iter_errors(doc))
    assert errors == [], f"CaseInstance without tenant rejected: {errors}"


def test_case_instance_rejects_invalid_tenant_pattern():
    schema = json.loads(CASE_INSTANCE_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["tenant"] = "INVALID"
    errors = list(validator.iter_errors(doc))
    assert errors, "tenant must match ADR 0068 D-1.1 DNS-label grammar"


def test_case_instance_accepts_default_tenant_literal():
    schema = json.loads(CASE_INSTANCE_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["instanceId"] = "default_case_01hw7rm71vfay8vvw14d2pf2db"
    doc["tenant"] = "default"
    errors = list(validator.iter_errors(doc))
    assert errors == [], f"valid default-tenant CaseInstance rejected: {errors}"
