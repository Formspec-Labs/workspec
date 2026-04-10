---
title: WOS Notification Template Sidecar
version: 1.0.0-draft.1
date: 2026-04-10
status: draft
---

# WOS Notification Template Sidecar v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-10
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS Notification Template Sidecar defines reusable templates for notices that WOS workflows generate during governance events: hold notifications, adverse decision notices, appeal instructions, SLA warnings, and case status updates. Government workflows have strict notice requirements -- an adverse benefits decision must include the specific determination, individualized reason codes, appeal rights, and filing deadlines (Governance S3.2). This sidecar separates notice content from governance logic, allowing templates to be versioned, localized, and audited independently.

The Workflow Governance Specification references notification templates via `notificationTemplateRef` in Hold Policy properties (Governance S12.2) and `noticeTemplateRef` in Adverse Decision Policy properties (Governance S3.1). This sidecar provides the template definitions those references resolve to.

This is a sidecar document, not a layer. It provides template data consumed by existing governance mechanisms without introducing new seams, document types, or processing concepts.

WOS MUST NOT alter core Formspec processing semantics. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is a sidecar to the WOS Kernel Specification v1.0 and the WOS Workflow Governance Specification v1.0. It defines notification template structures consumed by governance processors for notice generation.

---

## 1. Introduction

### 1.1 Purpose

Government agencies must provide legally adequate notice for consequential workflow events. The specific content, format, and delivery requirements depend on the event type and regulatory context:

- **Adverse decision notices** must include the specific determination, individualized reasons, appeal instructions, and filing deadlines (due process requirements per APA, ECOA Regulation B, OMB M-24-10).
- **Hold notifications** must inform the case subject of the hold reason, expected duration, and any required actions.
- **Appeal acknowledgments** must confirm receipt and provide the review timeline.
- **SLA warnings** must notify responsible actors of approaching deadlines.

The Workflow Governance Specification references notification templates in:

- **Governance S3.1** (Adverse Decision Policy): `noticeTemplateRef` for adverse decision notices.
- **Governance S12.2** (Hold Policy Properties): `notificationTemplateRef` for hold notifications.

This sidecar provides the template definitions those references resolve to.

### 1.2 Scope

**Within scope:** notification template definitions with category, required sections, placeholder variables, delivery channel, and localization references.

**Out of scope:** rendering engines (implementation concern); delivery mechanisms (email, postal, portal -- implementation concern); template authoring tools; Formspec Definition rendering (Core S1.4).

### 1.3 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

---

## 2. Document Structure

A Notification Template sidecar is a JSON document identified by the `$wosNotificationTemplate` document type marker. It targets a WOS Kernel Document via the `targetWorkflow` property and declares a collection of named notification templates.

### 2.1 Required Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `$wosNotificationTemplate` | string | REQUIRED | Document type marker. MUST be `"1.0"`. |
| `targetWorkflow` | string (URI) | REQUIRED | URI of the WOS Kernel Document this sidecar applies to. |
| `templates` | object (map of Template) | REQUIRED | Named notification templates. Template keys are the identifiers referenced by `notificationTemplateRef` and `noticeTemplateRef` in governance documents. |

### 2.2 Optional Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `version` | string | OPTIONAL | Version of this Notification Template document. |
| `title` | string | OPTIONAL | Human-readable name. |
| `description` | string | OPTIONAL | Human-readable description. |
| `extensions` | object | OPTIONAL | Extension data. All keys MUST be prefixed with `x-`. |

---

## 3. Notification Templates

### 3.1 Template Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `category` | enum | REQUIRED | Template category. Determines which governance mechanism uses this template. |
| `description` | string | OPTIONAL | Human-readable description of when this template is used. |
| `subject` | string | OPTIONAL | Subject line or title for the notification. MAY contain placeholders. |
| `sections` | array of Section | REQUIRED | Ordered content sections that compose the notification body. |
| `requiredVariables` | array of string | OPTIONAL | Variables that MUST be present in the rendering context. If any required variable is missing, the processor MUST NOT send the notification and MUST record the failure in provenance. |
| `deliveryChannels` | array of enum | OPTIONAL | Channels through which this notification can be delivered. When absent, delivery channel is implementation-defined. |
| `localeRef` | string | OPTIONAL | Reference to a locale-specific variant of this template. |
| `authority` | string | OPTIONAL | Regulatory or statutory authority requiring this notification. |

### 3.2 Template Categories

| Category | Governance Reference | Description |
|----------|---------------------|-------------|
| `adverse-decision` | Governance S3.1-S3.2 | Notice of adverse decision. MUST include determination, reason codes, appeal rights, and filing deadlines. |
| `hold-notification` | Governance S12.2 | Notice that a case has been placed on hold. MUST include hold reason and expected duration. |
| `appeal-acknowledgment` | Governance S3.5 | Acknowledgment that an appeal has been received. SHOULD include review timeline. |
| `sla-warning` | Governance S10.3 | Warning that an SLA deadline is approaching. |
| `case-status-update` | General | General case status notification. |
| `resume-notification` | Governance S12.3 | Notice that a case has resumed from hold. |

### 3.3 Delivery Channels

| Channel | Description |
|---------|-------------|
| `postal` | Physical mail. |
| `email` | Electronic mail. |
| `portal` | Web portal notification or message. |
| `sms` | Text message. |
| `in-app` | In-application notification. |

---

## 4. Template Sections

### 4.1 Section Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Section identifier. |
| `title` | string | OPTIONAL | Section heading. |
| `contentType` | enum | REQUIRED | Type of content in this section. |
| `content` | string | CONDITIONAL | Static content or content with placeholder variables. Required when `contentType` is `text` or `structured`. |
| `required` | boolean | OPTIONAL | Whether this section MUST appear in the rendered notification. Default: `true`. |
| `condition` | string | OPTIONAL | FEL expression that controls section visibility. When the expression evaluates to `false`, the section is omitted. |

### 4.2 Content Types

| Type | Description |
|------|-------------|
| `text` | Free-text content with optional placeholder variables. |
| `structured` | Structured content with labeled fields (e.g., determination details, reason codes). |
| `appeal-rights` | Standardized appeal rights block. Processor MUST include filing deadline, review body, and continuation-of-services status. |
| `action-required` | Block describing actions the recipient must take. |
| `contact-information` | Contact details for questions or assistance. |

### 4.3 Placeholder Variables

Template content MAY contain placeholder variables using the `{{variableName}}` syntax. Variables are resolved from the case state and governance context at render time.

Standard variables available in all templates:

| Variable | Source | Description |
|----------|--------|-------------|
| `caseId` | Case instance | The case identifier. |
| `applicantName` | Case state | Name of the case subject. |
| `currentDate` | Runtime | Current date in the calendar's timezone. |
| `currentState` | Case instance | Current lifecycle state. |

Category-specific variables:

| Variable | Category | Description |
|----------|----------|-------------|
| `determination` | `adverse-decision` | The specific determination made. |
| `reasonCodes` | `adverse-decision` | Array of individualized reason codes. |
| `appealDeadline` | `adverse-decision` | Date by which an appeal must be filed. |
| `appealBody` | `adverse-decision` | Name of the review body. |
| `continuationOfServices` | `adverse-decision` | Whether services continue during appeal. |
| `holdReason` | `hold-notification` | Reason the case was placed on hold. |
| `expectedDuration` | `hold-notification` | Expected hold duration. |
| `requiredAction` | `hold-notification` | Action the recipient must take to resume the case. |
| `slaDeadline` | `sla-warning` | SLA deadline date. |
| `taskName` | `sla-warning` | Name of the task approaching its deadline. |

### 4.4 Adverse Decision Template Requirements

Templates with category `adverse-decision` MUST include sections that address the following due process requirements (Governance S3.2):

1. The specific determination made (not a generic notice).
2. Individualized reason codes explaining why the determination was made.
3. Appeal rights, including filing deadline and review body.
4. Instructions for filing an appeal.
5. Whether services continue during the appeal period (when `continuationOfServices` is configured in the governance document).

A processor MUST reject an `adverse-decision` template that does not include sections addressing items 1-4. Item 5 is required only when the governance document configures `continuationOfServices`.

---

## 5. Template Resolution

### 5.1 Reference Resolution

When a governance document's `notificationTemplateRef` or `noticeTemplateRef` value matches a template key in a Notification Template sidecar targeting the same workflow, the processor resolves the reference to that template.

### 5.2 Missing Template

When a `notificationTemplateRef` or `noticeTemplateRef` references a template key that does not exist in any Notification Template sidecar targeting the workflow, the processor MUST record a warning in provenance. The notification is not sent, but the workflow continues.

### 5.3 Rendering

Template rendering is implementation-defined. The processor MUST:

1. Resolve all placeholder variables from the case state, governance context, and runtime environment.
2. Evaluate section `condition` expressions (if present) using the standard FEL evaluation context (Kernel S7.3).
3. Verify all `requiredVariables` are present. If any are missing, record the failure in provenance and do not send the notification.
4. Include all sections where `required` is `true` (or defaulted to `true`) and whose `condition` evaluates to `true` or is absent.

---

## 6. Conformance

### 6.1 Processor Requirements

A processor that supports Notification Template sidecars:

1. MUST parse and validate the document against the Notification Template schema.
2. MUST resolve `notificationTemplateRef` and `noticeTemplateRef` references from governance documents to template keys in this sidecar.
3. MUST verify that `adverse-decision` templates include sections addressing the due process requirements in S4.4.
4. MUST resolve placeholder variables from the case state and governance context.
5. MUST record notification rendering failures in provenance.
6. MUST NOT send a notification when required variables are missing.

### 6.2 Absence Behavior

When no Notification Template sidecar targets a workflow, `notificationTemplateRef` and `noticeTemplateRef` values in governance documents are unresolvable. The processor SHOULD log a warning but MUST NOT treat this as a fatal error. Notification delivery falls back to implementation-defined behavior.

---

## References

### Normative References

- [WOS Kernel] Formspec Working Group, "WOS Kernel Specification v1.0".
- [WOS Governance] Formspec Working Group, "WOS Workflow Governance Specification v1.0".
- [Formspec Core] Formspec Working Group, "Formspec Core Specification v1.0".

### Informative References

- [APA] Administrative Procedure Act, 5 U.S.C. 554.
- [ECOA] Equal Credit Opportunity Act, Regulation B, 12 CFR 1002.
- [OMB M-24-10] Office of Management and Budget, Memorandum M-24-10, "Advancing Governance, Innovation, and Risk Management for Agency Use of Artificial Intelligence", 2024.
