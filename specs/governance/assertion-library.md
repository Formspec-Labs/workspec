---
title: WOS Assertion Gate Library
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS Assertion Gate Library v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Sidecar to:** WOS Workflow Governance Specification v1.0

---

## Abstract

The WOS Assertion Gate Library is a sidecar document that provides a reusable collection of assertion gate definitions for WOS data validation pipelines. An Assertion Gate Library -- itself a JSON document -- declares named assertions with their types, expressions, field bindings, and descriptions. Pipelines in Workflow Governance Documents reference these assertions by identifier, enabling shared assertion definitions across multiple pipelines and governance documents.

---

## Status of This Document

This document is a **draft specification**. It is a sidecar to the WOS Workflow Governance Specification v1.0 and does not modify governance processing semantics.

---

## 1. Document Structure

### 1.1 Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `$wosAssertionLibrary` | string | REQUIRED | Document type marker. MUST be `"1.0"`. |
| `url` | string (URI) | OPTIONAL | Canonical URI identifier for this library. |
| `version` | string | OPTIONAL | Version of this library document. |
| `title` | string | OPTIONAL | Human-readable name. |
| `assertions` | array of AssertionDefinition | REQUIRED | Reusable assertion definitions. |
| `extensions` | object | OPTIONAL | Extension data. Keys MUST be prefixed with `x-`. |

### 1.2 Assertion Definition

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique assertion identifier. Referenced by pipelines. |
| `type` | enum | REQUIRED | One of: `source-grounded`, `arithmetic`, `range`, `consistency`, `format`, `cross-document`, `temporal`. |
| `expression` | string (FEL) | OPTIONAL | FEL expression defining the assertion constraint. |
| `fields` | array of string | OPTIONAL | Fields subject to this assertion. |
| `description` | string | REQUIRED | Human-readable description of what this assertion checks. |
| `rejectionPolicy` | enum | OPTIONAL | Default rejection policy: `retryWithCorrections`, `escalateToSupervisor`, `holdPendingData`, `failWithExplanation`. |

### 1.3 Usage

Pipelines reference library assertions by `id`. The pipeline stage inherits the assertion's type, expression, fields, and rejection policy unless overridden at the stage level.

---

## 2. Cross-Document Reference Protocol

The Assertion Gate Library is a reusable catalogue; individual [pipeline stages](workflow-governance.md) carry the authority to *use* a library entry at a given point in a workflow. Two schema shapes express that usage: an **inline assertion body** carried directly on the stage, and a **reference** that resolves into a library-defined body at load time. This subsection is the normative contract for how processors MUST handle the two shapes. The seam on the library schema is `#/$defs/AssertionUse`, which is a `oneOf` over `AssertionInlineUse` and `AssertionReference`. The governance spec's `PipelineStage.assertions[]` is the intended consumer — each item resolves through this seam before any assertion logic runs.

**Adoption path today.** Consuming `AssertionUse` from another schema (for example `wos-workflow-governance.schema.json` on `PipelineStage.assertions[]`) requires one of two moves: (a) a cross-schema URI `$ref` from the governance schema into `wos-assertion-gate.schema.json#/$defs/AssertionUse` — no schema in this repository currently does cross-schema URI `$ref` plumbing, so that path is untested territory and its validator/tooling behaviour under Draft 2020-12 would need to be exercised first; or (b) duplicating the three local `$def`s (`AssertionReference`, `AssertionInlineUse`, `AssertionUse`) into the consumer schema. The §4.5 planned merge of this library into Workflow Governance (TODO.md §4.5) dissolves the choice: once the merge lands the three `$def`s move into `wos-workflow-governance.schema.json` directly and consumers drop the cross-schema `$ref` question entirely. The seam in this spec is designed to migrate cheaply — a consuming schema that inlines the three `$def`s today will swap them for a local-`$ref` on merge with no shape change.

### 2.1 Shape of a Reference

An **inline** assertion is a body with the fields defined in §1.2 and an optional `assertionId` that pins the inline body to a stable external name. A **reference** is an object whose only data-bearing key is `assertionRef`, a URI that targets a published Assertion Gate Library.

Inline form:

```json
{
  "type": "arithmetic",
  "description": "Totals sum cleanly",
  "expression": "totalIncome = wageIncome + investmentIncome",
  "assertionId": "totalIncomeArithmetic"
}
```

Reference form:

```json
{
  "assertionRef": "https://agency.gov/assertion-libraries/income-verification#totalIncomeArithmetic"
}
```

The two forms are mutually exclusive at the schema layer; once `assertionRef` resolves, downstream processors MUST treat the reference identically to a locally-declared inline body with the same fields.

`assertionId` appears in two distinct roles and authors MUST NOT conflate them. On an `AssertionDefinition` (a library entry, §1.2) it is the catalogue's published name for the assertion. On an inline `AssertionInlineUse` body it is **permitted standalone** — it names the inline assertion for audit-log stability across inline↔reference rewrites and does not imply a library lookup. An inline body bearing `assertionId: X` is NOT required to correspond to any `AssertionDefinition.id` in a connected library; the processor MUST NOT treat an inline `assertionId` as a pointer into the library set. The pattern is shared across both roles only so the two identifiers can be compared when both are present (see §2.2.4).

### 2.2 Resolution Semantics

Throughout this section, **"configuration error"** means a condition the processor MUST detect at document-load time and reject; runtime evaluation MUST NOT proceed with a document carrying a configuration error. The term is used identically in `specs/governance/workflow-governance.md` §3.6 (Continuation of Service) and `specs/sidecars/business-calendar.md` §7.2 (Multi-Calendar Composition); those spec sites do not normatively define the phrase. The gloss here serves as the working definition for this document.

Resolution happens **at load time**, before any pipeline execution begins. The following rules are normative:

1. **Source of truth.** The referenced body is drawn from the WOS Assertion Gate Library sidecar whose `url` matches the authority + path portion of `assertionRef`, and whose `assertions[]` contains an entry whose `id` equals the key encoded in the reference URI (the fragment for HTTP(S) URIs, the terminal sub-component for `urn:` URIs). Processors MUST use a single, configured library map — they MUST NOT fetch unknown libraries at runtime.
2. **Unresolvable reference.** If no configured library matches the reference URI, or no `id` in that library matches the selector, the processor MUST reject the enclosing document as a **configuration error** at load time. Lazy resolution at pipeline execution is forbidden: failure to resolve is a deploy-time failure, not a per-case failure.
3. **Conflicting `id` values.** If two configured libraries each declare an assertion with the same `id` and either could satisfy a reference, the processor MUST reject the configuration as an error. This follows the existing [single-source authority](workflow-governance.md) principle — every named contract has exactly one owning document. Disambiguation belongs in the URI, not in runtime fallback logic.
4. **Self-declared `assertionId`.** If the referenced assertion body carries its own `assertionId` (permitted by `AssertionInlineUse` for bodies later lifted into a library), that `assertionId` MUST equal the `id` under which the library published the entry. Mismatch is a configuration error.

### 2.3 Override Precedence

Inline bodies and library references MUST NOT be combined on the same item. Schema enforces this via `oneOf` on `AssertionUse` plus `additionalProperties: false` on each branch — mixing keys is a **configuration error**, not a silent merge. This rule is intentional: a partial override mechanism would introduce a second dimension of assertion semantics (the inline body, the referenced body, and the projection of one onto the other) whose interactions are expensive to reason about and impossible to round-trip through audit traces. Authors who need a variant of a library assertion MUST either publish a new library entry with a distinct `id` or inline the full body. The either-or rule keeps the audit trail's authority pointer unambiguous.

### 2.4 Lint Follow-up

A new Tier-2 lint rule, **G-064 `assertion-library-resolution`**, is **planned; not yet implemented** (tracked in `TODO.md` §4.4 #38). G-064 will check that (a) every `assertionRef` URI resolves against the configured library set, (b) no two configured libraries declare colliding `id` values, and (c) when an `assertionRef` resolves to a library body that carries its own `assertionId`, that `assertionId` MUST match the library `id` (mirroring §2.2.4 — note a standalone inline `assertionId` per §2.1 is out of scope because it has no library pointer). G-064 SHALL also flag `assertionRef` URIs whose structure does not identify a specific library entry — e.g. an `https://…` URI with no fragment, or a `urn:…` URI with no terminal `:`-separated segment after the library namespace — since such URIs pass `format: uri` but cannot resolve to an entry. Until G-064 lands, processors are solely responsible for rejecting these configurations at load time; schema validation only covers URI syntax.
