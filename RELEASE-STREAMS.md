# WOS Release Streams

> **Updated 2026-04-28 per [ADR 0076](../thoughts/adr/0076-product-tier-consolidation.md) D-7:** the schema family collapses from 27 → 6 schemas with one author-time workflow marker, `$wosWorkflow`. Embedded-block semantics share the containing workflow format version; sidecars, runtime artifacts and tooling retain their own family markers. Compliance claims now compose as `$wosWorkflow@X.Y` plus the **claims map** below. The four-stream cadence and source-path framing below is preserved for repository-engineering hygiene (Changesets groupings, per-tier cadence) but is no longer the authoring or compliance-claim surface.

## Claims map (post-ADR-0076)

A vendor implementing `$wosWorkflow@1.0` declares which embedded blocks they exercise:

| Embedded block | Conformance suite | Old stream framing (reference) |
|---|---|---|
| (none — bare core) | Kernel-Basic conformance | `wos-kernel@1.0` |
| `governance` | Governance fixtures + due-process pipeline tests | `wos-governance@1.0` |
| `agents` + `aiOversight` | AI deontic + autonomy + drift fixtures | `wos-ai@1.0` |
| `signature` | T4 SIG-* conformance suite (signature.md ⇒ embedded `signature` block) | T4 (separate per Q11) |
| `custody` | Trellis custody-anchor cross-stack fixtures | (cross-stack, not stream-scoped) |
| `advanced` | Advanced equity + DCR + verifiable-constraints fixtures | `wos-advanced@1.0` |
| `assurance` | Assurance attestation fixtures | (typically paired with governance) |

A vendor's claim string: `$wosWorkflow@1.0 [governance, agents, aiOversight, signature]`.

Repository package versions and release tags identify the source or binary build.
They do not create independent governance, AI or signature document versions.
A conformance claim identifies the workflow format version, the blocks exercised
and the relevant conformance profile or suite. Merely declaring a block does not
prove conformance; the claim needs evidence from the named checks.

See [`COMPATIBILITY-MATRIX.md`](COMPATIBILITY-MATRIX.md) for the distinction between
historical stream ranges and a tested current source combination.

## Why four streams?

The [release-trains plan](thoughts/plans/2026-04-16-wos-release-trains.md) proposed
four repository release cadences: kernel, governance, AI and advanced. The labels
remain useful for package provenance and historical changes. ADR 0076 D-7
supersedes their use as independently versioned authored documents or vendor
compliance claims. It does not erase package versions or release history.

## Stream → path mapping

The former directory map (`schemas/kernel/**`, `schemas/governance/**`,
`schemas/ai/**`, `schemas/advanced/**`, and `specs/companions/**`) predates the
consolidation and is not a current file-ownership rule. Current source locations
are:

| Surface | Current owner |
|---|---|
| Authored workflow and embedded blocks | `schemas/wos-workflow.schema.json` |
| Normative block semantics | `specs/kernel/`, `specs/governance/`, `specs/ai/`, `specs/advanced/`, `specs/assurance/`, and retained `specs/profiles/signature.md` |
| Deployment sidecars | `schemas/sidecars/wos-delivery.schema.json`, `schemas/sidecars/wos-ontology-alignment.schema.json` |
| Runtime records | `schemas/wos-process.schema.json`, `schemas/wos-provenance-log.schema.json` |
| Tooling documents | `schemas/wos-tooling.schema.json` |

Schema support resources and API schemas do not introduce more authored-workflow
markers. The six-family routing is maintained in `tests/schemas/conftest.py`.

## Cross-cutting crates and sidecars

`wos-lint` and `wos-conformance` check the claimed workflow format and applicable
block semantics. `LINT-MATRIX.md` and rule-coverage reports describe those checks;
they do not independently version a block. Signature T4 conformance remains a
separate checking suite against the embedded `signature` block.

The delivery and ontology-alignment sidecars describe deployment configuration
and refer to a workflow through `targetWorkflow`. Signature and custody are
embedded blocks, not authoring sidecars. Runtime, authoring, MCP and synthesis
crates keep their declared package identities and dependencies.

## Tag convention

Historical per-stream tags use `<stream>-v<X.Y.Z>` (for example
`kernel-v1.0.0`). A tag or package version identifies repository provenance,
not evidence that every combination of stream versions interoperates. Current
qualification must bind the exact source and consumed artifacts to test results.

## Changesets

The release-trains plan describes Changesets with one fixed group per stream.
This checkout has no `.changeset` configuration; this document does not claim
that planned automation runs today. Use the maintained package manifests and
release tooling for build identities. Do not infer a workflow compliance version
from a proposed stream grouping.

## How to pick a stream when editing

1. Identify the affected schema, normative prose and consuming code using the
   current owner table above.
2. Preserve their declared package release identities. Follow the maintained
   release process for the packages actually changed.
3. State the workflow format, blocks and conformance checks affected. Report
   exact verification separately from historical package tags.

## References

- [Release-trains implementation plan](thoughts/plans/2026-04-16-wos-release-trains.md) — historical repository release proposal.
- [Compatibility matrix](COMPATIBILITY-MATRIX.md) — qualification evidence requirements.
- [ADR 0076 — Product-tier consolidation](../thoughts/adr/0076-product-tier-consolidation.md) — D-7 versioning and D-8 retained prose.
