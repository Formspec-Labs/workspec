"""Kernel §11.3: the workflow, not the caller, decides whether suspend and terminate require a reason."""
import pytest


def _workflow(**policies):
    return {
        "$wosWorkflow": "1.0",
        "url": "urn:wos:test:lifecycle-operation-policy",
        "version": "1.0.0",
        "title": "Lifecycle operation policy",
        "impactLevel": "operational",
        "actors": [{"id": "clerk", "type": "human"}],
        "lifecycle": {
            "initialState": "open",
            "states": {
                "open": {"type": "atomic", "transitions": [{"target": "closed", "actor": "clerk"}]},
                "closed": {"type": "final", "outcomeCode": "closed"},
            },
            **policies,
        },
    }


def test_workflow_declares_suspend_and_terminate_reason_policies(validators):
    document = _workflow(suspend={"reasonRequired": True}, terminate={"reasonRequired": False})
    errors = list(validators["$wosWorkflow"].iter_errors(document))
    assert not errors, "\n".join(error.message for error in errors)


@pytest.mark.parametrize("policy", [
    {"reasonRequired": "yes"},
    {"reasonRequired": True, "waivable": True},
])
def test_operation_policy_is_closed(validators, policy):
    assert not validators["$wosWorkflow"].is_valid(_workflow(suspend=policy))
