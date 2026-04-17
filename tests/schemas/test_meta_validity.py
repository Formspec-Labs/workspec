"""Every WOS schema is itself a valid JSON Schema 2020-12 document.

A malformed schema file will silently accept garbage documents, so this
is cheap first-line insurance: one parametrized case per `.json` file
under `schemas/`.
"""
from __future__ import annotations

import json

import pytest
from jsonschema import Draft202012Validator

from tests.schemas.conftest import SCHEMAS_ROOT

SCHEMA_FILES = sorted(SCHEMAS_ROOT.rglob("*.json"))


@pytest.mark.parametrize(
    "schema_path",
    SCHEMA_FILES,
    ids=[p.relative_to(SCHEMAS_ROOT).as_posix() for p in SCHEMA_FILES],
)
def test_schema_is_valid_json_schema_2020_12(schema_path):
    data = json.loads(schema_path.read_text())
    Draft202012Validator.check_schema(data)
