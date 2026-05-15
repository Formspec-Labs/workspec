"""Custody-hook append input schema regression tests.

Validates the runtime-artifact JSON shape for the WOS-owned authored-record
surface crossing the Kernel `custodyHook` seam. The root object is the
four-field append input from WOS Custody Hook Encoding §1.3; `$defs`
also pin the WOS-owned idempotency source and the minimum receipt shape.

Per ADR 0076 the standalone custody-hook JSON Schema artifact under ``schemas/kernel/``
was removed; this module inlines an equivalent Draft 2020-12 schema
so fixture-level JSON Schema checks remain aligned with ``specs/kernel/custody-hook-encoding.md``.
"""

from __future__ import annotations

import pytest
from jsonschema import Draft202012Validator, FormatChecker

# Inline mirror of Kernel custody-hook-encoding.md §1.3 (four-field append JSON).
# Canonical author-time workflow schema is ``schemas/wos-workflow.schema.json``;
# wire-format enforcement in production is ``wos_runtime::custody``.
CUSTODY_APPEND_SCHEMA: dict = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "$id": "urn:wos-spec:test:custody-append-input",
    "$defs": {
        "CaseTypeId": {
            "type": "string",
            "pattern": r"^[a-z][a-z0-9-]*_case_[0-9a-hjkmnp-tv-z]{26}$",
            "format": "wos-case-typeid",
        },
        "RecordTypeId": {
            "type": "string",
            "pattern": (
                r"^[a-z][a-z0-9-]*_"
                r"(?:prov|gov|ai|assurance|x-[a-z][a-z0-9-]+(?:-[a-z][a-z0-9-]+)+)"
                r"_[0-9a-hjkmnp-tv-z]{26}$"
            ),
            "format": "wos-record-typeid",
        },
        "IdempotencySource": {
            "type": "object",
            "additionalProperties": False,
            "required": ["caseId", "recordId"],
            "properties": {
                "caseId": {"$ref": "#/$defs/CaseTypeId"},
                "recordId": {"$ref": "#/$defs/RecordTypeId"},
            },
        },
        "CustodyAppendReceipt": {
            "type": "object",
            "additionalProperties": False,
            "required": ["canonical_event_hash"],
            "properties": {
                "canonical_event_hash": {
                    "type": "string",
                    "pattern": r"^(?:sha256:)?[0-9a-f]{64}$",
                },
            },
        },
    },
    "type": "object",
    "additionalProperties": False,
    "required": ["caseId", "recordId", "eventType", "record"],
    "properties": {
        "caseId": {"$ref": "#/$defs/CaseTypeId"},
        "recordId": {"$ref": "#/$defs/RecordTypeId"},
        "eventType": {
            "type": "string",
            "pattern": r"^wos\.[A-Za-z0-9._-]+$",
        },
        "record": {
            "type": "string",
            "contentEncoding": "base64",
            "pattern": r"^[A-Za-z0-9+/]+=*$",
        },
    },
}


@pytest.fixture(scope="module")
def schema() -> dict:
    return CUSTODY_APPEND_SCHEMA


@pytest.fixture(scope="module")
def format_checker() -> FormatChecker:
    checker = FormatChecker()
    alphabet = "0123456789abcdefghjkmnpqrstvwxyz"

    def _decode_typeid_tail(tail: str) -> bytes | None:
        if len(tail) != 26:
            return None
        value = 0
        for ch in tail:
            try:
                value = (value << 5) | alphabet.index(ch)
            except ValueError:
                return None
        if value >= (1 << 128):
            return None
        return value.to_bytes(16, "big")

    def _is_uuidv7_tail(tail: str) -> bool:
        decoded = _decode_typeid_tail(tail)
        if decoded is None:
            return False
        version = decoded[6] >> 4
        variant = decoded[8] >> 6
        return version == 7 and variant == 0b10

    @checker.checks("wos-case-typeid")
    def _check_case_typeid(value: object) -> bool:
        if not isinstance(value, str):
            return False
        parts = value.split("_")
        return len(parts) == 3 and parts[1] == "case" and _is_uuidv7_tail(parts[2])

    @checker.checks("wos-record-typeid")
    def _check_record_typeid(value: object) -> bool:
        if not isinstance(value, str):
            return False
        parts = value.split("_")
        return len(parts) == 3 and parts[1] != "case" and _is_uuidv7_tail(parts[2])

    return checker


def _validator_for_def(
    schema: dict, def_name: str, format_checker: FormatChecker
) -> Draft202012Validator:
    target = schema["$defs"][def_name]
    composed = {
        "$schema": schema.get("$schema", "https://json-schema.org/draft/2020-12/schema"),
        "$id": f"{schema.get('$id', 'urn:test')}#${def_name}",
        "$defs": schema["$defs"],
        **target,
    }
    return Draft202012Validator(composed, format_checker=format_checker)


def _valid_append_input() -> dict:
    return {
        "caseId": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd",
        "recordId": "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd",
        "eventType": "wos.kernel.state_transition",
        "record": "oA==",
    }


class TestAppendInputShape:
    def test_valid_append_input_accepted(self, schema, format_checker):
        validator = Draft202012Validator(schema, format_checker=format_checker)
        errors = list(validator.iter_errors(_valid_append_input()))
        assert errors == [], f"valid custody append input rejected: {errors}"

    def test_invalid_case_id_prefix_rejected(self, schema, format_checker):
        validator = Draft202012Validator(schema, format_checker=format_checker)
        doc = _valid_append_input()
        doc["caseId"] = "sba-poc_prov_01jqrpd32jf8xtx9qxkkv3rqsd"
        errors = list(validator.iter_errors(doc))
        assert errors, "caseId using non-`case` family prefix must fail"

    def test_invalid_record_id_prefix_rejected(self, schema, format_checker):
        validator = Draft202012Validator(schema, format_checker=format_checker)
        doc = _valid_append_input()
        doc["recordId"] = "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd"
        errors = list(validator.iter_errors(doc))
        assert errors, "recordId using `case` family prefix must fail"

    def test_invalid_event_type_rejected(self, schema, format_checker):
        validator = Draft202012Validator(schema, format_checker=format_checker)
        doc = _valid_append_input()
        doc["eventType"] = "trellis.appended"
        errors = list(validator.iter_errors(doc))
        assert errors, "non-`wos.*` event type must fail"

    def test_non_base64_record_rejected(self, schema, format_checker):
        validator = Draft202012Validator(schema, format_checker=format_checker)
        doc = _valid_append_input()
        doc["record"] = "{not-base64}"
        errors = list(validator.iter_errors(doc))
        assert errors, "record must be base64-encoded dCBOR bytes when serialized as JSON"

    def test_non_uuidv7_case_id_rejected(self, schema, format_checker):
        validator = Draft202012Validator(schema, format_checker=format_checker)
        doc = _valid_append_input()
        doc["caseId"] = "sba-poc_case_00000000000000000000000000"
        errors = list(validator.iter_errors(doc))
        assert errors, "caseId tail must decode to UUIDv7, not merely match the regex shape"

    def test_non_uuidv7_record_id_rejected(self, schema, format_checker):
        validator = Draft202012Validator(schema, format_checker=format_checker)
        doc = _valid_append_input()
        doc["recordId"] = "sba-poc_prov_00000000000000000000000000"
        errors = list(validator.iter_errors(doc))
        assert errors, "recordId tail must decode to UUIDv7, not merely match the regex shape"


class TestIdempotencySource:
    def test_idempotency_source_accepts_case_and_record_only(self, schema, format_checker):
        validator = _validator_for_def(schema, "IdempotencySource", format_checker)
        doc = {
            "caseId": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd",
            "recordId": "sba-poc_gov_01hw7rm71vfay8vvw14d2pf2db",
        }
        errors = list(validator.iter_errors(doc))
        assert errors == [], f"valid idempotency source rejected: {errors}"

    def test_idempotency_source_rejects_event_type(self, schema, format_checker):
        validator = _validator_for_def(schema, "IdempotencySource", format_checker)
        doc = {
            "caseId": "sba-poc_case_01jqrpd32jf8xtx9qxkkv3rqsd",
            "recordId": "sba-poc_gov_01hw7rm71vfay8vvw14d2pf2db",
            "eventType": "wos.governance.override_record",
        }
        errors = list(validator.iter_errors(doc))
        assert errors, "eventType must not appear in the WOS-owned idempotency source"


class TestReceiptShape:
    def test_receipt_accepts_canonical_event_hash(self, schema, format_checker):
        validator = _validator_for_def(schema, "CustodyAppendReceipt", format_checker)
        receipt = {
            "canonical_event_hash": "sha256:9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c"
        }
        errors = list(validator.iter_errors(receipt))
        assert errors == [], f"valid custody append receipt rejected: {errors}"

    def test_receipt_accepts_legacy_bare_canonical_event_hash(
        self, schema, format_checker
    ):
        validator = _validator_for_def(schema, "CustodyAppendReceipt", format_checker)
        receipt = {
            "canonical_event_hash": "9ad0556334071a0d40050c61ba4601506b87dbc4847d808fb3693b364af5090c"
        }
        errors = list(validator.iter_errors(receipt))
        assert errors == [], f"legacy custody append receipt rejected: {errors}"

    def test_receipt_rejects_malformed_hash(self, schema, format_checker):
        validator = _validator_for_def(schema, "CustodyAppendReceipt", format_checker)
        receipt = {"canonical_event_hash": "not-a-digest"}
        errors = list(validator.iter_errors(receipt))
        assert errors, "receipt must reject malformed canonical_event_hash values"
