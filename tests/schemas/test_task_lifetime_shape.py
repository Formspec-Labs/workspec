"""Explicit timer-bound lifetimes preserve legacy tasks and reject partial metadata."""
import copy
import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[2]


def workflow():
    return {
        "$wosWorkflow": "1.0", "url": "urn:wos:test:task-lifetime",
        "version": "1.0.0", "title": "Task lifetime schema fixture",
        "impactLevel": "operational", "actors": [{"id": "applicant", "type": "human"}],
        "lifecycle": {"initialState": "waiting", "states": {
            "waiting": {"type": "atomic", "onEntry": [{
                "action": "createTask", "taskRef": "provideInformation",
                "assignTo": "applicant", "expiresWithTimer": "responseWindow",
            }], "transitions": [{"event": {"kind": "message", "name": "response"}, "target": "done"}]},
            "done": {"type": "final"},
        }},
    }


@pytest.mark.parametrize("site", ["onEntry", "onExit", "transition"])
@pytest.mark.parametrize("change", [
    {"action": "invokeService"}, {"taskRef": None},
    {"expiresWithTimer": ""}, {"expiresWithTimer": 42},
])
def test_task_lifetime_refuses_wrong_action_or_missing_identity(validators, site, change):
    doc = workflow()
    validators["$wosWorkflow"].validate(doc)
    state = doc["lifecycle"]["states"]["waiting"]
    action = copy.deepcopy(state["onEntry"][0])
    for key, value in change.items():
        if value is None:
            action.pop(key)
        else:
            action[key] = value
    if site == "transition":
        state["transitions"][0]["actions"] = [action]
    else:
        state[site] = [action]
    assert not validators["$wosWorkflow"].is_valid(doc)


def test_legacy_task_without_explicit_lifetime_remains_valid(validators):
    doc = workflow()
    del doc["lifecycle"]["states"]["waiting"]["onEntry"][0]["expiresWithTimer"]
    validators["$wosWorkflow"].validate(doc)


def active_task_validator():
    schema = json.loads((ROOT / "schemas/wos-process.schema.json").read_text())
    return Draft202012Validator({
        "$schema": schema["$schema"], "$defs": schema["$defs"],
        **schema["$defs"]["ActiveTask"],
    }, format_checker=FormatChecker())


def task():
    return {
        "taskId": "task-1", "taskRef": "provideInformation", "status": "created",
        "createdAt": "2026-10-08T12:00:00Z", "updatedAt": "2026-10-08T12:00:00Z",
        "expiresAt": "2026-10-18T12:00:00Z", "expiresWithTimer": "responseWindow",
    }


def test_persisted_lifetime_pair_and_legacy_task_validate():
    validator = active_task_validator()
    doc = task()
    validator.validate(doc)
    del doc["expiresAt"]
    del doc["expiresWithTimer"]
    validator.validate(doc)


@pytest.mark.parametrize("change", [
    {"expiresAt": None}, {"expiresWithTimer": None},
    {"expiresAt": "not-a-timestamp"}, {"expiresWithTimer": ""},
])
def test_persisted_task_refuses_partial_or_malformed_lifetime(change):
    validator = active_task_validator()
    doc = task()
    validator.validate(doc)
    for key, value in change.items():
        if value is None:
            doc.pop(key)
        else:
            doc[key] = value
    assert not validator.is_valid(doc)
