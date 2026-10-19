"""Published MCP catalog aliases resolve from distributed schema resources offline."""
import json
from pathlib import Path
import pytest
from jsonschema import Draft202012Validator
from referencing import Registry
from referencing.exceptions import Unresolvable

ROOT = Path(__file__).resolve().parents[2]

def test_generated_catalog_matches_canonical_and_standalone_views(registry):
    catalog = json.loads((ROOT / "crates/wos-mcp/tool-catalog.json").read_text())
    standalone = json.loads((ROOT / "schemas/mcp/wos-mcp-tools.schema.json").read_text())
    Draft202012Validator(standalone, registry=registry).validate(catalog)
    malformed = {"tools": [{"name": "missing_declaration"}]}
    assert not Draft202012Validator(standalone, registry=registry).is_valid(malformed)
    with pytest.raises(Unresolvable, match="https://wos-spec.org/schemas/tooling/1.0"):
        Draft202012Validator(standalone, registry=Registry()).validate(catalog)
