"""Native AI structural agreement; semantic admission and execution have separate owners."""
import json

import pytest
from referencing import Registry
from referencing.exceptions import Unresolvable

from .conftest import SCHEMAS_ROOT, WOS_SPEC_ROOT, validator_for_def
from .test_wos_api_schema_discipline import api_referencing_registry, validator_for_api_def

REPORT = {"overall": 0.9, "method": "declared", "fieldLevel": {"amount": 0.8}}
AGENT = {"id": "assessor", "agentType": "generative", "modelIdentifier": "test-model", "modelVersion": "1.0.0"}
RULES = [
    {"id": "fields", "kind": "permission", "allowedFields": ["", "/amount~1net", "/tilde~0key"], "onViolation": "reject"},
    {"id": "block", "kind": "prohibition", "condition": "output.amount < 0", "onViolation": "reject"},
    {"id": "reason", "kind": "obligation", "requirement": "output.reason != null", "onViolation": "escalateToHuman"},
    {"id": "input", "kind": "right", "entitlement": "/caseFile/application"},
]


def valid(name, value):
    errors = list(validator_for_def(name).iter_errors(value))
    assert not errors, [(list(e.path), e.message) for e in errors]


def invalid(name, value):
    assert list(validator_for_def(name).iter_errors(value)), (name, value)


@pytest.mark.parametrize("rule", RULES, ids=[v["kind"] for v in RULES])
def test_kind_specific_native_rules(rule):
    valid("DeonticConstraint", {**rule, "citationRefs": ["42 CFR 431.211"], "x-note": {"kept": True}})


def test_complete_native_agent_settings_are_structural_requirements_only():
    agent = {**AGENT, "capabilities": [{"id": "assess", "preconditions": ["input != null"]}],
             "modelVersionPolicy": "approved", "approvedVersions": ["1.0.0"],
             "invoker": {"kind": "stub", "responses": [{"output": {}, "confidenceReport": REPORT}]},
             "deonticConstraints": RULES, "confidenceFloor": {"threshold": 0.8, "onViolation": "reject"},
             "confidenceDecay": {"enabled": True, "triggers": [{"event": "case.updated", "decayFactor": 0.9}]},
             "fallbackChain": [{"action": "retry", "maxRetries": 3, "backoff": "fixed", "initialInterval": "PT1S"}, {"action": "alternateAgent", "alternateAgentRef": "other"}, {"action": "escalateToHuman", "taskActionRef": "/lifecycle/states/review/onEntry/0"}],
             "calibration": {"required": True, "frequency": "P30D", "minimumSamples": 10, "method": "binning"},
             "autonomyPolicy": {"maxAutonomy": "supervisory", "demotion": [{"id": "drift", "condition": "true", "targetLevel": "assistive"}], "escalation": [{"id": "restore", "condition": "true", "targetLevel": "supervisory", "requiredApproverRef": "human", "escalationExpiry": "PT5M"}]},
             "actionOverrides": [{"actionRef": "/lifecycle/states/review/onEntry/0", "reviewWindow": "PT5M", "confidenceFloor": {"threshold": 0.9, "onViolation": "escalateToHuman"}, "deonticConstraints": RULES}],
             "driftMonitoring": {"monitors": [{"evaluationWindow": "P30D", "metrics": [{"name": "accuracy", "method": "accuracy", "threshold": 0.9}], "rubberStamp": {"enabled": True, "minReviewTime": "PT30S"}, "alertThresholds": [{"condition": "true", "severity": "warning", "policyRef": "drift"}]}], "deploymentSequence": {"enabled": True, "shadowDuration": "P1D", "canaryPercentage": 0.05, "canaryDuration": "P1D"}, "extensions": {"x-team": "quality"}}}
    valid("AgentDeclaration", agent)
    valid("AIOversight", {"defaultAutonomy": "assistive", "deonticConstraints": RULES, "confidenceFloor": {"threshold": 0.7, "onViolation": "reject"}, "fallbackChain": [{"action": "fail"}], "volumeConstraints": {"maxAutonomousPerHour": 0}, "disclosure": {"discloseThatAgentAssisted": True}})


@pytest.mark.parametrize("field,value", [("type", "agent"), ("autonomy", "assistive"), ("modelVersionPolicy", "minor-allowed"), ("capabilities", ["assess"]), ("confidenceFloor", 0.8), ("fallbackChain", ["human"]), ("calibration", {"required": True}), ("confidenceDecay", {"enabled": True}), ("driftMonitoring", {"monitors": [], "extensions": {"not-extension": True}})])
def test_retired_or_incomplete_agent_configuration_is_rejected(field, value):
    invalid("AgentDeclaration", {**AGENT, field: value})


@pytest.mark.parametrize("field", ["id", "agentType", "modelIdentifier", "modelVersion"])
def test_native_agent_identity_is_explicit(field):
    value = dict(AGENT)
    del value[field]
    invalid("AgentDeclaration", value)


@pytest.mark.parametrize("field", ["oversightExtensions", "reviewSampling", "narrativeTier", "driftDetection", "assistGovernanceProxy"])
def test_reserved_shared_configuration_is_rejected_even_empty(field):
    invalid("AIOversight", {field: {}})


@pytest.mark.parametrize("name,value", [
    ("DeonticConstraint", {"id": "generic", "kind": "prohibition", "expression": "true"}),
    ("DeonticConstraint", {**RULES[3], "onViolation": "reject"}),
    ("DeonticConstraint", {**RULES[0], "allowedFields": ["/bad~2escape"]}),
    ("DeonticConstraint", {**RULES[3], "entitlement": "/caseFileExtra"}),
    ("DeonticConstraint", {"id": "vacuous", "kind": "permission", "onViolation": "reject"}),
    ("ConfidenceReport", None),
    ("ConfidenceReport", {"overall": 1}),
    ("ConfidenceReport", {"overall": 0.9, "method": "declared", "explanation": None}),
    ("ConfidenceReport", {"overall": 0.9, "method": "declared", "fieldLevel": None}),
    ("ConfidenceReport", {**REPORT, "fieldLevel": {"amount": 1.1}}),
    ("ConfidenceReport", {"confidence": 1, "method": "declared"}),
    ("InvokerSpec", {"kind": "anthropic", "model": "second-model"}),
    ("InvokerSpec", {"kind": "a2A", "orchestrator": "service"}),
    ("InvokerSpec", {"kind": "stub", "responses": [{"output": {}}]}),
    ("InvokerSpec", {"kind": "stub", "responses": [{"output": {}, "confidence": 1}]}),
    ("FallbackStep", {"action": "escalateToHuman", "taskRef": "humanReview"}),
    ("FallbackStep", {"action": "retry", "maxRetries": 4294967296, "backoff": "fixed", "initialInterval": "PT1S"}),
    ("ActionOverride", {"actionRef": ""}),
    ("AgentAlertThreshold", {"condition": "true", "severity": "critical", "action": "notify", "policyRef": "drift"}),
    ("AgentAlertThreshold", {"condition": "true", "severity": "critical"}),
    ("AgentDeploymentSequence", {"enabled": True}),
    ("RubberStampConfig", {"enabled": True}),
    ("AgentPositiveDuration", "PT0S"),
])
def test_structural_negative_controls(name, value):
    invalid(name, value)


def test_a2a_uses_one_native_substrate_label():
    valid("InvokerSpec", {"kind": "a2a", "orchestrator": "service"})


def test_api_report_and_nested_records_resolve_canonical_report_offline():
    api = json.loads((SCHEMAS_ROOT / "api/provenance.schema.json").read_text())
    assert api["$defs"]["ConfidenceReport"]["$ref"] == "https://wos-spec.org/schemas/wos-workflow.schema.json#/$defs/ConfidenceReport"
    registry = api_referencing_registry()
    canonical = validator_for_api_def(api, "ConfidenceReport", registry)
    canonical.validate(REPORT)
    for bad in [0.9, {"overall": 0.9}, {"overall": 0.9, "method": "provider"}, {**REPORT, "fieldLevel": []}]:
        assert not canonical.is_valid(bad)
    records = {
        "ReasoningTierRecord": {"tier": "reasoning", "rulesApplied": [{"ruleId": "rule-1"}], "evidenceConsulted": [{"kind": "caseField", "caseFieldPath": "/amount"}], "criteriaChecked": []},
        "NarrativeTierRecord": {"tier": "narrative", "narrative": "Agent-assisted explanation.", "authoritative": False, "modelIdentifier": "test-model", "modelVersion": "1.0.0"},
    }
    base = {"id": "urn:wos:agency-gov_prov_01jqrxabcd3f8xtx9qxkkv3raa", "processId": "urn:wos:sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsc", "timestamp": "2026-04-23T12:00:00Z", "definitionVersion": "1.0.0", "event": "case.assessed", "confidenceReport": REPORT}
    for name, extra in records.items():
        doc = {**base, **extra}
        validator_for_api_def(api, name, registry).validate(doc)
        with pytest.raises(Unresolvable, match="wos-workflow.schema.json"):
            validator_for_api_def(api, name, registry.remove("https://wos-spec.org/schemas/wos-workflow.schema.json")).validate(doc)
        for bad_report in [None, {"confidence": 1}]:
            assert not validator_for_api_def(api, name, registry).is_valid({**doc, "confidenceReport": bad_report})
    with pytest.raises(Unresolvable, match="wos-workflow.schema.json"):
        validator_for_api_def(api, "ConfidenceReport", Registry()).validate(REPORT)


def test_advanced_calibration_remains_a_separate_definition():
    schema = json.loads((SCHEMAS_ROOT / "wos-workflow.schema.json").read_text())
    assert schema["$defs"]["AgentDeclaration"]["properties"]["calibration"]["$ref"] == "#/$defs/AgentCalibrationConfig"
    assert "required" not in schema["$defs"]["CalibrationConfig"]["properties"]


def test_shared_oversight_pairing_preserves_incremental_drafts(validators):
    workflow = {"$wosWorkflow": "1.0", "url": "https://example.test/native-ai", "version": "1.0.0", "title": "Native AI draft", "impactLevel": "operational", "actors": [{"id": "assessor", "type": "agent"}], "agents": [AGENT], "lifecycle": {"initialState": "done", "states": {"done": {"type": "final"}}}}
    validator = validators["$wosWorkflow"]
    validator.validate(workflow)
    assert not validator.is_valid({**workflow, "advanced": {}})
    validator.validate({**workflow, "advanced": {}, "aiOversight": {}})


def test_maintained_benefits_example_uses_native_ai_blocks():
    source = json.loads((WOS_SPEC_ROOT / "examples/benefits-adjudication.workflow.json").read_text())
    for agent in source["agents"]:
        valid("AgentDeclaration", agent)
        assert any(actor["id"] == agent["id"] and actor["type"] == "agent" for actor in source["actors"])
    valid("AIOversight", source["aiOversight"])
