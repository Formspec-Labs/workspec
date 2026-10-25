"""D26 record-kind registry seed regression tests."""
from __future__ import annotations

import json
import re
import runpy
from pathlib import Path


WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
REGISTRY_PATH = WOS_SPEC_ROOT / "schemas" / "record-kind-registry.json"
ADR_0093_PATH = (
    WOS_SPEC_ROOT / "thoughts" / "adr" / "0093-case-is-its-trellis-ledger.md"
)
WORKFLOW_PATH = WOS_SPEC_ROOT / "schemas" / "wos-workflow.schema.json"
API_PROVENANCE_PATH = WOS_SPEC_ROOT / "schemas" / "api" / "provenance.schema.json"
PROVENANCE_LOG_PATH = WOS_SPEC_ROOT / "schemas" / "wos-provenance-log.schema.json"
PROVENANCE_KIND_RS = (
    WOS_SPEC_ROOT / "crates" / "wos-events" / "src" / "provenance" / "kind.rs"
)
PARITY_SCRIPT = WOS_SPEC_ROOT / "scripts" / "check-recordkind-parity.py"
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


def _event_literal_to_record_kind() -> dict[str, str]:
    return {event: literal for literal, event in event_literal_mappings().items()}


def _camel_to_snake(value: str) -> str:
    return re.sub(r"(?<!^)([A-Z])", r"_\1", value).lower()


def _canonical_event_literals_from_rust() -> dict[str, str]:
    """`ProvenanceKind::canonical_event_literal()` rows, read by the CI parity
    script's parser so this test and the CI gate cannot drift apart."""
    parity = runpy.run_path(str(PARITY_SCRIPT))
    return parity["extract_canonical_event_literals"](PROVENANCE_KIND_RS)


def _record_kind_const_from_guard(guard: dict) -> str | None:
    return (
        guard.get("if", {})
        .get("properties", {})
        .get("recordKind", {})
        .get("const")
    )


def _event_const_from_guard(guard: dict) -> str | None:
    return (
        guard.get("if", {})
        .get("properties", {})
        .get("event", {})
        .get("const")
        or guard.get("then", {})
        .get("properties", {})
        .get("event", {})
        .get("const")
    )


def _const_pair_from_guard(guard: dict) -> tuple[str | None, str | None]:
    record_kind = _record_kind_const_from_guard(guard)
    event = (
        guard.get("then", {})
        .get("properties", {})
        .get("event", {})
        .get("const")
    )
    return record_kind, event


def _workflow_event_guard_mappings() -> dict[str, str]:
    schema = json.loads(WORKFLOW_PATH.read_text())
    event_to_literal = _event_literal_to_record_kind()
    mappings = {}
    for guard in schema["$defs"]["FactsTierRecord"].get("allOf", []):
        event = _event_const_from_guard(guard)
        record_kind = event_to_literal.get(event)
        if record_kind:
            mappings[record_kind] = event
    return mappings


def _api_event_guard_mappings() -> dict[str, str]:
    schema = json.loads(API_PROVENANCE_PATH.read_text())
    event_to_literal = _event_literal_to_record_kind()
    mappings = {}
    for guard in schema["$defs"]["FactsTierRecord"].get("allOf", []):
        event = _event_const_from_guard(guard)
        record_kind = event_to_literal.get(event)
        if record_kind:
            assert "event" in guard.get("if", {}).get("required", [])
            mappings[record_kind] = event
    return mappings


def _provenance_log_event_guard_mappings() -> dict[str, str]:
    schema = json.loads(PROVENANCE_LOG_PATH.read_text())
    event_to_literal = _event_literal_to_record_kind()
    mappings = {}
    for definition in schema["$defs"].values():
        for guard in definition.get("allOf", []):
            event = _event_const_from_guard(guard)
            record_kind = event_to_literal.get(event)
            if record_kind:
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
        # These bounded causal facts avoid the determination_* wire prefix,
        # which names actual determinations in Trellis verification.
        native_causal_names = {
            "determinationReasoning": "reasoning_recorded",
            "determinationCounterfactual": "counterfactual_recorded",
        }
        assert event_literal.rsplit(".", maxsplit=1)[1] == native_causal_names.get(
            entry["literal"], _camel_to_snake(entry["literal"])
        )


def test_record_kind_registry_event_literals_mirror_canonical_event_literals():
    entries = _by_literal(_registry())
    expected = _canonical_event_literals_from_rust()

    assert expected, "ProvenanceKind::canonical_event_literal() parser found no rows"
    assert event_literal_mappings() == expected
    for literal, event_literal in expected.items():
        assert entries[literal]["eventLiteral"] == event_literal


def test_record_kind_registry_admission_contract_sources_runtime_catalog():
    registry = _registry()
    contract = registry["admissionContract"]

    assert contract["source"] == "ProvenanceKind::canonical_event_literal()"
    assert contract["runtimeSurface"] == "GET /v1/scopes/{scope}/registries/event-types"
    assert contract["dispatchField"] == "event"
    assert contract["legacyRecordKindField"] == "recordKind"
    assert "recordKinds[].eventLiteral" in contract["instanceGeneration"]


def test_adr_0093_registry_count_prose_matches_json():
    """TWREF-061: ADR 0093 body must not drift from record-kind-registry.json totals."""
    registry = _registry()
    total = registry["totalCount"]
    overlays = registry["schemaValidatedCount"]
    flat = total - overlays
    text = ADR_0093_PATH.read_text()

    long_form = re.search(
        rf"\({total} kinds; (\d+) with schema-validated overlays; (\d+) flat\)",
        text,
    )
    assert long_form is not None, (
        "ADR 0093 missing parenthetical registry counts "
        f"({total} kinds; {overlays} with schema-validated overlays; {flat} flat)"
    )
    assert int(long_form.group(1)) == overlays
    assert int(long_form.group(2)) == flat

    short_form = re.search(
        rf"\({total} kinds; (\d+) schema-validated; (\d+) flat\)",
        text,
    )
    assert short_form is not None, (
        "ADR 0093 missing short-form registry counts "
        f"({total} kinds; {overlays} schema-validated; {flat} flat)"
    )
    assert int(short_form.group(1)) == overlays
    assert int(short_form.group(2)) == flat


def test_registry_event_literal_mappings_drive_workflow_api_and_log_guards():
    mappings = event_literal_mappings()

    assert _workflow_event_guard_mappings() == mappings
    # Public API schemas stay typify-clean: the shared facts envelope rejects
    # legacy recordKind, while workflow/provenance-log schemas carry the
    # event-conditioned payload guards.
    assert _api_event_guard_mappings() == {}
    assert _provenance_log_event_guard_mappings().items() <= mappings.items()


def test_parity_walk_skips_instance_keywords_only_in_keyword_position():
    """A property, pattern or definition may be named `default` or `examples`
    (wos-tooling has an `examples` property); its subschema is still walked,
    while the value of the `default`/`examples`/`const`/`enum` keyword is data."""
    pins = runpy.run_path(str(PARITY_SCRIPT))["iter_record_kind_pins"]
    pin = {"properties": {"recordKind": {"const": "bogusKind"}}}
    named = {
        "properties": {"default": pin, "examples": pin},
        "patternProperties": {"^const$": pin},
        "$defs": {"enum": pin},
    }
    assert sorted(pins(named)) == [
        ("/$defs/enum", "bogusKind"),
        ("/patternProperties/^const$", "bogusKind"),
        ("/properties/default", "bogusKind"),
        ("/properties/examples", "bogusKind"),
    ]
    keyword_values = {"default": pin, "examples": [pin], "const": pin, "enum": [pin]}
    assert list(pins(keyword_values)) == []
