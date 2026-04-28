"""TaskPattern SLA authoring shape regression tests (#40 / Governance §10.4).

Guards the four new `TaskPattern` authoring properties landed on
`schemas/governance/wos-workflow-governance.schema.json`:
`slaDefinitions`, `warningThresholds`, `breachPolicy`, and
`escalationChain`. Runtime Companion §10.3 specifies the processor
semantics these properties author against; cross-reference integrity is
handled by Tier 2 lint rules (calendarRef via G-023, template keys via
G-063, escalation-step ids via G-066) and is deliberately NOT covered here.
"""
from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
GOVERNANCE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "wos-workflow.schema.json"
)


@pytest.fixture(scope="module")
def schema() -> dict:
    return json.loads(GOVERNANCE_SCHEMA.read_text())


def _validator_for_def(schema: dict, def_name: str) -> Draft202012Validator:
    """Build a validator that evaluates one $def with the parent $defs visible."""
    target = schema["$defs"][def_name]
    composed = {
        "$schema": schema.get("$schema", "https://json-schema.org/draft/2020-12/schema"),
        "$id": f"{schema.get('$id', 'urn:test')}#${def_name}",
        "$defs": schema["$defs"],
        **target,
    }
    return Draft202012Validator(composed)


def _minimal_task_with_all_four() -> dict:
    """TaskPattern carrying all four SLA authoring properties, valid shape."""
    return {
        "pattern": "incomeVerification",
        "verifiable": "yes",
        "slaDefinitions": [
            {
                "id": "firstResponse",
                "expectedDuration": "PT4H",
                "calendarType": "wall-clock",
                "startAt": "assignment",
            },
            {
                "id": "fullResolution",
                "expectedDuration": "P5BD",
                "calendarType": "business",
                "calendarRef": "urn:wos:sidecar:business-calendar:fy2026-federal",
                "startAt": "custom-event",
                "startEvent": "applicantResponseReceived",
            },
        ],
        "warningThresholds": [
            {
                "beforeBreach": "P1D",
                "templateKey": "slaWarning1Day",
                "notify": ["taskOwner"],
            }
        ],
        "breachPolicy": {
            "action": "escalate",
            "escalationStepId": "level-1",
            "timeoutPolicy": {"onRepeatedBreach": "suspend"},
        },
        "escalationChain": [
            {
                "level": 1,
                "assignTo": "teamLead",
                "gracePeriod": "PT4H",
                "onExhaustion": "escalate",
            },
            {
                "level": 2,
                "assignTo": "divisionDirector",
                "gracePeriod": "P1D",
                "onExhaustion": "ticketCreate",
            },
        ],
    }


class TestTaskPatternSlaRoundTrip:
    def test_all_four_properties_round_trip(self, schema):
        v = _validator_for_def(schema, "TaskPattern")
        errors = list(v.iter_errors(_minimal_task_with_all_four()))
        assert errors == [], f"valid task with full SLA block rejected: {errors}"

    def test_all_four_properties_absent_round_trips(self, schema):
        """Optionality: a TaskPattern with no SLA properties still validates."""
        v = _validator_for_def(schema, "TaskPattern")
        errors = list(
            v.iter_errors(
                {
                    "pattern": "applicationCompleteness",
                    "verifiable": "yes",
                }
            )
        )
        assert errors == [], f"bare TaskPattern rejected: {errors}"


class TestSlaDefinitionExpectedDuration:
    @pytest.mark.parametrize("value", ["P1D", "PT4H", "P5BD"])
    def test_valid_durations_accepted(self, schema, value):
        v = _validator_for_def(schema, "SlaDefinition")
        errors = list(
            v.iter_errors(
                {
                    "id": "firstResponse",
                    "expectedDuration": value,
                    "calendarType": "wall-clock",
                    "startAt": "assignment",
                }
            )
        )
        assert errors == [], f"expectedDuration={value!r} rejected: {errors}"

    def test_eventually_rejected(self, schema):
        v = _validator_for_def(schema, "SlaDefinition")
        errors = list(
            v.iter_errors(
                {
                    "id": "firstResponse",
                    "expectedDuration": "eventually",
                    "calendarType": "wall-clock",
                    "startAt": "assignment",
                }
            )
        )
        assert errors, "expectedDuration='eventually' must fail — not an ISO 8601 duration"

    def test_expected_duration_rejects_indefinite(self, schema):
        """`indefinite` is valid on HoldPolicy.expectedDuration but MUST be rejected on
        SlaDefinition.expectedDuration — an indefinite SLA has no elapse point for
        warningThresholds or breachPolicy to fire against (Review D #40a)."""
        v = _validator_for_def(schema, "SlaDefinition")
        errors = list(
            v.iter_errors(
                {
                    "id": "firstResponse",
                    "expectedDuration": "indefinite",
                    "calendarType": "wall-clock",
                    "startAt": "assignment",
                }
            )
        )
        assert errors, (
            "SlaDefinition.expectedDuration='indefinite' must fail — "
            "unlike HoldPolicy, SLAs have no semantic for open-ended windows"
        )

    def test_start_event_rejects_dollar_prefix(self, schema):
        """startEvent MUST reject `$`-prefixed reserved kernel event names
        (e.g. `$continuous`, `$join`, `$timeout.*`) — those are not valid
        clock origins for an SLA (Review D #40b)."""
        v = _validator_for_def(schema, "SlaDefinition")
        errors = list(
            v.iter_errors(
                {
                    "id": "firstResponse",
                    "expectedDuration": "PT4H",
                    "calendarType": "wall-clock",
                    "startAt": "custom-event",
                    "startEvent": "$continuous",
                }
            )
        )
        assert errors, (
            "SlaDefinition.startEvent='$continuous' must fail — the "
            "kernel event-name grammar forbids $-prefixed reserved names"
        )

    def test_start_event_rejects_whitespace(self, schema):
        """startEvent MUST reject names containing whitespace or other
        characters outside the kernel event-name grammar (Review D #40b)."""
        v = _validator_for_def(schema, "SlaDefinition")
        errors = list(
            v.iter_errors(
                {
                    "id": "firstResponse",
                    "expectedDuration": "PT4H",
                    "calendarType": "wall-clock",
                    "startAt": "custom-event",
                    "startEvent": "hello world",
                }
            )
        )
        assert errors, (
            "SlaDefinition.startEvent='hello world' must fail — "
            "whitespace is outside `^[a-zA-Z][a-zA-Z0-9_-]*$`"
        )

    def test_custom_event_requires_start_event(self, schema):
        """startAt = custom-event MUST require a startEvent."""
        v = _validator_for_def(schema, "SlaDefinition")
        errors = list(
            v.iter_errors(
                {
                    "id": "firstResponse",
                    "expectedDuration": "PT4H",
                    "calendarType": "wall-clock",
                    "startAt": "custom-event",
                }
            )
        )
        assert errors, (
            "SlaDefinition with startAt='custom-event' but no startEvent must fail — "
            "the custom-event clock origin is undefined without an event name"
        )


class TestWarningThresholdBeforeBreach:
    @pytest.mark.parametrize("value", ["P2D", "PT4H", "PT30M"])
    def test_iso_8601_durations_accepted(self, schema, value):
        v = _validator_for_def(schema, "WarningThreshold")
        errors = list(
            v.iter_errors(
                {
                    "beforeBreach": value,
                    "templateKey": "slaWarning",
                    "notify": ["taskOwner"],
                }
            )
        )
        assert errors == [], f"beforeBreach={value!r} rejected: {errors}"

    @pytest.mark.parametrize("value", ["soon", "4 hours", "indefinite", ""])
    def test_non_iso_rejected(self, schema, value):
        v = _validator_for_def(schema, "WarningThreshold")
        errors = list(
            v.iter_errors(
                {
                    "beforeBreach": value,
                    "templateKey": "slaWarning",
                    "notify": ["taskOwner"],
                }
            )
        )
        assert errors, f"beforeBreach={value!r} must fail — not an ISO 8601 duration"


class TestBreachPolicyAction:
    @pytest.mark.parametrize("action", ["notify", "escalate", "autoReassign", "fail"])
    def test_enum_values_accepted(self, schema, action):
        v = _validator_for_def(schema, "BreachPolicy")
        errors = list(v.iter_errors({"action": action}))
        assert errors == [], f"breachPolicy.action={action!r} rejected: {errors}"

    def test_unknown_action_rejected(self, schema):
        v = _validator_for_def(schema, "BreachPolicy")
        errors = list(v.iter_errors({"action": "ignore"}))
        assert errors, "breachPolicy.action='ignore' must fail — not in enum"


class TestEscalationStepLevel:
    def test_level_one_accepted(self, schema):
        v = _validator_for_def(schema, "EscalationStep")
        errors = list(
            v.iter_errors(
                {
                    "level": 1,
                    "assignTo": "teamLead",
                    "gracePeriod": "PT4H",
                    "onExhaustion": "escalate",
                }
            )
        )
        assert errors == [], f"level=1 rejected: {errors}"

    def test_optional_id_round_trips(self, schema):
        """EscalationStep.id is OPTIONAL and, when present, matches the kernel
        identifier grammar so BreachPolicy.escalationStepId can target a
        named step rather than an ordinal level (Review D #40c)."""
        v = _validator_for_def(schema, "EscalationStep")
        errors = list(
            v.iter_errors(
                {
                    "id": "supervisor",
                    "level": 2,
                    "assignTo": "divisionDirector",
                    "gracePeriod": "P1D",
                    "onExhaustion": "ticketCreate",
                }
            )
        )
        assert errors == [], f"EscalationStep with id='supervisor' rejected: {errors}"

    @pytest.mark.parametrize("level", [0, -1])
    def test_level_below_one_rejected(self, schema, level):
        v = _validator_for_def(schema, "EscalationStep")
        errors = list(
            v.iter_errors(
                {
                    "level": level,
                    "assignTo": "teamLead",
                    "gracePeriod": "PT4H",
                    "onExhaustion": "escalate",
                }
            )
        )
        assert errors, f"escalationChain[].level={level} must fail — minimum is 1"


class TestEnumRejections:
    """Negative coverage for every enum across the four SLA authoring shapes.

    The positive paths are already exercised by the round-trip fixture and
    by TestBreachPolicyAction above; these tests guard against the next
    authoring typo silently being accepted (Review D Finding 4)."""

    def test_invalid_calendar_type_rejected(self, schema):
        v = _validator_for_def(schema, "SlaDefinition")
        errors = list(
            v.iter_errors(
                {
                    "id": "firstResponse",
                    "expectedDuration": "PT4H",
                    "calendarType": "lunar",
                    "startAt": "assignment",
                }
            )
        )
        assert errors, (
            "SlaDefinition.calendarType='lunar' must fail — enum is "
            "{wall-clock, business}"
        )

    def test_invalid_start_at_rejected(self, schema):
        v = _validator_for_def(schema, "SlaDefinition")
        errors = list(
            v.iter_errors(
                {
                    "id": "firstResponse",
                    "expectedDuration": "PT4H",
                    "calendarType": "wall-clock",
                    "startAt": "creation",
                }
            )
        )
        assert errors, (
            "SlaDefinition.startAt='creation' must fail — enum is "
            "{assignment, activation, custom-event}"
        )

    def test_invalid_on_exhaustion_rejected(self, schema):
        v = _validator_for_def(schema, "EscalationStep")
        errors = list(
            v.iter_errors(
                {
                    "level": 1,
                    "assignTo": "teamLead",
                    "gracePeriod": "PT4H",
                    "onExhaustion": "ignore",
                }
            )
        )
        assert errors, (
            "EscalationStep.onExhaustion='ignore' must fail — enum is "
            "{escalate, fail, ticketCreate}"
        )

    def test_invalid_on_repeated_breach_rejected(self, schema):
        v = _validator_for_def(schema, "BreachPolicy")
        errors = list(
            v.iter_errors(
                {
                    "action": "notify",
                    "timeoutPolicy": {"onRepeatedBreach": "retry"},
                }
            )
        )
        assert errors, (
            "BreachPolicy.timeoutPolicy.onRepeatedBreach='retry' must fail — "
            "enum is {suspend, fail, continue}"
        )


class TestAdditionalPropertiesClosed:
    """Unknown properties anywhere under the four new sub-shapes are rejected."""

    def test_sla_definition_unknown_property_rejected(self, schema):
        v = _validator_for_def(schema, "SlaDefinition")
        errors = list(
            v.iter_errors(
                {
                    "id": "firstResponse",
                    "expectedDuration": "PT4H",
                    "calendarType": "wall-clock",
                    "startAt": "assignment",
                    "mystery": "garbage",
                }
            )
        )
        assert errors, "SlaDefinition must reject unknown properties (additionalProperties: false)"

    def test_warning_threshold_unknown_property_rejected(self, schema):
        v = _validator_for_def(schema, "WarningThreshold")
        errors = list(
            v.iter_errors(
                {
                    "beforeBreach": "P1D",
                    "templateKey": "slaWarning",
                    "notify": ["taskOwner"],
                    "mystery": "garbage",
                }
            )
        )
        assert errors, "WarningThreshold must reject unknown properties"

    def test_breach_policy_unknown_property_rejected(self, schema):
        v = _validator_for_def(schema, "BreachPolicy")
        errors = list(v.iter_errors({"action": "notify", "mystery": "garbage"}))
        assert errors, "BreachPolicy must reject unknown properties"

    def test_breach_policy_timeout_policy_unknown_rejected(self, schema):
        v = _validator_for_def(schema, "BreachPolicy")
        errors = list(
            v.iter_errors(
                {
                    "action": "notify",
                    "timeoutPolicy": {
                        "onRepeatedBreach": "suspend",
                        "mystery": "garbage",
                    },
                }
            )
        )
        assert errors, "BreachPolicy.timeoutPolicy must reject unknown properties"

    def test_escalation_step_unknown_property_rejected(self, schema):
        v = _validator_for_def(schema, "EscalationStep")
        errors = list(
            v.iter_errors(
                {
                    "level": 1,
                    "assignTo": "teamLead",
                    "gracePeriod": "PT4H",
                    "onExhaustion": "escalate",
                    "mystery": "garbage",
                }
            )
        )
        assert errors, "EscalationStep must reject unknown properties"
