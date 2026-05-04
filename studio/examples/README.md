# Studio (Authoring) — Worked examples

This folder contains **vertical slices**: end-to-end authoring journeys from real source documents to compiled `$wosWorkflow` artifacts. Each slice exercises every Studio concept at low complexity to prove the framework can produce real wos-spec content.

**Audience:** developers, reviewers evaluating Studio fitness, parent-stack consumers verifying composition.

## Slices

| Slice | Domain | Sources | Output | Purpose |
|---|---|---|---|---|
| [`snap-redetermination-from-sources/`](snap-redetermination-from-sources/) | SNAP redetermination (state) | 7 CFR §273 excerpt, state SNAP manual excerpt, office memo, 2024 corrective-action letter | `wos-workflow.json` (compatible shape with parent [`../../examples/benefits-adjudication.workflow.json`](../../examples/benefits-adjudication.workflow.json)) | Demonstrate Sarah Chen's persona authoring journey end-to-end. Closes "wrong-by-omission" gap from parent BLUF. |

## Status

Stage-2 (authoring): JSON files are illustrative — schemas (Stage 3) are not yet authored, so files do not validate against any schema. **Shape is plausible**; structural truth comes when Stage-3 schemas land. Each slice's README details what each file demonstrates and where Stage-3 work will tighten it.

## Composition with parent fixtures

These slices intentionally **compose** with parent [`../../examples/`](../../examples/) and [`../../crates/wos-conformance/fixtures/`](../../crates/wos-conformance/fixtures/) where possible — the compiled `wos-workflow.json` produced here mirrors the existing `benefits-adjudication.workflow.json` shape but adds richer content closing the BLUF's flagged "wrong-by-omission" gap.

## Cross-references

- Parent examples: [`../../examples/`](../../examples/) (`benefits-adjudication.workflow.json`, BLUF, `nda.workflow.json`, `timeoff.workflow.json`).
- Parent conformance fixtures: [`../../crates/wos-conformance/fixtures/`](../../crates/wos-conformance/fixtures/).
- Studio specs: [`../specs/`](../specs/).
- Studio concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md).
