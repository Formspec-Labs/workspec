"""Owning deferred callback shapes; runtime separately checks correlations."""
import copy
import json
from pathlib import Path

import pytest
from .conftest import validator_for_def

ROOT = Path(__file__).resolve().parents[2]


@pytest.mark.parametrize("phase", ["outbound", "inbound"])
def test_explicit_callback_phase_validates(phase):
    validator_for_def("LifecycleAction").validate(
        {"action": "invokeService", "serviceRef": "notice", "callbackPhase": phase})


@pytest.mark.parametrize("change", [
    {"callbackPhase": None}, {"callbackPhase": "incoming"},
    {"callbackPhase": 1}, {"action": "setData"}, {"serviceRef": " "},
])
def test_explicit_callback_phase_refuses_unsupported_actions(change):
    action = {"action": "invokeService", "serviceRef": "notice", "callbackPhase": "inbound"}
    action.update(change)
    assert not validator_for_def("LifecycleAction").is_valid(action)


def request():
    return {
        "definitionUrl": "urn:wos:test:deferred", "definitionVersion": "1.0.0",
        "bindingId": "notice", "invocationId": "process:notice:deferred:0", "subject": "subject:0",
        "request": {"id": "process:notice:deferred:0", "source": "urn:wos:test:provider",
                    "specversion": "1.0", "type": "request", "subject": "subject:0", "data": {}},
        "bindingSnapshot": {"type": "callback", "dispatchMode": "deferred",
                            "callbackEventType": "receipt", "dataMapping": {}, "outputBinding": {}},
    }


def test_typed_deferred_request_and_legacy_callback_shapes_validate():
    validator_for_def("PendingOutboundEvent").validate(request())
    validator_for_def("PendingCallback").validate({"bindingId": "notice", "invocationId": "old"})
    validator_for_def("IntegrationBindingSnapshot").validate({"type": "request-response"})
    # The maintained serializer emits nullable optional fields and empty maps.
    value = request()
    value["bindingSnapshot"].update(description=None, requestContract=None, responseContract=None,
                                    idempotencyKeyExpression=None, inputMapping={}, contextMapping={})
    validator_for_def("PendingOutboundEvent").validate(value)


@pytest.mark.parametrize("field", ["definitionUrl", "definitionVersion", "bindingId", "invocationId",
                                    "subject", "request", "bindingSnapshot"])
def test_deferred_request_refuses_missing_original_fields(field):
    value = request()
    value.pop(field)
    assert not validator_for_def("PendingOutboundEvent").is_valid(value)


@pytest.mark.parametrize("change", [
    {"dispatchMode": None}, {"dispatchMode": "queued"}, {"type": "request-response"},
    {"callbackEventType": None}, {"callbackEventType": " "}, {"callbackEventType": 1},
    {"dataMapping": []}, {"outputBinding": {"target": 1}},
])
def test_binding_snapshot_refuses_unsupported_deferred_interpretations(change):
    value = request()
    value["bindingSnapshot"].update(change)
    assert not validator_for_def("PendingOutboundEvent").is_valid(value)


@pytest.mark.parametrize("change", [{"id": ""}, {"subject": None}, {"specversion": "0.3"},
                                     {"unrecognizedAttribute": True}])
def test_retained_request_refuses_noncanonical_cloud_event_shape(change):
    value = request()
    value["request"].update(change)
    assert not validator_for_def("PendingOutboundEvent").is_valid(value)


def test_process_declares_omitted_defaults_and_real_pending_maps(validators):
    # Use a maintained minimal portable process fixture, without unrelated test DTOs.
    doc = {"$wosProcess": "1.0", "processId": "default_process_01hw7rm71vfay8vvw14d2pf2db",
           "caseLedgerId": "default_case_01hw7rm71vfay8vvw14d2pf2db",
           "tenant": "default",
           "definitionUrl": "urn:wos:test:deferred", "definitionVersion": "1.0.0",
           "configuration": ["waiting"], "caseState": {}, "provenancePosition": 0,
           "timers": [], "activeTasks": [], "status": "active",
           "createdAt": "2026-10-08T20:00:00Z", "updatedAt": "2026-10-08T20:00:00Z"}
    validators["$wosProcess"].validate(doc)
    doc.update(pendingCallbacks={"subject:0": {"bindingId": "notice", "invocationId": "process:notice:deferred:0",
                                              "dispatchMode": "deferred"}},
               pendingOutboundEvents={"subject:0": request()}, nextDeferredCallbackSequence=1)
    validators["$wosProcess"].validate(doc)
    for key, value in [("nextDeferredCallbackSequence", -1), ("nextDeferredCallbackSequence", 2**64),
                       ("pendingOutboundEvents", []), ("pendingCallbacks", None)]:
        malformed = copy.deepcopy(doc)
        malformed[key] = value
        assert not validators["$wosProcess"].is_valid(malformed)
    schema = json.loads((ROOT / "schemas/wos-process.schema.json").read_text())
    assert schema["properties"]["pendingCallbacks"]["default"] == {}
    assert schema["properties"]["pendingOutboundEvents"]["default"] == {}
    assert schema["properties"]["nextDeferredCallbackSequence"]["default"] == 0


def test_d1_authored_action_sites_validate(validators):
    # The maintained 0.3.0 workflow is the deferred lifecycle: notices advance on receipts.
    doc = json.loads((ROOT / "examples/d1-passport-complete.workflow.json").read_text())
    validators["$wosWorkflow"].validate(doc)
    # A mutation at the actual receipt-region site is rejected by the owning schema.
    receipt = doc["lifecycle"]["states"]["caseAndDelivery"]["regions"]["delivery"]["states"]["awaitingDeliveryReceipt"]
    receipt["transitions"][0]["actions"][0]["action"] = "setData"
    assert not validators["$wosWorkflow"].is_valid(doc)


def test_historical_deferred_d1_refuses_old_governance_and_retains_callback_action_shape(validators):
    # Historical authored bytes stay immutable; this is not a fresh D1 qualification.
    doc = json.loads((ROOT / "examples/d1-passport-complete-deferred.workflow.json").read_text())
    errors = list(validators["$wosWorkflow"].iter_errors(doc))
    assert len(errors) == 1
    assert list(errors[0].path) == ["governance", "dueProcess"]
    assert errors[0].validator == "additionalProperties"
    assert "explanationLevel" in errors[0].message
    receipt = doc["lifecycle"]["states"]["caseAndDelivery"]["regions"]["delivery"]["states"]["awaitingDeliveryReceipt"]
    action = receipt["transitions"][0]["actions"][0]
    validator = validator_for_def("LifecycleAction")
    validator.validate(action)
    action["action"] = "setData"
    assert not validator.is_valid(action)


def test_callback_phase_restrictions_apply_inside_compensation():
    action = {"action": "invokeService", "serviceRef": "notice",
              "compensatingAction": {"action": "invokeService", "serviceRef": "cancel",
                                     "callbackPhase": "outbound"}}
    validator_for_def("LifecycleAction").validate(action)
    action["compensatingAction"]["action"] = "setData"
    assert not validator_for_def("LifecycleAction").is_valid(action)


@pytest.mark.parametrize("site", ["onEntry", "onExit", "transition", "compensation", "nestedParallel"])
def test_parallel_region_actions_reach_owning_recursive_state_constraints(validators, site):
    action = {"action": "invokeService", "serviceRef": "notice", "callbackPhase": "outbound"}
    state = {"type": "atomic"}
    bad_action = action
    if site in ["onEntry", "onExit"]:
        state[site] = [action]
    elif site == "transition":
        state["transitions"] = [{"event": {"kind": "message", "name": "receipt"},
                                 "target": "waiting", "actions": [action]}]
    elif site == "compensation":
        state["onEntry"] = [{"action": "invokeService", "serviceRef": "outer",
                             "compensatingAction": action}]
    else:
        state = {"type": "parallel", "regions": {"inner": {
            "initialState": "leaf", "states": {"leaf": {"type": "atomic", "onEntry": [action]}}}}}
    doc = {"$wosWorkflow": "1.0", "url": "urn:wos:test:parallel-shape", "version": "1.0.0",
           "title": "Parallel action schema fixture", "impactLevel": "operational",
           "actors": [{"id": "processor", "type": "system"}],
           "lifecycle": {"initialState": "parallel", "states": {"parallel": {
               "type": "parallel", "regions": {"receipt": {"initialState": "waiting", "states": {"waiting": state}}}}}}}
    validators["$wosWorkflow"].validate(doc)
    bad_action["action"] = "setData"
    assert not validators["$wosWorkflow"].is_valid(doc)
