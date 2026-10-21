"""Validate the current loaded-workflow catalog against its maintained API schema."""

from copy import deepcopy

import pytest

from .test_wos_api_schema_discipline import (
    API_SCHEMA_DIR, api_referencing_registry, load_schema, validator_for_api_def,
)


@pytest.fixture
def summary():
    return {"url": "urn:example:workflow:application", "title": "Application review",
            "version": "1.0.0", "status": "draft", "impactLevel": "operational"}


def validator(name="KernelSummary"):
    schema = load_schema(API_SCHEMA_DIR / "kernel-summary.schema.json")
    return validator_for_api_def(schema, name, api_referencing_registry())


def test_summary_and_list_accept_canonical_fields(summary):
    for status in ("draft", "active", "deprecated"):
        for impact in ("rights-impacting", "safety-impacting", "operational", "informational"):
            value = {**summary, "status": status, "impactLevel": impact}
            assert list(validator().iter_errors(value)) == []
    assert list(validator("KernelSummaryList").iter_errors([summary])) == []
    assert list(validator("KernelSummaryList").iter_errors([])) == []


@pytest.mark.parametrize("field", ["url", "title", "version", "status", "impactLevel"])
def test_summary_refuses_missing_null_wrong_type_fields(summary, field):
    missing = deepcopy(summary)
    del missing[field]
    assert list(validator().iter_errors(missing))
    for invalid in (None, 42):
        assert list(validator().iter_errors({**summary, field: invalid}))


@pytest.mark.parametrize("field,invalid", [
    ("url", "not a URI"), ("title", ""), ("version", ""),
    ("status", "published"), ("impactLevel", "vendor-level"),
])
def test_summary_refuses_invalid_identity_and_taxonomy(summary, field, invalid):
    assert list(validator().iter_errors({**summary, field: invalid}))


def test_summary_refuses_unknown_properties_and_invalid_list_members(summary):
    assert list(validator().iter_errors({**summary, "bundleId": "urn:test:retired"}))
    assert list(validator("KernelSummaryList").iter_errors([summary, {}]))
