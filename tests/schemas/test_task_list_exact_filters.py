"""Exact task selection stays optional and distinct from a task's form pin."""
import pytest

from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR, load_schema, api_referencing_registry, validator_for_api_def,
    load_openapi,
)

SCHEMA = load_schema(API_SCHEMA_DIR / "task.schema.json")
VALIDATOR = validator_for_api_def(SCHEMA, "TaskListOptions", api_referencing_registry())


@pytest.mark.parametrize("query", [
    {},
    {"workflowVersion": "1.0.0"},
    {"taskRef": "intakeApplication"},
    {"workflowUrl": "urn:workflow:shared", "workflowVersion": "2.0.0",
     "taskRef": "review", "limit": 20},
])
def test_exact_task_filters_accept_optional_independent_and_combined_selection(query):
    VALIDATOR.validate(query)


@pytest.mark.parametrize("name", ["workflowVersion", "taskRef"])
@pytest.mark.parametrize("value", [None, "", "  ", "\t", 42, ["review"]])
def test_exact_task_filters_refuse_invalid_identifiers(name, value):
    assert not VALIDATOR.is_valid({name: value})


def test_task_form_version_is_not_a_workflow_query_alias():
    assert not VALIDATOR.is_valid({"definitionVersion": "1.0.0"})
    assert not VALIDATOR.is_valid({"definitionPinVersion": "1.0.0"})


@pytest.mark.parametrize("route", [
    "/tasks", "/cases/{case_id}/processes/{process_id}/tasks",
])
def test_openapi_task_routes_expose_the_owning_optional_filters(route):
    parameters = {p["name"]: p for p in load_openapi()["paths"][route]["get"]["parameters"]}
    for name in ["workflowVersion", "taskRef"]:
        parameter = parameters[name]
        assert parameter["in"] == "query"
        assert parameter["required"] is False
        assert parameter["schema"]["$ref"] == (
            f"{SCHEMA['$id']}#/$defs/TaskListOptions/properties/{name}"
        )
