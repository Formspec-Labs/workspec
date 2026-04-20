# WOS Spec Conventions

This repo treats specs as executable contracts: prose defines normative processor behavior, schemas encode authoring surfaces, and conformance fixtures/lint rules prevent drift.

## Required Sections (for any new or materially revised spec)

Every spec MUST include these sections (or a clear explanation of why one is not applicable):

1. **Normative Contract**
2. **Composition**
3. **Conformance**

These are not "nice to have" documentation. They are how we keep the spec, schemas, runtime behavior, and fixtures aligned.

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

