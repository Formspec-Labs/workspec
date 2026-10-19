"""Diagnostic schema aliases resolve through the maintained offline registry."""

import json

import pytest
from jsonschema import Draft202012Validator
from referencing import Registry
from referencing.exceptions import Unresolvable

from tests.schemas.conftest import SCHEMAS_ROOT


def diagnostic():
    return {"ruleId": "WOS-EMBED-TARGET-001", "severity": "block", "tier": "T1",
            "path": "/agents/0/targetWorkflow", "message": "Remove embedded targetWorkflow.",
            "suggestedFix": {"kind": "remove-property", "path": "/agents/0/targetWorkflow"}}


def test_diagnostic_views_resolve_offline_and_reject_mismatched_inputs(registry):
    standalone = json.loads((SCHEMAS_ROOT / "lint/wos-lint-diagnostic.schema.json").read_text())
    canonical = {"$ref": "https://wos-spec.org/schemas/tooling/1.0#/$defs/lintDiagnostic__Root"}
    for schema in [standalone, canonical]:
        validator = Draft202012Validator(schema, registry=registry)
        validator.validate(diagnostic())
        for field, invalid in [("path", "$.agents[0]"), ("severity", "fatal")]:
            mismatched = {**diagnostic(), field: invalid}
            assert list(validator.iter_errors(mismatched))
    # An absent resource fails explicitly; the registry never retrieves over HTTP.
    with pytest.raises(Unresolvable, match="https://wos-spec.org/schemas/tooling/1.0"):
        Draft202012Validator(standalone, registry=Registry()).validate(diagnostic())


def test_synth_severity_alias_resolves_offline(registry):
    standalone = json.loads((SCHEMAS_ROOT / "synth/wos-synth-trace.schema.json").read_text())
    validator = Draft202012Validator(standalone["$defs"]["Severity"], registry=registry)
    for severity in ["block", "error", "warning", "info"]:
        validator.validate(severity)
    assert list(validator.iter_errors("fatal"))
