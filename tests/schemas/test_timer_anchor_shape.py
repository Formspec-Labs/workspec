"""Validate the native historical timer anchor at every lifecycle action site."""
import copy

import pytest


def workflow():
    return {
        "$wosWorkflow": "1.0",
        "url": "urn:wos:test:historical-timer",
        "version": "1.0.0",
        "title": "Historical timer schema fixture",
        "impactLevel": "operational",
        "actors": [{"id": "processor", "type": "system"}],
        "lifecycle": {
            "initialState": "waiting",
            "states": {
                "waiting": {
                    "type": "atomic",
                    "onEntry": [{
                        "action": "startTimer",
                        "timerId": "response",
                        "duration": "P10D",
                        "startAtExpression": "caseFile.notice.sentAt",
                        "event": {"kind": "timer", "timerId": "response", "source": "state"},
                    }],
                    "transitions": [{
                        "event": {"kind": "timer", "timerId": "response", "source": "state"},
                        "target": "done",
                    }],
                },
                "done": {"type": "final"},
            },
        },
    }


@pytest.mark.parametrize("site", ["onEntry", "onExit", "transition"])
@pytest.mark.parametrize("change", [
    {"action": "invokeService"},
    {"duration": None},
    {"startAtExpression": ""},
    {"startAtExpression": 42},
    {"deadline": "2026-10-08T12:00:00Z"},
])
def test_timer_anchor_shape_refuses_unsupported_combinations(validators, site, change):
    doc = workflow()
    validators["$wosWorkflow"].validate(doc)
    state = doc["lifecycle"]["states"]["waiting"]
    action = copy.deepcopy(state["onEntry"][0])
    for field, value in change.items():
        if value is None:
            action.pop(field)
        else:
            action[field] = value
    if site == "transition":
        state["transitions"][0]["actions"] = [action]
    else:
        state[site] = [action]
    assert not validators["$wosWorkflow"].is_valid(doc)


def test_generic_anchored_clock_validates(validators):
    validators["$wosWorkflow"].validate(workflow())


def test_unanchored_timer_remains_valid(validators):
    doc = workflow()
    del doc["lifecycle"]["states"]["waiting"]["onEntry"][0]["startAtExpression"]
    validators["$wosWorkflow"].validate(doc)
