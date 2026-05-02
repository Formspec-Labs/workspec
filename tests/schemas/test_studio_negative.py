"""Negative-fixture coverage for Stage 3 Studio schema enforcement (Wave 1).

Each test asserts that the per-kind/per-state `allOf` clauses in the polymorphic
schemas REJECT documents missing the load-bearing fields the schemas claim to
require. Without these tests the per-kind enforcement is implementation-only —
no test would notice if an `allOf` clause regressed silently.

Tests are organized by schema; each test constructs a minimal-but-invalid
document and asserts validation produces ≥1 error containing the expected
substring.
"""
from __future__ import annotations

import pytest
from jsonschema import Draft202012Validator


def _validator(validators: dict[str, Draft202012Validator], marker: str) -> Draft202012Validator:
    """Resolve a registered marker to its compiled validator."""
    if marker not in validators:
        pytest.fail(f"marker {marker!r} not registered in conftest MARKER_TO_SCHEMA")
    return validators[marker]


def _assert_rejects(validator: Draft202012Validator, doc: dict, expected_substr: str) -> None:
    """Assert validator emits ≥1 error whose message contains expected_substr."""
    errors = list(validator.iter_errors(doc))
    assert errors, f"expected rejection of {doc!r}; got 0 errors"
    messages = [e.message for e in errors]
    matched = any(expected_substr in m for m in messages)
    assert matched, (
        f"expected error containing {expected_substr!r}; got: {messages[:3]}"
    )


# ---------------------------------------------------------------------------
# wos-studio-mapping per-state enforcement (Wave 1, fixes Agent 2 M2)
# ---------------------------------------------------------------------------

class TestMappingPerStateEnforcement:
    def test_mapsToWos_rejects_empty_targets(self, validators):
        v = _validator(validators, "$wosStudioMapping")
        doc = {
            "$wosStudioMapping": "1.0",
            "mapping": {
                "id": "map-x", "subjectPolicyObjectRef": "pol-x",
                "mappingState": "mapsToWos",
            },
        }
        _assert_rejects(v, doc, "targets")

    def test_requiresSpecExtension_rejects_missing_extensionRecord_and_ref(self, validators):
        v = _validator(validators, "$wosStudioMapping")
        doc = {
            "$wosStudioMapping": "1.0",
            "mapping": {
                "id": "map-x", "subjectPolicyObjectRef": "pol-x",
                "mappingState": "requiresSpecExtension",
            },
        }
        # Either extensionRecord OR extensionRecordRef MUST be present
        _assert_rejects(v, doc, "")

    def test_unmappedButApproved_rejects_missing_rationale(self, validators):
        v = _validator(validators, "$wosStudioMapping")
        doc = {
            "$wosStudioMapping": "1.0",
            "mapping": {
                "id": "map-x", "subjectPolicyObjectRef": "pol-x",
                "mappingState": "unmappedButApproved",
            },
        }
        _assert_rejects(v, doc, "unmappedRationale")

    def test_authoringOnly_passes_without_extras(self, validators):
        v = _validator(validators, "$wosStudioMapping")
        doc = {
            "$wosStudioMapping": "1.0",
            "mapping": {
                "id": "map-x", "subjectPolicyObjectRef": "pol-x",
                "mappingState": "authoringOnly",
            },
        }
        errors = list(v.iter_errors(doc))
        assert not errors, f"authoringOnly should accept minimal mapping; got {errors[:2]}"


# ---------------------------------------------------------------------------
# wos-studio-approval per-kind enforcement (Wave 1, fixes Agent 1 MAJOR-3)
# ---------------------------------------------------------------------------

class TestApprovalPerKindEnforcement:
    def test_ApprovalPackage_rejects_no_approvals(self, validators):
        v = _validator(validators, "$wosStudioApproval")
        doc = {"$wosStudioApproval": "1.0", "kind": "ApprovalPackage"}
        _assert_rejects(v, doc, "approvals")

    def test_ApprovalDecision_rejects_no_decision_block(self, validators):
        v = _validator(validators, "$wosStudioApproval")
        doc = {"$wosStudioApproval": "1.0", "kind": "ApprovalDecision"}
        _assert_rejects(v, doc, "decision")

    def test_ChangeImpactReport_rejects_no_triggerKind(self, validators):
        v = _validator(validators, "$wosStudioApproval")
        doc = {"$wosStudioApproval": "1.0", "kind": "ChangeImpactReport"}
        _assert_rejects(v, doc, "triggerKind")

    def test_ChangeImpactReport_rejects_no_affected_collection(self, validators):
        v = _validator(validators, "$wosStudioApproval")
        doc = {
            "$wosStudioApproval": "1.0", "kind": "ChangeImpactReport",
            "triggerKind": "policy-object-edit",
        }
        # has triggerKind but no affected* arrays — anyOf clause MUST reject
        _assert_rejects(v, doc, "")


# ---------------------------------------------------------------------------
# wos-studio-readiness per-kind enforcement (Wave 1, fixes Agent 1 MAJOR-3)
# ---------------------------------------------------------------------------

class TestReadinessPerKindEnforcement:
    def test_ValidationReport_rejects_no_findings(self, validators):
        v = _validator(validators, "$wosStudioReadiness")
        doc = {"$wosStudioReadiness": "1.0", "kind": "ValidationReport"}
        _assert_rejects(v, doc, "findings")

    def test_RuleRegistry_rejects_no_rules(self, validators):
        v = _validator(validators, "$wosStudioReadiness")
        doc = {"$wosStudioReadiness": "1.0", "kind": "RuleRegistry"}
        _assert_rejects(v, doc, "rules")

    def test_ValidationFinding_rejects_no_finding_block(self, validators):
        v = _validator(validators, "$wosStudioReadiness")
        doc = {"$wosStudioReadiness": "1.0", "kind": "ValidationFinding"}
        _assert_rejects(v, doc, "finding")


# ---------------------------------------------------------------------------
# wos-studio-workflow-intent body+kernelKind enforcement (Wave 1, fixes
# Agent 1 MAJOR-1 + Agent 2 m1)
# ---------------------------------------------------------------------------

class TestWorkflowIntentBodyEnforcement:
    def _doc_with_element(self, element: dict) -> dict:
        return {
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-x", "workspaceId": "ws-x", "title": "T",
            "impactLevel": "operational", "lifecycleState": "draft",
            "elements": [element],
        }

    def test_phase_rejects_missing_body(self, validators):
        v = _validator(validators, "$wosStudioWorkflowIntent")
        # phase element has no body field at all — was passing pre-Wave-1
        doc = self._doc_with_element({
            "id": "el-1", "kind": "phase", "name": "P", "bridge": {"stateName": "p"},
        })
        _assert_rejects(v, doc, "body")

    def test_phase_rejects_body_missing_contains(self, validators):
        v = _validator(validators, "$wosStudioWorkflowIntent")
        doc = self._doc_with_element({
            "id": "el-1", "kind": "phase", "name": "P",
            "bridge": {"stateName": "p"}, "body": {},
        })
        _assert_rejects(v, doc, "contains")

    def test_notice_rejects_missing_noticeRequirementRef(self, validators):
        v = _validator(validators, "$wosStudioWorkflowIntent")
        doc = self._doc_with_element({
            "id": "el-n", "kind": "notice", "name": "N",
            "bridge": {"noticeId": "n1"}, "body": {},
        })
        _assert_rejects(v, doc, "noticeRequirementRef")

    def test_step_rejects_missing_kernelKind(self, validators):
        v = _validator(validators, "$wosStudioWorkflowIntent")
        # step is ambiguous; bridge MUST carry kernelKind explicitly
        doc = self._doc_with_element({
            "id": "el-s", "kind": "step", "name": "S",
            "bridge": {"taskRef": "t1"}, "body": {},
        })
        _assert_rejects(v, doc, "kernelKind")

    def test_system_check_rejects_missing_kernelKind(self, validators):
        v = _validator(validators, "$wosStudioWorkflowIntent")
        doc = self._doc_with_element({
            "id": "el-sc", "kind": "system-check", "name": "SC",
            "bridge": {"taskRef": "t1"}, "body": {"checkPurpose": "x"},
        })
        _assert_rejects(v, doc, "kernelKind")

    def test_phase_with_body_passes(self, validators):
        v = _validator(validators, "$wosStudioWorkflowIntent")
        doc = self._doc_with_element({
            "id": "el-1", "kind": "phase", "name": "P",
            "bridge": {"stateName": "p"},
            "body": {"contains": ["el-2"]},
        })
        # may have other errors (dangling ref to el-2) but body+kernelKind enforcement
        # for phase should not fire
        errors = [e.message for e in v.iter_errors(doc)]
        assert not any("'body' is a required property" in m for m in errors)
        assert not any("'contains' is a required property" in m for m in errors)


# ---------------------------------------------------------------------------
# wos-studio-policy-object collection-form oneOf (Wave 1, fixes Agent 2 M3)
# ---------------------------------------------------------------------------

class TestPolicyObjectCollectionForm:
    def test_collection_form_validates(self, validators):
        v = _validator(validators, "$wosStudioPolicyObject")
        # collection wrapper: workspaceId + policyObjects[] (no per-child marker)
        doc = {
            "workspaceId": "ws-x",
            "policyObjects": [
                {"id": "pol-1", "kind": "Assumption", "lifecycleState": "draft",
                 "originClass": "assumption", "body": {"narrative": "x"}},
            ],
        }
        errors = list(v.iter_errors(doc))
        assert not errors, f"collection-form should validate; got {errors[:3]}"

    def test_single_form_still_validates(self, validators):
        v = _validator(validators, "$wosStudioPolicyObject")
        doc = {
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-1", "kind": "Assumption",
            "lifecycleState": "draft", "originClass": "assumption",
            "body": {"narrative": "x"},
        }
        errors = list(v.iter_errors(doc))
        assert not errors, f"single-form should still validate; got {errors[:3]}"

    def test_collection_rejects_missing_workspaceId(self, validators):
        v = _validator(validators, "$wosStudioPolicyObject")
        doc = {"policyObjects": [{"id": "p", "kind": "Assumption",
                                  "lifecycleState": "draft",
                                  "originClass": "assumption"}]}
        errors = list(v.iter_errors(doc))
        # Both single-form (missing $wosStudioPolicyObject) and collection-form
        # (missing workspaceId) should reject — neither branch matches
        assert errors
