"""Validate all golden conformance traces against the published schema.

Every JSON file under `fixtures/conformance/expected-traces/` is a
serialized `ConformanceTrace` emitted by the wos-conformance runner.
This test suite loads the schema at
`schemas/conformance/conformance-trace.schema.json` and asserts each
golden trace validates without errors.
"""
from __future__ import annotations

import json

import pytest
from jsonschema import Draft202012Validator

from tests.schemas.conftest import SCHEMAS_ROOT, WOS_SPEC_ROOT

TRACE_SCHEMA_PATH = SCHEMAS_ROOT / "conformance" / "conformance-trace.schema.json"
GOLDEN_TRACES_DIR = WOS_SPEC_ROOT / "fixtures" / "conformance" / "expected-traces"

GOLDEN_TRACE_FILES = sorted(GOLDEN_TRACES_DIR.glob("*.json"))


@pytest.fixture(scope="module")
def trace_validator() -> Draft202012Validator:
    """Compile the conformance-trace schema once for the module."""
    schema = json.loads(TRACE_SCHEMA_PATH.read_text())
    return Draft202012Validator(schema)


@pytest.mark.parametrize(
    "trace_path",
    GOLDEN_TRACE_FILES,
    ids=[p.name for p in GOLDEN_TRACE_FILES],
)
def test_golden_trace_validates_against_schema(trace_path, trace_validator):
    """Each golden trace must pass schema validation with zero errors."""
    doc = json.loads(trace_path.read_text())
    errors = list(trace_validator.iter_errors(doc))
    assert not errors, (
        f"{trace_path.name}: {errors[0].message} "
        f"at {list(errors[0].absolute_path)}"
    )
