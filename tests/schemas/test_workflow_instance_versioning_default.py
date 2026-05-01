"""workflow execution.instanceVersioning default and enum tests.

Guards three invariants from Kernel §9.6 and spec prose at line 1483:
  1. Omitted `instanceVersioning` is valid (documented default is `pinned`).
  2. Explicit `migrateable` validates.
  3. Misspelled `migratable` (wrong spelling; spec calls this out) is rejected.
"""

from __future__ import annotations

import json
from pathlib import Path

from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_SCHEMA = WOS_SPEC_ROOT / "schemas" / "wos-workflow.schema.json"


def _minimal_workflow() -> dict:
    """Minimal valid WOS workflow document."""
    return {
        "$wosWorkflow": "1.0",
        "url": "urn:wos:workflow:test:1.0.0",
        "version": "1.0.0",
        "title": "Test Workflow",
        "status": "active",
        "impactLevel": "operational",
        "actors": [{"id": "applicant", "type": "human"}],
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {"type": "atomic"},
            },
        },
    }


def test_instance_versioning_omitted_is_valid():
    """Omitted instanceVersioning is valid; documented default is `pinned`."""
    schema = json.loads(WORKFLOW_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_workflow()
    # Confirm `execution.instanceVersioning` is NOT present
    assert "execution" not in doc or "instanceVersioning" not in doc.get("execution", {})
    errors = list(validator.iter_errors(doc))
    assert errors == [], f"Workflow without instanceVersioning rejected: {errors}"


def test_instance_versioning_migrateable_is_valid():
    """Explicit `migrateable` is an accepted enum value."""
    schema = json.loads(WORKFLOW_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_workflow()
    doc["execution"] = {"instanceVersioning": "migrateable"}
    errors = list(validator.iter_errors(doc))
    assert errors == [], f"Workflow with instanceVersioning=migrateable rejected: {errors}"


def test_instance_versioning_pinned_is_valid():
    """Explicit `pinned` is an accepted enum value."""
    schema = json.loads(WORKFLOW_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_workflow()
    doc["execution"] = {"instanceVersioning": "pinned"}
    errors = list(validator.iter_errors(doc))
    assert errors == [], f"Workflow with instanceVersioning=pinned rejected: {errors}"


def test_instance_versioning_migratable_wrong_spelling_rejected():
    """Misspelled `migratable` (missing 'e') MUST be rejected.

    The spec explicitly calls out the correct spelling: `migrateable`.
    """
    schema = json.loads(WORKFLOW_SCHEMA.read_text())
    validator = Draft202012Validator(schema)
    doc = _minimal_workflow()
    doc["execution"] = {"instanceVersioning": "migratable"}
    errors = list(validator.iter_errors(doc))
    assert errors, (
        "instanceVersioning='migratable' (wrong spelling) must be rejected; "
        "correct spelling is 'migrateable'"
    )


def test_schema_declares_default_pinned():
    """Schema property carries `default: pinned` per Kernel §9.6 / spec line 1483."""
    schema = json.loads(WORKFLOW_SCHEMA.read_text())
    execution_props = (
        schema.get("$defs", {})
        .get("Execution", {})
        .get("properties", {})
    )
    if not execution_props:
        # Fallback: some schemas inline execution under properties directly
        execution_props = (
            schema.get("properties", {})
            .get("execution", {})
            .get("properties", {})
        )
    iv = execution_props.get("instanceVersioning", {})
    assert iv.get("default") == "pinned", (
        f"instanceVersioning schema property must carry default='pinned'; "
        f"got: {iv}"
    )
