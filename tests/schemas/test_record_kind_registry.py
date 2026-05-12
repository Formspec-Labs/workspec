"""D26 record-kind registry seed regression tests."""
from __future__ import annotations

import json
import re
from pathlib import Path


WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
REGISTRY_PATH = WOS_SPEC_ROOT / "schemas" / "record-kind-registry.json"
WORKFLOW_PATH = WOS_SPEC_ROOT / "schemas" / "wos-workflow.schema.json"
API_PROVENANCE_PATH = WOS_SPEC_ROOT / "schemas" / "api" / "provenance.schema.json"
PROVENANCE_LOG_PATH = WOS_SPEC_ROOT / "schemas" / "wos-provenance-log.schema.json"
EVENT_LITERAL_RE = re.compile(
    r"^wos\.(kernel|governance|ai|assurance)\.[a-z][a-z0-9]*(?:_[a-z0-9]+)*$"
)


def _registry() -> dict:
    return json.loads(REGISTRY_PATH.read_text())


def _by_literal(registry: dict) -> dict[str, dict]:
    return {entry["literal"]: entry for entry in registry["recordKinds"]}


def event_literal_mappings() -> dict[str, str]:
    return {
        entry["literal"]: entry["eventLiteral"]
        for entry in _registry()["recordKinds"]
        if "eventLiteral" in entry
    }


def _camel_to_snake(value: str) -> str:
    return re.sub(r"(?<!^)([A-Z])", r"_\1", value).lower()


def _const_pair_from_guard(guard: dict) -> tuple[str | None, str | None]:
    record_kind = (
        guard.get("if", {})
        .get("properties", {})
        .get("recordKind", {})
        .get("const")
    )
    event = (
        guard.get("then", {})
        .get("properties", {})
        .get("event", {})
        .get("const")
    )
    return record_kind, event


def _workflow_event_guard_mappings() -> dict[str, str]:
    schema = json.loads(WORKFLOW_PATH.read_text())
    mappings = {}
    for guard in schema["$defs"]["FactsTierRecord"].get("allOf", []):
        record_kind, event = _const_pair_from_guard(guard)
        if record_kind and event:
            mappings[record_kind] = event
    return mappings


def _api_event_guard_mappings() -> dict[str, str]:
    schema = json.loads(API_PROVENANCE_PATH.read_text())
    mappings = {}
    for guard in schema["$defs"]["FactsTierRecord"].get("oneOf", []):
        record_kind, event = _const_pair_from_guard({"if": guard, "then": guard})
        if record_kind and event:
            assert set(guard.get("required", [])) >= {"recordKind", "event"}
            mappings[record_kind] = event
    return mappings


def _provenance_log_event_guard_mappings() -> dict[str, str]:
    schema = json.loads(PROVENANCE_LOG_PATH.read_text())
    mappings = {}
    for definition in schema["$defs"].values():
        for guard in definition.get("allOf", []):
            record_kind, event = _const_pair_from_guard(guard)
            if record_kind and event and "event" in guard.get("then", {}).get("required", []):
                mappings[record_kind] = event
    return mappings


def test_record_kind_registry_counts_match_entries():
    registry = _registry()
    record_kinds = registry["recordKinds"]

    assert registry["totalCount"] == len(record_kinds)
    assert registry["schemaValidatedCount"] == sum(
        1 for entry in record_kinds if entry.get("schemaValidated") is True
    )


def test_record_kind_registry_entries_have_required_identity_fields():
    for entry in _registry()["recordKinds"]:
        assert entry.get("literal"), f"{entry} is missing literal"
        assert entry.get("rustVariant"), f"{entry} is missing rustVariant"
        assert entry.get("category"), f"{entry} is missing category"


def test_record_kind_registry_event_literals_are_f13_shaped():
    for entry in _registry()["recordKinds"]:
        event_literal = entry.get("eventLiteral")
        if event_literal is None:
            continue

        assert EVENT_LITERAL_RE.match(event_literal), (
            f"{entry['literal']} has non-F-13 eventLiteral {event_literal!r}"
        )
        assert event_literal.rsplit(".", maxsplit=1)[1] == _camel_to_snake(
            entry["literal"]
        )


def test_record_kind_registry_current_d26_event_mappings():
    entries = _by_literal(_registry())

    expected = {
        "caseCreated": "wos.kernel.case_created",
        "intakeAccepted": "wos.kernel.intake_accepted",
        "intakeRejected": "wos.kernel.intake_rejected",
        "intakeDeferred": "wos.kernel.intake_deferred",
        "forEachIterationStarted": "wos.kernel.for_each_iteration_started",
        "forEachIterationCompleted": "wos.kernel.for_each_iteration_completed",
        "forEachCompleted": "wos.kernel.for_each_completed",
        "signatureAffirmation": "wos.kernel.signature_affirmation",
        "signatureAdmissionFailed": "wos.kernel.signature_admission_failed",
        "determinationRescinded": "wos.governance.determination_rescinded",
        "reinstated": "wos.governance.reinstated",
        "clockStarted": "wos.governance.clock_started",
        "clockResolved": "wos.governance.clock_resolved",
        "identityAttestation": "wos.assurance.identity_attestation",
    }

    assert event_literal_mappings() == expected
    for literal, event_literal in expected.items():
        assert entries[literal]["eventLiteral"] == event_literal


def test_registry_event_literal_mappings_drive_workflow_api_and_log_guards():
    mappings = event_literal_mappings()

    assert _workflow_event_guard_mappings() == mappings
    assert _api_event_guard_mappings() == mappings
    assert _provenance_log_event_guard_mappings() == mappings
