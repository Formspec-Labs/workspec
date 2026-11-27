"""Schema tests for WOS review dispositions and authorized comments read contract (fs-a60c)."""
import pytest

from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR,
    api_referencing_registry,
    load_schema,
    validator_for_api_def,
)

SCHEMA = load_schema(API_SCHEMA_DIR / "review-dispositions.schema.json")
REGISTRY = api_referencing_registry()
DISPOSITION_VALIDATOR = validator_for_api_def(SCHEMA, "ReviewDisposition", REGISTRY)
QUERY_REQ_VALIDATOR = validator_for_api_def(
    SCHEMA, "ReviewDispositionQueryRequest", REGISTRY
)
LOOKUP_REQ_VALIDATOR = validator_for_api_def(
    SCHEMA, "ReviewDispositionLookupRequest", REGISTRY
)
QUERY_RESP_VALIDATOR = validator_for_api_def(
    SCHEMA, "ReviewDispositionQueryResponse", REGISTRY
)

VALID_HASH = "sha256:" + "a" * 64


@pytest.mark.parametrize(
    "state",
    ["returned", "accepted", "closed-without-acceptance", "reopened"],
)
def test_mapped_canonical_disposition_states_validate(state):
    DISPOSITION_VALIDATOR.validate(
        {
            "responseId": "resp_01j9example0000000000000001",
            "responseHash": VALID_HASH,
            "outcomeUri": f"https://example.org/outcomes/{state}",
            "mappingStatus": "mapped",
            "state": state,
            "sourceKind": "decision-record",
            "decidedAt": "2026-11-27T12:00:00Z",
            "comments": [
                {
                    "id": "c-1",
                    "body": "Decision rationale recorded.",
                    "recordedAt": "2026-11-27T12:00:00Z",
                    "authorRef": "actor:human:reviewer-1",
                }
            ],
        }
    )


def test_unmapped_imported_determination_validates_without_state():
    DISPOSITION_VALIDATOR.validate(
        {
            "responseId": "resp_01j9example0000000000000002",
            "responseHash": VALID_HASH,
            "outcomeUri": "https://example.org/outcomes/unmapped-custom",
            "mappingStatus": "unmapped",
            "sourceKind": "imported-determination",
            "decidedAt": "2026-11-27T12:00:00Z",
            "importedSource": "external-register",
            "comments": [],
        }
    )


@pytest.mark.parametrize(
    "bad_hash",
    [
        "",
        "md5:" + "a" * 32,
        "sha256:" + "A" * 64,
        "sha256:" + "a" * 63,
    ],
)
def test_lookup_request_rejects_malformed_response_hash(bad_hash):
    assert not LOOKUP_REQ_VALIDATOR.is_valid(
        {
            "responseId": "resp_01j9example0000000000000001",
            "responseHash": bad_hash,
        }
    )


def test_query_request_and_response_closed_objects():
    QUERY_REQ_VALIDATOR.validate(
        {
            "filings": [
                {
                    "responseId": "resp_01j9example0000000000000001",
                    "responseHash": VALID_HASH,
                }
            ]
        }
    )
    assert not QUERY_REQ_VALIDATOR.is_valid({"filings": []})
    assert not QUERY_REQ_VALIDATOR.is_valid(
        {
            "filings": [
                {
                    "responseId": "resp_01j9example0000000000000001",
                    "responseHash": VALID_HASH,
                }
            ],
            "unexpected": True,
        }
    )
    QUERY_RESP_VALIDATOR.validate({"items": []})
    assert not QUERY_RESP_VALIDATOR.is_valid({"items": [], "total": 0})
