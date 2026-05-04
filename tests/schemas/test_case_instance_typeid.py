"""CaseInstance TypeID regression tests.

Guards the runtime-artifact CaseInstance schema after T1-3 tightened
`instanceId` to the custody-bound `case` TypeID family. The same pattern
also applies to Formspec task context handoff data nested under the
instance schema.
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


def test_case_instance_rejects_non_case_typeid():
    schema = json.loads(CASE_INSTANCE_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_instance()
    doc["instanceId"] = "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd"
    errors = list(validator.iter_errors(doc))
    assert errors, "CaseInstance.instanceId must use the reserved `case` family prefix"
