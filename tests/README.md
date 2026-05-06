# WOS Tests

Three pytest suites protect the WOS schema surface under `tests/schemas/`.
**Meta-validity** (`test_meta_validity.py`) asserts every file in
`schemas/` is itself a valid JSON Schema 2020-12 document. **Fixture
validity** (`test_fixture_validity.py`) runs every `$wos*`-marked fixture
under `fixtures/` through its declaring schema, complemented by
`test_negative_fixtures.py`, which asserts the intentionally-broken
documents catalogued in `fixtures/kernel/invalid-documents.json` are
rejected. **Spec-example validity** (`test_spec_examples.py`) runs every
fenced `json` block in canonical `specs/**/*.md` (generated `*.llm.md`
digests are skipped). A future schema edit must keep all four files
green; adding a new schema means adding a one-line entry to
`tests/schemas/conftest.py::MARKER_TO_SCHEMA` mapping the new `$wos*`
marker to its schema path, after which any fixture or spec block
carrying that marker joins the regression suite automatically.

## Running locally

Requires `jsonschema[format]` and `pytest` (both already dependencies of
the parent Formspec monorepo). If running in an isolated environment:

```bash
python3 -m pip install "jsonschema[format]" pytest
```

Then, from the `work-spec/` root (or the parent monorepo):

```bash
python3 -m pytest tests/schemas/ -v
# or from the parent formspec/ repo:
python3 -m pytest work-spec/tests/schemas/ -v
```
