"""Agent declaration references preserve the native nonempty authored identifier."""
import pytest
from jsonschema import Draft202012Validator

from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR, SCHEMA_DIR, load_schema, api_referencing_registry, validator_for_api_def,
)

SCHEMA = load_schema(API_SCHEMA_DIR / "governance.schema.json")
REGISTRY = api_referencing_registry()
NATIVE = load_schema(SCHEMA_DIR / "wos-workflow.schema.json")["$defs"]["AgentDeclaration"]["properties"]["id"]


@pytest.mark.parametrize("identifier", ["docExtractor", "  agent/解析:#  ", " ", "\t"])
def test_agent_references_accept_exact_native_identifier(identifier):
    Draft202012Validator(NATIVE).validate(identifier)
    for definition, field in [("AgentView", "id"), ("AutonomyEvent", "agentId")]:
        check = Draft202012Validator({"$defs": SCHEMA["$defs"], **SCHEMA["$defs"][definition]["properties"][field]}, registry=REGISTRY)
        check.validate(identifier)


@pytest.mark.parametrize("identifier", ["", None, 42, [], {}])
def test_agent_references_refuse_invalid_native_identifier(identifier):
    assert not Draft202012Validator(NATIVE).is_valid(identifier)
    for definition, field in [("AgentView", "id"), ("AutonomyEvent", "agentId")]:
        check = Draft202012Validator({"$defs": SCHEMA["$defs"], **SCHEMA["$defs"][definition]["properties"][field]}, registry=REGISTRY)
        assert not check.is_valid(identifier)
