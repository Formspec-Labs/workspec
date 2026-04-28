"""Extension Registry schema regression tests.

Guards the contract documented in `wos-extension-registry.schema.json`
(Registry §1, §2, §3): every entry MUST carry the four required fields
(seam, kind, lifecycle, description); `kind` and `lifecycle` are closed
enums; the document MUST refuse unknown root properties (apart from `x-`
vendor extensions).

The reference fixture at `fixtures/registry/wos-extension-registry-example.json`
is also validated end-to-end so the published example never drifts from
the schema.
"""
from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
REGISTRY_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "wos-tooling.schema.json"
)
REGISTRY_FIXTURE = (
    WOS_SPEC_ROOT / "fixtures" / "registry" / "wos-extension-registry-example.json"
)


@pytest.fixture(scope="module")
def schema() -> dict:
    return json.loads(REGISTRY_SCHEMA.read_text())


@pytest.fixture(scope="module")
def validator(schema: dict) -> Draft202012Validator:
    return Draft202012Validator(schema)


def _minimal_valid_entry() -> dict:
    return {
        "seam": "kernel.actorExtension.agent",
        "kind": "actor-extension",
        "lifecycle": "stable",
        "description": "Layer 2 registers the `agent` actor type with model-endpoint and autonomy-level requirements.",
    }


def _minimal_valid_document() -> dict:
    return {
        "$wosTooling": "1.0",
        "kind": "extensionRegistry",
        "version": "1.0.0",
        "entries": [_minimal_valid_entry()],
    }


class TestHappyPath:
    def test_minimal_document_validates(self, validator):
        errors = list(validator.iter_errors(_minimal_valid_document()))
        assert errors == [], f"minimal valid registry rejected: {errors}"

    def test_reference_fixture_validates(self, validator):
        doc = json.loads(REGISTRY_FIXTURE.read_text())
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"reference fixture rejected: {errors}"

    def test_vendor_extensions_at_root_are_allowed(self, validator):
        doc = _minimal_valid_document()
        doc["x-acme-source"] = "imported"
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"x-prefixed root key rejected: {errors}"

    def test_reference_fixture_publishes_wos_custody_identifiers(self, validator):
        doc = json.loads(REGISTRY_FIXTURE.read_text())
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"reference fixture rejected: {errors}"

        extensions = doc.get("extensions", {})
        assert extensions.get("x-wos-owning-spec-version"), (
            "reference fixture must publish x-wos-owning-spec-version"
        )

        event_types = extensions.get("x-wos-custody-event-types", [])
        published_event_types = {entry["eventType"] for entry in event_types}
        assert {
            "wos.kernel.stateTransition",
            "wos.governance.overrideRecord",
            "wos.ai.autonomyDemotion",
            "wos.assurance.attestation",
        } <= published_event_types, (
            "reference fixture must publish WOS-owned custody event types for "
            "kernel, governance, ai, and assurance"
        )

        family_prefixes = extensions.get("x-wos-typeid-family-prefixes", [])
        published_prefixes = {entry["prefix"] for entry in family_prefixes}
        assert {"case", "prov", "gov", "ai", "assurance"} <= published_prefixes, (
            "reference fixture must publish the reserved WOS TypeID family prefixes"
        )


class TestNegativeCases:
    """Explicitly required by the task: at least 3 negative cases."""

    @pytest.mark.parametrize(
        "missing_field", ["seam", "kind", "lifecycle", "description"]
    )
    def test_entry_missing_required_field_rejected(self, validator, missing_field):
        doc = _minimal_valid_document()
        del doc["entries"][0][missing_field]
        errors = list(validator.iter_errors(doc))
        assert errors, f"entry missing required `{missing_field}` must fail"

    def test_unknown_lifecycle_value_rejected(self, validator):
        doc = _minimal_valid_document()
        doc["entries"][0]["lifecycle"] = "experimental"
        errors = list(validator.iter_errors(doc))
        assert errors, "unknown lifecycle value `experimental` must fail"

    def test_unknown_kind_value_rejected(self, validator):
        doc = _minimal_valid_document()
        doc["entries"][0]["kind"] = "magic-hook"
        errors = list(validator.iter_errors(doc))
        assert errors, "unknown kind value `magic-hook` must fail"

    def test_unknown_composition_value_rejected(self, validator):
        doc = _minimal_valid_document()
        doc["entries"][0]["composition"] = "merge-or-replace"
        errors = list(validator.iter_errors(doc))
        assert errors, "unknown composition value must fail"

    def test_missing_marker_rejected(self, validator):
        doc = _minimal_valid_document()
        del doc["$wosTooling"]
        errors = list(validator.iter_errors(doc))
        assert errors, "registry without `$wosTooling` marker must fail"

    def test_wrong_marker_version_rejected(self, validator):
        doc = _minimal_valid_document()
        doc["$wosTooling"] = "2.0"
        errors = list(validator.iter_errors(doc))
        assert errors, "registry with non-1.0 `$wosTooling` marker must fail"

    def test_empty_entries_array_rejected(self, validator):
        doc = _minimal_valid_document()
        doc["entries"] = []
        errors = list(validator.iter_errors(doc))
        assert errors, "registry with zero entries must fail (minItems: 1)"

    def test_unknown_root_property_rejected(self, validator):
        doc = _minimal_valid_document()
        doc["registries"] = "oops"  # close to `entries` but not allowed
        errors = list(validator.iter_errors(doc))
        assert errors, "unknown root property must fail (additionalProperties: false)"

    def test_unknown_entry_property_rejected(self, validator):
        doc = _minimal_valid_document()
        doc["entries"][0]["nonsense"] = True
        errors = list(validator.iter_errors(doc))
        assert errors, "unknown entry property must fail (additionalProperties: false)"
