"""Workflow-process TypeID regression tests.

Guards the runtime-artifact Process schema after the greenfield split between
workflow execution identity and durable case-ledger identity. A root WOS
Process must carry a `processId` from the `process` TypeID family and a
`caseLedgerId` from the `case` TypeID family.
"""

from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
PROCESS_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "wos-process.schema.json"
)


def _minimal_instance() -> dict:
    return {
        "$wosProcess": "1.0",
        "processId": "sba-poc_process_01jqrpd32jf8xtx9qxkkv3rqsd",
        "caseLedgerId": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd",
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


def test_workflow_process_accepts_split_identity_typeids():
    schema = json.loads(PROCESS_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    errors = list(validator.iter_errors(_minimal_instance()))
    assert errors == [], f"valid WorkflowProcess rejected: {errors}"


def test_workflow_process_rejects_root_instance_id_bridge():
    schema = json.loads(PROCESS_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["instanceId"] = doc["processId"]
    errors = list(validator.iter_errors(doc))
    assert errors, "WorkflowProcess root instanceId bridge must be rejected"


def test_workflow_process_rejects_non_process_id_family():
    schema = json.loads(PROCESS_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["processId"] = "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd"
    errors = list(validator.iter_errors(doc))
    assert errors, "WorkflowProcess.processId must use the `process` family"


def test_workflow_process_rejects_swapped_identity_families():
    schema = json.loads(PROCESS_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["processId"] = "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd"
    doc["caseLedgerId"] = "sba-poc_process_01jqrpd32jf8xtx9qxkkv3rqsd"
    errors = list(validator.iter_errors(doc))
    assert errors, "processId and caseLedgerId must use their reserved families"


def test_workflow_process_rejects_non_case_ledger_id_family():
    schema = json.loads(PROCESS_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["caseLedgerId"] = "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd"
    errors = list(validator.iter_errors(doc))
    assert errors, "WorkflowProcess.caseLedgerId must use the `case` family"


def test_formspec_task_context_accepts_process_id():
    schema = json.loads(PROCESS_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["activeTasks"] = [
        {
            "taskId": "task-1",
            "taskRef": "complete-intake",
            "status": "created",
            "context": {
                "taskId": "task-1",
                "processId": "sba-poc_process_01jqrpd32jf8xtx9qxkkv3rqsd",
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


def test_workflow_process_omitted_tenant_still_validates():
    """Older persisted rows and minimal hand-authored fixtures may omit tenant."""
    schema = json.loads(PROCESS_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    del doc["tenant"]
    errors = list(validator.iter_errors(doc))
    assert errors == [], f"WorkflowProcess without tenant rejected: {errors}"


def test_workflow_process_rejects_invalid_tenant_pattern():
    schema = json.loads(PROCESS_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["tenant"] = "INVALID"
    errors = list(validator.iter_errors(doc))
    assert errors, "tenant must match ADR 0068 D-1.1 DNS-label grammar"


def test_workflow_process_accepts_default_tenant_literal():
    schema = json.loads(PROCESS_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["processId"] = "default_process_01hw7rm71vfay8vvw14d2pf2db"
    doc["caseLedgerId"] = "default_case_01hw7rm71vfay8vvw14d2pf2db"
    doc["tenant"] = "default"
    errors = list(validator.iter_errors(doc))
    assert errors == [], f"valid default-tenant WorkflowProcess rejected: {errors}"
