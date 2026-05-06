# WOS Spec Conventions

This repo treats specs as executable contracts: prose defines normative processor behavior, schemas encode authoring surfaces, and conformance fixtures/lint rules prevent drift.

## Required Sections (for layered-sieve specs)

The three-section rubric below applies to **layered-sieve specs only** — the docs that encode behavioral semantics schemas alone cannot express:

- `kernel/spec.md` (L0 — orchestration substrate)
- `governance/workflow-governance.md` (L1 — due process, review, validation pipelines)
- `ai/ai-integration.md` (L2 — agent integration, deontic enforcement, oversight)
- `advanced/advanced-governance.md` (L3 — DCR, equity, SMT verifiable constraints)

These specs MUST include:

1. **Normative Contract**
2. **Composition**
3. **Conformance**

These are not "nice to have" documentation. They are how we keep the spec, schemas, runtime behavior, and fixtures aligned for behavioral semantics that schemas cannot encode (evaluation order, MUST-fire-before, sieve precedence, hook firing).

### Embedded-block specs follow the rubric when behavioral

Embedded blocks of `$wosWorkflow` (`signature`, `custody`, etc.) live partly in `schemas/wos-workflow.schema.json` and partly in companion specs that encode behavioral semantics schemas cannot capture (e.g., signing flow patterns, witness/notary semantics, custody lattice precedence). When such a companion spec exists (e.g., `specs/profiles/signature.md`, `specs/kernel/custody-hook-encoding.md`), it MUST follow the three-section rubric — the embedded-block boundary does not exempt it from layered-sieve discipline. Embedded blocks whose entire surface is captured by schema descriptions (no separate behavioral-semantics doc) ARE exempt under the same rule as sidecars below.

### Sidecars are exempt

Sidecars (`wos-delivery`, `wos-ontology-alignment`) do NOT require the three-section rubric when:

- The schema's `description` and `$comment` fields cover the Normative Contract (processor obligations are deployment-environment configuration, not behavioral semantics).
- The schema's `targetWorkflow` URI joining + ADR 0076 D-3 prose covers Composition (sidecars compose by URI and MUST NOT alter case state).
- Conformance reduces to schema validation (no runtime behavior beyond what the schema validates).

Sidecars under this exemption are indexed in [`specs/sidecars/README.md`](specs/sidecars/README.md) — one paragraph per sidecar naming what it does, how it joins, where its conformance lives. The exemption applies because the schema description fields are dense enough to substitute for prose, and the cold-read test fails for prose that restates schema content (six weeks later it drifts and someone hand-reconciles).

The exemption does NOT extend to layered-sieve specs whose semantics genuinely require prose. ADR 0076 D-3 establishes the sidecar/layered-sieve distinction; this rubric tracks that boundary.

## Normative Contract

Define processor obligations using MUST/SHOULD/MAY language.

Minimum expectations:

- Each MUST is written as an explicit statement, not buried in narrative prose.
- Each MUST is either:
  - enforced by schema validation, or
  - enforced by lint, or
  - enforced by runtime behavior with a conformance fixture (preferred when observable).
- Any MUST that is not yet enforceable is called out explicitly as a gap, with a tracking ID.

## Composition

Explain how this spec composes with the rest of WOS. Answer these questions directly:

- Where does this attach (kernel, governance, AI integration, advanced governance, sidecar)?
- Precedence: what wins when requirements conflict?
- Conflict handling: reject, override, or merge? If override/merge, what is the deterministic rule?
- Versioning/migration: what changes require a version bump vs. a lint warning vs. a runtime tolerance?

If the spec introduces a new "seam", name it and state what other specs are allowed to plug into it.

## Conformance

Define how correctness is checked.

Minimum expectations:

- Enumerate what is checked by:
  - schema validation
  - lint rules
  - runtime conformance (fixtures)
- For each non-trivial normative behavior, include at least one executable fixture that demonstrates it.
- If a behavior cannot be expressed as a fixture yet (e.g., requires external time or IO), state the intended test strategy and the current limitation.

## Examples normative status

Schema `examples` entries are load-bearing for the LLM authoring loop: authors and authoring agents read examples first and copy from them. A drifted example is a factory for invalid documents.

Every `examples[i]` entry MUST validate against the sub-schema that declares it. The CI test at `tests/schemas/test_examples_validate.py` walks every classified schema, collects every `examples` array under any sub-schema (root, `properties.<key>`, `$defs.<key>`, `items`, `oneOf`/`anyOf`/`allOf` branches, conditional branches), and asserts each entry validates against the fragment that owns it.

When updating a schema:

- Update the example in lockstep with the constraint change.
- If an example becomes infeasible (e.g., needs runtime data the schema cannot describe), remove the example rather than leaving it drifted.
- Prefer minimal, copy-pasteable examples that exercise one feature each. Composite examples are easier to drift and harder to repair.

## x-lm / x-wos Annotation Conventions

- `x-lm.critical: true` marks load-bearing leaf properties. Those nodes MUST meet the stricter `description` and `examples` bar enforced by `SCHEMA-DOC-001`.
- `x-wos.openStringKind` marks intentional open string leaves. Use it only on string leaves without `enum`, `const`, or `pattern`. Allowed values: `prose`, `fel`, `uri`, `identifier`, `pathExpression`, `hash`, `timestamp`, `tagLabel`. `SCHEMA-OPEN-001` enforces the annotation; it records why the leaf stays open. For the open-string-leaf ratchet (`open_string_leaf_ratchet.rs`), a leaf carrying a **listed** `openStringKind` counts as **constrained** the same way as `enum`/`const`/`pattern` so honest opens lower the baseline without lying about vocabulary closure.
- Bulk helpers (for example `work-spec/scripts/annotate_open_string_kinds.py`) assign kinds by heuristics. Treat their output as a draft: spot-check or domain-review important leaves. `lint_schema` plus the open-string ratchet enforce **shape** (allowed kinds, placement rules), not whether a given kind is the best semantic label for each path.

## Sidecar Normative-Contract Audit Rubric (TODO #45)

Use this when auditing sidecars and deciding whether to keep them as independent specs.

### Step 0: Does This Sidecar Deserve Independent Existence?

A sidecar earns independent existence only when it has at least one of:

- A distinct semantic model that does not fit cleanly into an existing spec
- A distinct artifact lifecycle (authorship, review, ownership, release cadence) from the core tiers

If neither is true, prefer merging it into the closest host spec.

### Three-Question Rubric

1. **Structure**
   - Is there a schema surface that matches the prose (no normative behavior that has zero authoring/configuration surface unless explicitly intentional)?
   - Are escape hatches (`additionalProperties`, open unions) justified and bounded?
2. **Semantics**
   - Are processor obligations explicit (what the processor MUST do, not what an author "can" express)?
   - Are failure modes defined (reject vs. warn vs. ignore vs. record provenance)?
3. **Composition**
   - Is the attachment point explicit (where in evaluation/execution this applies)?
   - Are precedence and conflicts with other sidecars/specs resolved deterministically?
