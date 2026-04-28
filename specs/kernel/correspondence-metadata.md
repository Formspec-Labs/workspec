---
title: WOS Correspondence Metadata Config
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS Correspondence Metadata Config v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Sidecar to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Correspondence Metadata Config is a sidecar document that declares the metadata schema for correspondence entries stored in case state. Government workflows track correspondence -- letters, phone calls, emails, portal submissions that have been acknowledged as correspondence, in-person interactions -- as part of the case record. The kernel's existing event mechanism handles correspondence events: events that match no transition are recorded in provenance (Kernel S4.9). This sidecar defines the structured metadata that each correspondence entry carries, enabling consistent cataloging and retrieval without adding new event types or modifying lifecycle semantics.

This sidecar is additive. It does not alter kernel processing semantics, lifecycle evaluation, or the event matching algorithm.

---

## Status of This Document

This document is a **draft specification**. It is a sidecar to the WOS Kernel Specification v1.0 and does not modify kernel processing semantics.

---

## 1. Document Structure

Correspondence metadata is declared as the `correspondence` block within a `$wosDelivery` sidecar. The sidecar joins to the target workflow via `targetWorkflow` (the `url` of the `$wosWorkflow` document). It declares which case state fields store correspondence entries and what metadata each entry carries.

### 1.1 Properties

The `correspondence` block carries:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `correspondenceField` | string | REQUIRED | Case state field path where correspondence entries are stored (e.g., `caseFile.correspondence`). |
| `entryTemplates` | array of EntryTemplate | REQUIRED | Templates defining the metadata structure for correspondence entries. |
| `extensions` | object | OPTIONAL | Extension data. Keys MUST be prefixed with `x-`. |

### 1.2 Entry Template

Each entry template defines the metadata carried by a category of correspondence entry. Templates allow different metadata requirements for different correspondence types (e.g., inbound mail requires a scan reference; phone calls require a duration).

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique template identifier. |
| `description` | string | OPTIONAL | Human-readable description of when this template applies. |
| `channel` | enum | REQUIRED | Communication channel: `in-person`, `phone`, `mail`, `email`, `portal`, `fax`. Extensible via `x-` prefixed values. |
| `direction` | enum | REQUIRED | Direction of the correspondence: `inbound`, `outbound`. |
| `actorType` | enum | REQUIRED | Who sent or received the correspondence: `applicant`, `representative`, `third-party`, `system`, `agency`. |
| `requiredFields` | array of string | OPTIONAL | Fields that MUST be present in a correspondence entry using this template (beyond the base metadata). |
| `extensions` | object | OPTIONAL | Extension data. Keys MUST be prefixed with `x-`. |

### 1.3 Correspondence Entry Metadata

Each correspondence entry stored in case state carries the following base metadata. These fields are defined by this sidecar and populated by the workflow when recording correspondence.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `templateRef` | string | REQUIRED | The `id` of the entry template this entry follows. |
| `channel` | enum | REQUIRED | Communication channel (from the template's allowed values). |
| `direction` | enum | REQUIRED | `inbound` or `outbound`. |
| `actorType` | enum | REQUIRED | Actor type for this correspondence. |
| `contentRef` | string | REQUIRED | Claim check reference -- a URI or storage reference for the actual content. The content itself is not stored in case state; this is a pointer to it. |
| `summary` | string | REQUIRED | Human-readable summary of the correspondence. |
| `relatedTaskRef` | string | OPTIONAL | Reference to a task this correspondence relates to. |
| `timestamp` | string (date-time) | REQUIRED | When the correspondence occurred (ISO 8601). |

### 1.4 Correspondence and the Kernel Event Model

Correspondence does not introduce a new event type. The workflow author defines correspondence-related events in the kernel lifecycle as needed. Common patterns:

- **Correspondence as a lifecycle event:** An event like `correspondenceReceived` triggers a transition (e.g., from `awaitingDocuments` to `documentsReceived`). The correspondence metadata is stored in case state; the event drives the lifecycle.
- **Correspondence as a provenance-only event:** An event like `phoneCallLogged` matches no transition and is recorded in provenance per Kernel S4.9. The correspondence metadata is still stored in case state.

In both cases, the correspondence metadata sidecar defines the structure of the case state entry -- it does not control whether the event triggers a transition.

### 1.5 Example

The `correspondence` block embedded in a `$wosDelivery` sidecar:

```json
{
  "$wosDelivery": "1.0",
  "targetWorkflow": "https://agency.gov/workflows/benefits-adjudication",
  "correspondence": {
    "correspondenceField": "caseFile.correspondence",
    "entryTemplates": [
      {
        "id": "inboundMail",
        "description": "Physical mail received from applicant or representative",
        "channel": "mail",
        "direction": "inbound",
        "actorType": "applicant",
        "requiredFields": ["contentRef"]
      },
      {
        "id": "outboundNotice",
        "description": "Official notice sent to applicant",
        "channel": "mail",
        "direction": "outbound",
        "actorType": "agency",
        "requiredFields": ["contentRef", "relatedTaskRef"]
      },
      {
        "id": "phoneContact",
        "description": "Phone call with applicant or representative",
        "channel": "phone",
        "direction": "inbound",
        "actorType": "applicant"
      }
    ]
  }
}
```
