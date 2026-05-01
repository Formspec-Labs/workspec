# benefits-adjudication.workflow.json — BLUF

**This example demonstrates structural shape — every embedded block present, every composition seam exercised. Inner blocks are abbreviated.**

## What this fixture proves

- Every ADR 0076 D-2 embedded block is present (`governance`, `agents`, `aiOversight`, `signature`, `custody`, `advanced`, `assurance`).
- The conditional schema rules fire (rights-impacting `impactLevel` requires `governance`; agent-typed actor requires `agents`; signature-gated transition requires `signature`).
- Cross-references resolve (`actors[*].type=='agent'` matches `agents[*].id`; signature-gated transitions covered by `signature.signers[]`).
- The 316-line size is illustrative, not production.

## What this fixture does NOT prove

A real benefits-adjudication workflow at production scale exercises ~15 pipeline stages, ~12 deontic constraints, ~3-4 signers (intake clerk, reviewer, supervisor, regional director), ~10 verifiable constraints. This fixture has 3 toy stages, 4 toy deontic items, 1 signer, 2 toy SMT predicates. **The schema accepts it because inner-block *content* is abbreviated: every embedded block is present for seam coverage, but field-level narrative and examples are thinner than a production workflow would carry. CI enforces zero `SCHEMA-DOC-001` violations on all `*.schema.json` files (`crates/wos-lint/tests/schema_doc_zero_regression.rs`); PLN-0176..0207 remains the spec-to-schema absorption pass for deeper normative alignment, not a violation ceiling.**

An LLM authoring against this fixture **WILL ship workflows that schema-validate and are wrong-by-omission** — too few pipeline stages, too few deontic constraints, single signer where multi-signer is the norm. The agency reviewer would catch it; the SBA pilot operator running on a real PoC would not, because nothing in the conformance harness asserts "your benefits workflow has fewer than 5 deontic constraints, are you sure?"

## What's coming

A `benefits-adjudication.minimum-viable.workflow.json` companion (production-scaffold size) is queued for after the ADR 0076 spec absorption pass stabilizes. Growing the fixture before absorption finishes means rewriting it once absorption surfaces what real workflows need (e.g., the cross-referencing of `notificationTemplateKey` between governance and the merged `wos-delivery` sidecar isn't yet exercised here).

## Until then

If you are forking this fixture as a starting point for an SBA pilot or production benefits workflow:

1. Use the structural shape only — every block, every seam.
2. Do NOT take the inner-block content as a complete model. The 3-stage pipeline is structural illustration, not a real adjudication pipeline.
3. Re-run `cargo run -p wos-lint --example schema_string_leaf_report -- schemas/wos-workflow.schema.json` when you change inner blocks — open string-leaf triage is the practical signal for where to tighten `enum`/`oneOf`/`pattern` next. The minimum-viable companion lands after PLN-0176..0207 spec absorption stabilizes real field shapes, not after a removed schema-doc ceiling.

## See also

- [`timeoff.workflow.json`](timeoff.workflow.json) — Forms+ tier (~30 lines, structural)
- [`nda.workflow.json`](nda.workflow.json) — DocuSign tier (~85 lines, signature-load-bearing)
- [ADR 0076 §D-9](../../thoughts/adr/0076-product-tier-consolidation.md) — examples define the tier ladder
- [`CONVENTIONS.md`](../CONVENTIONS.md) — three-section rubric (layered-sieve specs only)
