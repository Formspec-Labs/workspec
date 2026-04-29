---
title: WOS Due Process Config
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

> **Absorbed (ADR 0076 D-1).** Due Process Config content lives at [`Governance.dueProcess`](../../schemas/wos-workflow.schema.json) inside the merged `$wosWorkflow` envelope (`#/$defs/Governance` → `dueProcess` → `#/$defs/DueProcess`). Standalone `wos-due-process-config` schema and `$wosDueProcessConfig` marker are retired; the `targetGovernance` reference pattern collapses to "the workflow's own URL" since governance and due process now share one document. Prose below remains as normative reference for due-process semantics.

# WOS Due Process Config v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Sidecar to:** WOS Workflow Governance Specification v1.0

---

## Abstract

The WOS Due Process Config is a sidecar document that provides detailed due process configuration for a WOS Workflow Governance Document. It separates operational due process parameters -- notice templates, grace periods, explanation templates, appeal routing rules, and continuation-of-service policies -- from the governance document's structural requirements. This separation allows due process parameters to be updated independently of the governance structure (e.g., changing a notice grace period from 30 to 45 days without modifying the governance document).

---

## Status of This Document

This document is a **draft specification**. It is a sidecar to the WOS Workflow Governance Specification v1.0 and does not modify governance processing semantics.

---

## 1. Document Structure

A Due Process Config targets a Workflow Governance Document via `targetGovernance`. It provides implementation-level detail for the due process requirements declared in the governance document.

### 1.1 Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `$wosDueProcess` | string | REQUIRED | Document type marker. MUST be `"1.0"`. |
| `targetGovernance` | string (URI) | REQUIRED | URI of the Workflow Governance Document this config targets. |
| `version` | string | OPTIONAL | Version of this config document. |
| `explanationTemplates` | array of ExplanationTemplate | OPTIONAL | Templates for decision explanations. |
| `appealRouting` | AppealRouting | OPTIONAL | Detailed appeal routing configuration. |
| `continuationPolicies` | array of ContinuationPolicy | OPTIONAL | Detailed continuation-of-service policies. |
| `extensions` | object | OPTIONAL | Extension data. Keys MUST be prefixed with `x-`. |

> **Notice templates live in the Notification Template sidecar, not here.** This config previously carried a thin `noticeTemplates` array of `{id, title, sections}` records, but the rich `TemplateSection`-based shape in the [WOS Notification Template Config](../sidecars/notification-template.md) sidecar is the canonical authoring surface for notices. The thin shape was removed to eliminate the divergent surface; `noticeTemplateKey` (Governance §3.1) and `notificationTemplateKey` (Governance §12.2) both resolve through the Notification Template sidecar (see lint rule G-063).

### 1.3 Appeal Routing

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `defaultReviewerPool` | string | OPTIONAL | Default pool of eligible appeal reviewers. |
| `independenceConstraint` | string | REQUIRED | How independence from the original determination is ensured. |
| `escalationPath` | array of string | OPTIONAL | Ordered escalation path if initial appeal review is contested. |
