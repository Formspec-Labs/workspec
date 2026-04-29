"""AssertionReference / AssertionInlineUse / AssertionUse shape regression tests.

Covers the cross-document reference protocol for assertion gates. Per ADR
0076 D-5 the assertion ``$def``s (``AssertionUse``, ``AssertionInlineUse``,
``AssertionReference``, ``AssertionDefinition``) are inlined into
``schemas/wos-workflow.schema.json``; the standalone
``wos-assertion-gate.schema.json`` is gone. The library prose lives in
``specs/governance/assertion-library.md``; the schema seam is the workflow
schema's ``$defs``.

- ``AssertionReference`` — the ``{ assertionRef: URI }`` shape for cross-library
  references, closed to additional keys.
- ``AssertionInlineUse`` — the inline assertion body carried on a pipeline stage
  item, with optional ``assertionId`` constrained to the stable identifier
  pattern.
- ``AssertionUse`` — the ``oneOf`` seam consumers ``$ref`` to express either-or
  semantics. Mixing inline body fields with ``assertionRef`` is a configuration
  error (see specs/governance/assertion-library.md §Cross-Document Reference
  Protocol).
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator, FormatChecker

from .conftest import validator_for_def

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_SCHEMA = WOS_SPEC_ROOT / "schemas" / "wos-workflow.schema.json"


@pytest.fixture(scope="module")
def schema() -> dict:
    """Workflow schema (post-ADR-0076 home of assertion ``$def``s)."""
    return json.loads(WORKFLOW_SCHEMA.read_text())


def _validator_for_def(schema: dict, def_name: str) -> Draft202012Validator:
    """Registry-aware ``$def`` lookup. ``schema`` is ignored; def lookup
    spans every classified schema via ``conftest._REGISTRY``.
    """
    return validator_for_def(def_name)


class TestAssertionUseInlineHappyPath:
    def test_valid_inline_body_round_trips(self, schema):
        validator = _validator_for_def(schema, "AssertionUse")
        doc = {
            "type": "arithmetic",
            "description": "Totals sum cleanly",
            "expression": "totalIncome = wageIncome + investmentIncome",
        }

        errors = list(validator.iter_errors(doc))

        assert errors == [], (
            "A well-formed inline assertion (type + description + expression, no "
            f"assertionRef) must validate: {errors}"
        )

    def test_inline_body_with_stable_assertion_id_round_trips(self, schema):
        validator = _validator_for_def(schema, "AssertionUse")
        doc = {
            "type": "source-grounded",
            "description": "Wage appears verbatim in the source pay stub",
            "fields": ["wageIncome"],
            "assertionId": "wageSourceGrounded",
        }

        errors = list(validator.iter_errors(doc))

        assert errors == [], (
            "Inline body with a pattern-conforming `assertionId` is the documented "
            f"stable-identity happy path: {errors}"
        )


class TestAssertionUseReferenceHappyPath:
    def test_valid_reference_round_trips(self, schema):
        validator = _validator_for_def(schema, "AssertionUse")
        doc = {
            "assertionRef": "urn:formspec:test:library:rule-42",
        }

        errors = list(validator.iter_errors(doc))

        assert errors == [], (
            "A single-key AssertionReference with a well-formed URI must validate "
            f"through the AssertionUse oneOf: {errors}"
        )

    def test_http_reference_round_trips(self, schema):
        validator = _validator_for_def(schema, "AssertionUse")
        doc = {
            "assertionRef": "https://agency.gov/assertion-libraries/income#totalArithmetic",
        }

        errors = list(validator.iter_errors(doc))

        assert errors == [], (
            f"HTTP(S) reference URIs are permitted by the oneOf branch: {errors}"
        )


class TestAssertionUseHybridRejection:
    def test_hybrid_inline_and_reference_is_rejected(self, schema):
        validator = _validator_for_def(schema, "AssertionUse")
        doc = {
            "type": "arithmetic",
            "description": "Totals sum cleanly",
            "expression": "totalIncome = wageIncome + investmentIncome",
            "assertionRef": "urn:formspec:test:library:rule-42",
        }

        errors = list(validator.iter_errors(doc))

        assert errors, (
            "Mixing inline body fields with `assertionRef` is a configuration error "
            "per specs/governance/assertion-library.md §2.3 Override Precedence — "
            "the oneOf MUST reject this, otherwise the single-source authority "
            "principle collapses."
        )


class TestAssertionReferenceShape:
    def test_assertion_reference_rejects_malformed_uri(self, schema):
        validator = _validator_for_def(schema, "AssertionReference")
        doc = {
            "assertionRef": "not a uri",
        }

        errors = list(validator.iter_errors(doc))

        assert errors, (
            "`assertionRef` is constrained by `format: uri`; a value lacking a "
            "scheme MUST be rejected at authoring time rather than deferred to "
            "runtime resolution."
        )

    def test_assertion_reference_rejects_additional_properties(self, schema):
        validator = _validator_for_def(schema, "AssertionReference")
        doc = {
            "assertionRef": "urn:formspec:test:library:rule-42",
            "rejectionPolicy": "retryWithCorrections",
        }

        errors = list(validator.iter_errors(doc))

        assert errors, (
            "AssertionReference MUST be strictly closed (additionalProperties: "
            "false). Authors who want to override rejection policy on a referenced "
            "assertion need a separate, explicitly-modeled override — not silent "
            "extra keys on the reference."
        )

    def test_assertion_reference_requires_assertion_ref_key(self, schema):
        validator = _validator_for_def(schema, "AssertionReference")
        doc: dict = {}

        errors = list(validator.iter_errors(doc))

        assert errors, (
            "AssertionReference without `assertionRef` has no resolvable target "
            "and MUST fail validation."
        )

    def test_assertion_ref_rejects_empty_string(self, schema):
        validator = _validator_for_def(schema, "AssertionReference")
        doc = {"assertionRef": ""}

        errors = list(validator.iter_errors(doc))

        assert errors, (
            "An empty `assertionRef` carries no resolution target. The schema "
            "applies `minLength: 1` so the empty string MUST be rejected at "
            "authoring time rather than slipping through as a silent no-op."
        )

    def test_assertion_ref_rejects_null(self, schema):
        validator = _validator_for_def(schema, "AssertionReference")
        doc = {"assertionRef": None}

        errors = list(validator.iter_errors(doc))

        assert errors, (
            "`assertionRef` is typed `string`; a JSON null is a type mismatch "
            "and MUST be rejected. Nullable reference keys would contradict "
            "the §2.1 either-or contract."
        )

    def test_assertion_ref_rejects_relative_fragment(self, schema):
        validator = _validator_for_def(schema, "AssertionReference")
        doc = {"assertionRef": "#localFrag"}

        errors = list(validator.iter_errors(doc))

        assert errors, (
            "`format: uri` requires an absolute URI with a scheme. A bare "
            "fragment like `#localFrag` is a URI reference, not a URI, and "
            "cannot identify a library — it MUST be rejected so references "
            "always point at a resolvable library entry."
        )


class TestAssertionInlineUseIdentifierPattern:
    def test_inline_assertion_id_pattern_mismatch_is_rejected(self, schema):
        validator = _validator_for_def(schema, "AssertionInlineUse")
        doc = {
            "type": "arithmetic",
            "description": "Totals sum cleanly",
            "expression": "totalIncome = wageIncome + investmentIncome",
            "assertionId": "1-starts-with-digit",
        }

        errors = list(validator.iter_errors(doc))

        assert errors, (
            "`assertionId` is constrained to `^[a-zA-Z][a-zA-Z0-9_-]*$` so it is "
            "safe to embed in URI selectors. An identifier starting with a digit "
            "MUST be rejected."
        )

    def test_inline_assertion_id_with_spaces_is_rejected(self, schema):
        validator = _validator_for_def(schema, "AssertionInlineUse")
        doc = {
            "type": "arithmetic",
            "description": "Totals sum cleanly",
            "expression": "totalIncome = wageIncome + investmentIncome",
            "assertionId": "has spaces",
        }

        errors = list(validator.iter_errors(doc))

        assert errors, (
            "Whitespace in `assertionId` breaks URI embedding. MUST be rejected by "
            "the pattern constraint."
        )


class TestAssertionDefinitionAssertionIdAlignment:
    """AssertionDefinition (the library entry) accepts an optional `assertionId`
    that MUST match its `id`. Enforcement of equality is a processor/lint
    concern (planned rule G-064); at the schema layer we only validate that
    both fields share the same identifier pattern so they can be compared.
    """

    def test_assertion_definition_assertion_id_pattern_is_enforced(self, schema):
        validator = _validator_for_def(schema, "AssertionDefinition")
        doc = {
            "id": "totalArithmetic",
            "type": "arithmetic",
            "description": "Totals sum cleanly",
            "assertionId": "1-bad",
        }

        errors = list(validator.iter_errors(doc))

        assert errors, (
            "`assertionId` on AssertionDefinition is constrained by the same "
            "pattern as `id`; a non-conforming value MUST be rejected."
        )

    def test_assertion_definition_without_assertion_id_still_validates(self, schema):
        validator = _validator_for_def(schema, "AssertionDefinition")
        doc = {
            "id": "totalArithmetic",
            "type": "arithmetic",
            "description": "Totals sum cleanly",
        }

        errors = list(validator.iter_errors(doc))

        assert errors == [], (
            "`assertionId` is OPTIONAL on AssertionDefinition — a library entry "
            f"without it MUST still validate: {errors}"
        )
