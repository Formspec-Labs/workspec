"""The same authored governance values feed JSON Schema, Rust model and lint tests."""
import json

import pytest

from tests.schemas.conftest import WOS_SPEC_ROOT, validator_for_def

MATRIX = json.loads((WOS_SPEC_ROOT / "fixtures/validation/governance-policy-matrix.json").read_text())
CASES = MATRIX["cases"]


@pytest.mark.parametrize("case", CASES, ids=lambda case: case["id"])
def test_canonical_governance_structure(case):
    errors = list(validator_for_def("Governance").iter_errors(case["governance"]))
    assert (not errors) == case["structurallyValid"], case["id"]


@pytest.mark.parametrize("name,delivery", [(name, delivery) for name, delivery in MATRIX["deliveries"].items() if delivery["document"] is not None])
def test_canonical_delivery_structure(name, delivery, validators):
    errors = list(validators["$wosDelivery"].iter_errors(delivery["document"]))
    assert (not errors) == delivery["structurallyValid"], name
