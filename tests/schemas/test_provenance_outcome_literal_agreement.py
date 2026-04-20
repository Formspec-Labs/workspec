"""ProvenanceOutcome literal-agreement smoke test (Review B Finding 6).

After §4.3b #F5d moved `CapabilityInvocationRecord` into the kernel
provenance schema, the string ``"preconditionNotSatisfied"`` appears in
two places inside the same file:

1. `$defs/ProvenanceOutcome.oneOf[0].enum` -- the reserved-literal set.
2. `$defs/CapabilityInvocationRecord.allOf[0].then.properties.outcome.const`
   -- the guard that pins a blocked-invocation record to that literal.

Both sites MUST agree. Review B Finding 6 recommended either a `$ref`
into a narrow sub-$def OR a grep-based smoke test that the two literals
stay in sync. We chose the grep route so the enforcing schema structure
remains as close to the sibling open-enum convention as possible.

If these ever drift (typo, rename, half-finished migration), the test
fires before lint or conformance ever runs.
"""

from __future__ import annotations

import json
from pathlib import Path

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
PROVENANCE_SCHEMA = (
    WOS_SPEC_ROOT / "schemas" / "kernel" / "wos-provenance-record.schema.json"
)

RESERVED_LITERAL = "preconditionNotSatisfied"


def test_precondition_not_satisfied_literal_agrees_across_provenance_schema_sites():
    schema = json.loads(PROVENANCE_SCHEMA.read_text())

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
