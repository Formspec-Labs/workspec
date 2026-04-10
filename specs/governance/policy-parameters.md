---
title: WOS Policy Parameter Config
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS Policy Parameter Config v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Sidecar to:** WOS Workflow Governance Specification v1.0

---

## Abstract

The WOS Policy Parameter Config is a sidecar document that provides date-indexed parameter values for temporal parameter resolution in WOS workflows. Government workflows apply rules effective at specific dates, not today's date: income thresholds change annually, benefit rates adjust quarterly, eligibility criteria evolve with legislative amendments. This sidecar declares parameters with their effective-date schedules and resolution date references, enabling the workflow to apply the correct rules for any given case.

This follows the OpenFisca model of date-indexed parameter values.

---

## Status of This Document

This document is a **draft specification**. It is a sidecar to the WOS Workflow Governance Specification v1.0 and does not modify governance processing semantics.

---

## 1. Document Structure

### 1.1 Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `$wosPolicyParameters` | string | REQUIRED | Document type marker. MUST be `"1.0"`. |
| `targetWorkflow` | string (URI) | REQUIRED | URI of the WOS Kernel Document these parameters apply to. |
| `version` | string | OPTIONAL | Version of this config document. |
| `title` | string | OPTIONAL | Human-readable name. |
| `parameters` | map of ParameterDefinition | REQUIRED | Named parameters with date-indexed values. |
| `bindings` | map of RegulatoryBinding | OPTIONAL | Named bindings with date-indexed document references (S1.5). |
| `extensions` | object | OPTIONAL | Extension data. Keys MUST be prefixed with `x-`. |

### 1.2 Parameter Definition

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `description` | string | REQUIRED | Human-readable description of this parameter. |
| `type` | enum | REQUIRED | Parameter value type: `number`, `integer`, `string`, `boolean`. |
| `unit` | string | OPTIONAL | Unit of measure (e.g., `USD`, `days`, `percent`). |
| `resolutionDateRef` | string | REQUIRED | Case state field path that provides the resolution date (e.g., `caseFile.applicationDate`). Different parameters MAY resolve against different dates. |
| `values` | array of DateValue | REQUIRED | Date-indexed values, ordered by effective date. |
| `authority` | string | OPTIONAL | Regulatory or statutory authority for this parameter. |

### 1.3 Date-Indexed Values

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `effectiveDate` | string (date) | REQUIRED | Date this value becomes effective (ISO 8601 date). |
| `value` | any | REQUIRED | The parameter value effective from this date until the next entry. |
| `authority` | string | OPTIONAL | Specific authority for this value change (e.g., legislative citation). |

### 1.4 Resolution Mechanism

Resolution follows the kernel's evaluation context enrichment model (Kernel S7.3, Governance S13.2):

1. For each parameter, look up `resolutionDateRef` in case state to get the resolution date.
2. Find the most recent `values` entry whose `effectiveDate` is on or before the resolution date.
3. Inject the resolved value into the evaluation context under `parameters.[parameterName]`.

The workflow author writes `caseFile.income < parameters.eligibilityThreshold`. The system resolves `eligibilityThreshold` to the value effective on the date referenced by `resolutionDateRef`.

### 1.5 Regulatory Version Bindings

Government workflows bind against specific versions of external documents: Formspec Definitions, JSON Schemas, decision services, and Mapping DSL documents. These documents change on known effective dates, just as scalar parameters do. Regulatory version bindings declare which version of each external document applies for a given case, using the same date-indexed resolution mechanism as scalar parameters.

#### 1.5.1 Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `bindings` | map of RegulatoryBinding | OPTIONAL | Named bindings with date-indexed document references. |

#### 1.5.2 Regulatory Binding Definition

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique binding identifier. |
| `description` | string | REQUIRED | Human-readable description of what this binding controls. |
| `resolutionDateRef` | string | REQUIRED | Case state field path that provides the resolution date (same mechanism as parameters, S1.4). |
| `bindingType` | enum | REQUIRED | Type of document being versioned: `formspec`, `jsonSchema`, `service`, `mapping`. |
| `authority` | string | OPTIONAL | Regulatory authority reference for this binding. |
| `values` | array of BindingDateValue | REQUIRED | Date-indexed values, ordered by effective date. Each `value` is a URI string referencing the versioned document. |

#### 1.5.3 Binding Date Values

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `effectiveDate` | string (date) | REQUIRED | Date this binding becomes effective (ISO 8601 date). |
| `value` | string (URI) | REQUIRED | URI of the document version effective from this date. |
| `authority` | string | OPTIONAL | Specific authority for this version change. |

#### 1.5.4 Resolution Mechanism

Binding resolution follows the same mechanism as scalar parameter resolution (S1.4):

1. For each binding, look up `resolutionDateRef` in case state to get the resolution date.
2. Find the most recent `values` entry whose `effectiveDate` is on or before the resolution date.
3. Inject the resolved URI into the evaluation context under `parameters.[bindingId]`.

The workflow author references bindings the same way as parameters. The distinction is that the resolved value is a URI suitable for document retrieval, not a scalar for computation.

**Binding types:** The `bindingType` property declares what kind of document the URI references. Processors MAY use this to apply type-specific validation (e.g., resolving a `formspec` binding against a Formspec-conformant processor per Core S1.4, or validating a `jsonSchema` binding as a valid JSON Schema document). Processors MUST NOT alter the resolution mechanism based on `bindingType`.

### 1.6 Example

```json
{
  "$wosPolicyParameters": "1.0",
  "targetWorkflow": "https://agency.gov/workflows/benefits-adjudication",
  "version": "2026.1.0",
  "title": "Benefits Program Parameters FY2025-2026",
  "parameters": {
    "eligibilityThreshold": {
      "description": "Maximum household income for program eligibility",
      "type": "number",
      "unit": "USD",
      "resolutionDateRef": "caseFile.applicationDate",
      "authority": "42 USC 1396a",
      "values": [
        { "effectiveDate": "2025-01-01", "value": 35000 },
        { "effectiveDate": "2025-07-01", "value": 36500 },
        { "effectiveDate": "2026-01-01", "value": 38000 }
      ]
    },
    "appealDeadlineDays": {
      "description": "Number of days to file an appeal after adverse decision",
      "type": "integer",
      "unit": "days",
      "resolutionDateRef": "caseFile.adverseDecisionDate",
      "authority": "APA 5 USC 554",
      "values": [
        { "effectiveDate": "2020-01-01", "value": 30 },
        { "effectiveDate": "2025-06-01", "value": 45 }
      ]
    }
  },
  "bindings": {
    "eligibilityForm": {
      "id": "eligibilityForm",
      "description": "Formspec Definition for the eligibility determination form",
      "resolutionDateRef": "caseFile.applicationDate",
      "bindingType": "formspec",
      "authority": "Agency directive 2024-03",
      "values": [
        { "effectiveDate": "2025-01-01", "value": "https://agency.gov/forms/eligibility/v2.1" },
        { "effectiveDate": "2025-09-01", "value": "https://agency.gov/forms/eligibility/v3.0" }
      ]
    }
  }
}
```
