"""Public request bodies and error codes describe what the reference server does.

The server takes the actor from the bearer, keys replay by the `Idempotency-Key`
header, mints process identity from the admitted scope (ADR 0168) and refuses
caller-selected identity; these schemas are its wire contract (its `wos-api`
types are generated from them), so an extra body field here is a field the
server silently ignores.
"""
import json
import re

import pytest
from jsonschema import Draft202012Validator

from .test_wos_api_schema_discipline import API_SCHEMA_DIR, ROOT

PROCESS_URN = "urn:wos:acme_process_01jqrpd32jf8xtx9qxkkv3rqsd"


def _validator(schema_file, name):
    schema = json.loads((API_SCHEMA_DIR / schema_file).read_text())
    return Draft202012Validator({"$defs": schema["$defs"], "$ref": f"#/$defs/{name}"})


@pytest.mark.parametrize("schema_file, name, valid, refused", [
    ("instance.schema.json", "WorkflowProcessCreateRequest",
     {"definitionUrl": "https://example.org/wf/timeoff"},
     [{"processId": PROCESS_URN}, {"tenant": "acme"}]),
    ("instance.schema.json", "EventSubmissionRequest",
     {"eventName": "submit"},
     [{"actorRef": "urn:wos:acme_actor_01jqrpd32jf8xtx9qxkkv3rqsd"},
      {"occurredAt": "2026-10-17T12:00:00Z"}, {"correlationKey": "group-1"}]),
    ("instance.schema.json", "ProcessLifecycleRequest",
     {},
     [{"actorRef": "urn:wos:acme_actor_01jqrpd32jf8xtx9qxkkv3rqsd"}]),
    ("case-view.schema.json", "CaseEventAppendRequest",
     {"event": "wos.governance.review_requested"},
     [{"actorId": "clerk"}, {"idempotencyToken": "retry-1"}]),
    ("applicant.schema.json", "ApplicantCaseListOptions",
     {},
     [{"actionNeeded": True}]),
])
def test_request_body_carries_no_field_the_server_ignores(schema_file, name, valid, refused):
    validator = _validator(schema_file, name)
    assert validator.is_valid(valid), list(validator.iter_errors(valid))
    for extra in refused:
        assert not validator.is_valid({**valid, **extra}), f"{name} accepted {sorted(extra)}"


def test_error_registry_lists_the_codes_the_server_emits():
    registry = (ROOT / "specs/api/error-registry.md").read_text()
    codes = set(re.findall(r"^\| `(WOS-\d{4})` \|", registry, re.MULTILINE))
    assert "WOS-1501" in codes, "the server answers WOS-1501 for storage that cannot run an operation"
    assert "WOS-1453" not in codes, "nothing emits WOS-1453"


def test_error_docs_name_the_wire_code_field():
    schema = json.loads((API_SCHEMA_DIR / "error.schema.json").read_text())
    code_field, = (name for name, field in schema["$defs"]["Problem"]["properties"].items()
                   if field.get("$ref") == "#/$defs/ErrorCode")
    spec = (ROOT / "specs/api/error.md").read_text()
    required_section = spec.split("`Problem` requires:\n", 1)[1].split("Optional fields:", 1)[0]
    required_fields = set(re.findall(r"^- `([^`]+)`:", required_section, re.MULTILINE))
    assert required_fields == set(schema["$defs"]["Problem"]["required"])
    registry = (ROOT / "specs/api/error-registry.md").read_text()
    assert f"`{code_field}`" in registry
