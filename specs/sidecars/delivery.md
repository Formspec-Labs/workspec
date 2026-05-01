---
title: WOS Delivery Sidecar
version: 1.0.0-draft.1
date: 2026-05-01
status: draft
---

> **Normative home.** `$wosDelivery` is the deployment-environment sidecar for calendar, notification templates, and correspondence metadata. The merged schema is the source of truth; this file holds the prose home.

# WOS Delivery Sidecar

## 1. Scope

A `$wosDelivery` sidecar joins a workflow by `targetWorkflow` and carries deployment-environment data only. It does not alter case state, lifecycle semantics, or the kernel event model.

Calendar, notification, and correspondence details are split into the absorbed reference docs:

- [`business-calendar.md`](business-calendar.md) for calendar semantics.
- [`notification-template.md`](notification-template.md) for notice-rendering semantics.
- [`../kernel/correspondence-metadata.md`](../kernel/correspondence-metadata.md) for the historical correspondence reference.

## 2. Calendar

The `calendar` block carries business-day, holiday, and operating-hours data for SLA evaluation and temporal parameter resolution. See Delivery §2 in the schema descriptions; the detailed algorithm stays in the absorbed calendar reference.

## 3. Notification Templates

The `notifications` block carries named templates for adverse decisions, holds, appeals, SLA warnings, and status notices. See Delivery §3 in the schema descriptions; the detailed rendering rules stay in the absorbed notification reference.

## 4. Correspondence

The `correspondence` block carries case-state metadata for correspondence entries. Its closed vocabularies are shared across `EntryTemplate` and `CorrespondenceEntry`.

Delivery uses `correspondenceRole`, not kernel `actorType`, because kernel `actorType` classifies the workflow actor (`human`, `system`, `agent`) while delivery records the party's communication role.

### 4.1 Vocabularies

- `channel`: `in-person`, `phone`, `mail`, `email`, `portal`, `fax`.
- `direction`: `inbound`, `outbound`.
- `correspondenceRole`: `applicant`, `representative`, `third-party`, `system`, `agency`.

### 4.2 EntryTemplate

An entry template defines `id`, `channel`, `direction`, `correspondenceRole`, optional `description`, and optional `requiredFields`. `requiredFields` adds entry fields beyond the base correspondence entry shape: `templateRef`, `channel`, `direction`, `correspondenceRole`, `contentRef`, `summary`, `timestamp`.

### 4.3 CorrespondenceEntry

A correspondence entry stores `templateRef`, `channel`, `direction`, `correspondenceRole`, `contentRef`, `summary`, `timestamp`, and any template-required fields. `contentRef` is a claim-check pointer; the payload lives elsewhere.

## 5. Conformance

A delivery sidecar validates by schema. Calendar and notification blocks are deployment configuration, not runtime behavior. Correspondence metadata catalogs communications; it does not add event types or change lifecycle semantics.

```json
{
  "$wosDelivery": "1.0",
  "targetWorkflow": "https://agency.gov/workflows/benefits-adjudication",
  "correspondence": {
    "correspondenceField": "caseFile.correspondence",
    "entryTemplates": [
      {
        "id": "inboundMail",
        "channel": "mail",
        "direction": "inbound",
        "correspondenceRole": "applicant",
        "requiredFields": ["contentRef"]
      }
    ]
  }
}
```
