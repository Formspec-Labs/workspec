"""ProvenanceOutcome literal-agreement smoke test (Review B Finding 6).

The string ``"preconditionNotSatisfied"`` must agree between:

1. ``$defs/ProvenanceOutcome.oneOf[0].enum`` (reserved literals).
2. ``$defs/CapabilityInvocationRecord.allOf[0].then.properties.outcome.const``.

Both live in ``schemas/wos-workflow.schema.json`` (ADR 0076 canonical $defs);
``wos-provenance-log`` composes them via ``$ref`` into ``FactsTierRecord``.

If these ever drift (typo, rename, half-finished migration), the test
fires before lint or conformance ever runs.
"""

from __future__ import annotations

import json
from pathlib import Path

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW_SCHEMA = WOS_SPEC_ROOT / "schemas" / "wos-workflow.schema.json"

RESERVED_LITERAL = "preconditionNotSatisfied"


def test_precondition_not_satisfied_literal_agrees_across_workflow_schema_sites():
    schema = json.loads(WORKFLOW_SCHEMA.read_text())

    outcome_enum = schema["$defs"]["ProvenanceOutcome"]["oneOf"][0]["enum"]
    assert RESERVED_LITERAL in outcome_enum, (
        f"{RESERVED_LITERAL!r} must be a reserved literal in "
        "$defs/ProvenanceOutcome.oneOf[0].enum"
    )

    then_block = schema["$defs"]["CapabilityInvocationRecord"]["allOf"][0]["then"]
    outcome_const = then_block["properties"]["outcome"]["const"]

    assert outcome_const == RESERVED_LITERAL, (
        "The CapabilityInvocationRecord if/then branch pins "
        f"`outcome` to {outcome_const!r} but the ProvenanceOutcome enum "
        "does not list that literal as reserved. These two sites carry "
        "the same string and MUST agree -- otherwise the MUST either "
        "fails to validate documents that the enum accepts, or accepts "
        "documents the enum rejects."
    )

    assert outcome_const in outcome_enum, (
        f"The const {outcome_const!r} pinned by "
        "CapabilityInvocationRecord.allOf[0].then.properties.outcome "
        "is not present in ProvenanceOutcome.oneOf[0].enum; the open-"
        "enum branch would then REJECT the very value the guard pins."
    )
