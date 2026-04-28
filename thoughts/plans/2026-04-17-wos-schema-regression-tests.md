# WOS Schema Regression Tests — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Permanent regression protection for every WOS schema. Port the proven patterns from parent Formspec's `tests/conformance/spec/test_spec_examples.py` and extend with explicit meta-validation. Every future schema edit either adds/updates a fixture or fails CI.

**Architecture:** Three complementary layers, all pytest — matching Formspec's convention rather than inventing a Rust test harness for JSON semantics.

1. **Meta-validity** — Every schema file is itself a valid JSON Schema 2020-12 document (`Draft202012Validator.check_schema`). Parent Formspec does NOT do this; WOS adds it as ~10 lines of cheap insurance.
2. **Fixture validity** — Every JSON file under `fixtures/**/` whose `$wos*` marker classifies it must validate against the matching schema.
3. **Spec example validity** — Every `json` code block in `specs/**/*.md` with a `$wos*` marker must validate against the matching schema (direct port of `test_spec_examples.py`).

**Tech Stack:** Python 3.11+, `pytest`, `jsonschema[format]` (already in parent's `pyproject.toml`).

**Spec anchor:** [Code review findings](../archive/reviews/2026-04-16-architecture-review-open-questions.md), Finding #7 — schema change has no committed regression guard.

**Prior art:**

- `/Users/mikewolfd/Work/formspec/tests/conformance/spec/test_spec_examples.py:19-34` — pattern to port.
- `/Users/mikewolfd/Work/formspec/tests/conformance/schemas/test_definition_schema.py` — per-schema good/bad fixture pattern.

---

## Prerequisites

- Python 3.11+ on the developer machine and in CI. Parent repo already requires it.
- `jsonschema[format]` available (already used by parent conformance suite).
- WOS fixtures use consistent `$wos*` markers (post–ADR 0076: author-time core is `$wosWorkflow`; historical markers retired).

## Completion criteria

1. `wos-spec/tests/schemas/` directory exists with four test files (meta-validity, fixtures, spec examples, plus a shared discovery helper).
2. Every schema under `wos-spec/schemas/` is meta-validated.
3. Every fixture under `wos-spec/fixtures/` is validated against its declaring schema.
4. Every fenced `json` block in `wos-spec/specs/**/*.md` containing a `$wos*` marker is validated.
5. CI gate: a GitHub Actions job runs `pytest wos-spec/tests/schemas/` on every PR touching `wos-spec/schemas/`, `wos-spec/fixtures/`, or `wos-spec/specs/`.
6. Running locally via `python3 -m pytest wos-spec/tests/schemas/ -v` passes on current tree.

## File structure

- **Create:** `wos-spec/tests/__init__.py`
- **Create:** `wos-spec/tests/schemas/__init__.py`
- **Create:** `wos-spec/tests/schemas/conftest.py` — shared helpers (schema registry, marker → schema mapping).
- **Create:** `wos-spec/tests/schemas/test_meta_validity.py` — every schema is valid JSON Schema 2020-12.
- **Create:** `wos-spec/tests/schemas/test_fixture_validity.py` — every fixture validates against its schema.
- **Create:** `wos-spec/tests/schemas/test_spec_examples.py` — every json code block in specs validates.
- **Modify:** `.github/workflows/wos-*.yml` — add pytest step.
- **Create:** `wos-spec/tests/README.md` — one paragraph explaining the three layers.

---

## Task 1: Shared discovery helpers

**Files:**

- Create: `wos-spec/tests/schemas/conftest.py`

- [ ] **Step 1.1:** Define a marker → schema mapping from the `$wos*` discriminators to schema paths:

```python
# wos-spec/tests/schemas/conftest.py
from pathlib import Path
import json
import pytest
from jsonschema import Draft202012Validator

WOS_SPEC_ROOT = Path(__file__).resolve().parents[2]
SCHEMAS_ROOT = WOS_SPEC_ROOT / "schemas"

MARKER_TO_SCHEMA = {
    "$wosWorkflow": "wos-workflow.schema.json",
    "$wosDelivery": "sidecars/wos-delivery.schema.json",
    "$wosOntologyAlignment": "sidecars/wos-ontology-alignment.schema.json",
    "$wosCaseInstance": "wos-case-instance.schema.json",
    "$wosProvenanceLog": "wos-provenance-log.schema.json",
    "$wosTooling": "wos-tooling.schema.json",
}

@pytest.fixture(scope="session")
def validators():
    """Load and compile every schema once per session."""
    result = {}
    for marker, rel in MARKER_TO_SCHEMA.items():
        path = SCHEMAS_ROOT / rel
        schema = json.loads(path.read_text())
        result[marker] = Draft202012Validator(schema)
    return result

def classify(doc: dict) -> str | None:
    """Return the $wos* marker key in a document, or None if unmarked."""
    for k in doc:
        if k.startswith("$wos"):
            return k
    return None
```

- [ ] **Step 1.2:** Commit. `test(wos): shared schema-discovery helpers for regression tests`.

## Task 2: Meta-validity test

**Files:**

- Create: `wos-spec/tests/schemas/test_meta_validity.py`

- [ ] **Step 2.1:**

```python
"""Every WOS schema is itself a valid JSON Schema 2020-12 document."""
from pathlib import Path
import json
import pytest
from jsonschema import Draft202012Validator

from tests.schemas.conftest import SCHEMAS_ROOT

@pytest.mark.parametrize("schema_path", sorted(SCHEMAS_ROOT.rglob("*.json")), ids=lambda p: p.relative_to(SCHEMAS_ROOT).as_posix())
def test_schema_is_valid_json_schema_2020_12(schema_path):
    data = json.loads(schema_path.read_text())
    Draft202012Validator.check_schema(data)
```

- [ ] **Step 2.2:** Run `python3 -m pytest wos-spec/tests/schemas/test_meta_validity.py -v`. Expect 19 parametrized cases, all passing.

- [ ] **Step 2.3:** Commit. `test(wos): meta-validate every schema as JSON Schema 2020-12`.

## Task 3: Fixture-validity test

**Files:**

- Create: `wos-spec/tests/schemas/test_fixture_validity.py`

- [ ] **Step 3.1:**

```python
"""Every WOS fixture validates against its classified schema."""
from pathlib import Path
import json
import pytest

from tests.schemas.conftest import WOS_SPEC_ROOT, classify

FIXTURES_ROOT = WOS_SPEC_ROOT / "fixtures"
ALL_FIXTURES = sorted(p for p in FIXTURES_ROOT.rglob("*.json") if p.is_file())

@pytest.mark.parametrize("fixture_path", ALL_FIXTURES, ids=lambda p: p.relative_to(FIXTURES_ROOT).as_posix())
def test_fixture_validates(fixture_path, validators):
    doc = json.loads(fixture_path.read_text())
    marker = classify(doc) if isinstance(doc, dict) else None
    if marker is None:
        pytest.skip(f"no $wos* marker in {fixture_path.name}")
    if marker not in validators:
        pytest.fail(f"unknown marker {marker} in {fixture_path.relative_to(FIXTURES_ROOT)}")
    errors = list(validators[marker].iter_errors(doc))
    assert not errors, f"{fixture_path.name}: {errors[0].message}"
```

- [ ] **Step 3.2:** Run — expect ~41 fixtures discovered. Some may be negative fixtures (`invalid-documents.json` and similar); gracefully skip those or split into a separate negative-assertion test. Document the triage.

- [ ] **Step 3.3:** For any fixture that is INTENDED to fail validation (negative fixtures under `fixtures/validation/` and `fixtures/*/invalid-*.json`), move or annotate them so the positive test doesn't choke. Acceptable: naming convention `invalid-*.json` → excluded from this test and exercised by a separate `test_negative_fixtures.py`.

- [ ] **Step 3.4:** Commit. `test(wos): validate every fixture against its classified schema`.

## Task 4: Spec-example test

**Files:**

- Create: `wos-spec/tests/schemas/test_spec_examples.py`

- [ ] **Step 4.1:** Port parent Formspec's extractor pattern:

```python
"""Every fenced json code block in wos-spec/specs/ validates against its schema."""
import json
import re
from pathlib import Path
import pytest

from tests.schemas.conftest import WOS_SPEC_ROOT, classify

SPECS_ROOT = WOS_SPEC_ROOT / "specs"
SPEC_FILES = sorted(p for p in SPECS_ROOT.rglob("*.md") if not p.name.endswith(".llm.md"))

JSON_BLOCK = re.compile(r"```json\n(.*?)\n```", re.DOTALL)

def extract_examples():
    cases = []
    for spec in SPEC_FILES:
        text = spec.read_text()
        for i, match in enumerate(JSON_BLOCK.finditer(text)):
            block = match.group(1)
            try:
                doc = json.loads(block)
            except json.JSONDecodeError:
                continue  # Non-JSON block (pseudocode, partial); skip
            if not isinstance(doc, dict):
                continue
            marker = classify(doc)
            if marker:
                cases.append((spec.relative_to(SPECS_ROOT).as_posix(), i, marker, doc))
    return cases

@pytest.mark.parametrize(
    "spec_name,block_index,marker,doc",
    extract_examples(),
    ids=lambda v: str(v) if not isinstance(v, dict) else "doc",
)
def test_spec_example_validates(spec_name, block_index, marker, doc, validators):
    errors = list(validators[marker].iter_errors(doc))
    assert not errors, (
        f"{spec_name} block #{block_index} ({marker}): {errors[0].message}"
    )
```

- [ ] **Step 4.2:** Run — expect several examples discovered (kernel spec has at least one; component-integration and sidecar specs have more). Fix any failing block: either (a) the block is wrong (correct it) or (b) the schema is wrong (spec-before-schema discipline: the block wins, update the schema in a follow-up commit).

- [ ] **Step 4.3:** Commit. `test(wos): validate every spec code block against its classified schema`.

## Task 5: CI gate

**Files:**

- Modify or create: `.github/workflows/wos-tests.yml` (or add steps to existing workflow that runs on wos-spec changes).

- [ ] **Step 5.1:** Add a job:

```yaml
wos-schema-regression:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
      with:
        submodules: recursive
    - uses: actions/setup-python@v5
      with:
        python-version: '3.11'
    - run: python3 -m pip install jsonschema[format] pytest
    - run: python3 -m pytest wos-spec/tests/schemas/ -v
```

- [ ] **Step 5.2:** Condition the job on path filters: `wos-spec/schemas/**`, `wos-spec/fixtures/**`, `wos-spec/specs/**/*.md`.

- [ ] **Step 5.3:** Commit. `build: CI gate for WOS schema regression tests`.

## Task 6: README

**Files:**

- Create: `wos-spec/tests/README.md`

- [ ] **Step 6.1:** One paragraph: "Three pytest suites protect the WOS schema surface. Meta-validity asserts every schema is a valid JSON Schema 2020-12 document. Fixture validity runs every fixture through its classified schema. Spec-example validity runs every fenced `json` block in the canonical spec prose. A future schema edit must keep all three green; adding a new schema means adding its entry to `tests/schemas/conftest.py::MARKER_TO_SCHEMA`."

- [ ] **Step 6.2:** Commit. `docs(wos): tests/README explains the three-layer schema regression suite`.

---

## Self-review checklist

- Every schema meta-validated (Task 2).
- Every positive fixture validated (Task 3).
- Every spec code block validated (Task 4).
- CI enforces the gate (Task 5).
- New schemas require a one-line addition to `conftest.py::MARKER_TO_SCHEMA`, and new fixtures / code blocks self-discover.

## Why this matters

This review surfaced a concrete gap: the 2026-04-16 schema patch was verified empirically in a one-off script, not by a committed regression test. Every future schema edit has the same risk until this plan lands. Cost: ~1 engineer-day. Benefit: permanent. Copies Formspec's proven pattern; contributor muscle-memory transfers.

**Estimated effort:** ~1 engineer-day.
