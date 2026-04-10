---
title: WOS Equity Config
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS Equity Config v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Sidecar to:** WOS Advanced Governance Specification v1.0

---

## Abstract

The WOS Equity Config is a sidecar document that provides detailed equity monitoring configuration for a WOS workflow. It separates operational equity parameters -- protected category definitions, disparity calculation methods, reporting schedules, remediation triggers -- from the Advanced Governance Document's structural equity guardrail declarations. This separation allows equity monitoring parameters to be updated independently of the governance structure.

Equity monitoring applies to human AND AI decisions. It is a civil rights concern, not an AI-specific concern.

---

## Status of This Document

This document is a **draft specification**. It is a sidecar to the WOS Advanced Governance Specification v1.0 and does not modify governance processing semantics.

---

## 1. Document Structure

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `$wosEquityConfig` | string | REQUIRED | Document type marker. MUST be `"1.0"`. |
| `targetWorkflow` | string (URI) | REQUIRED | URI of the WOS Kernel Document this config targets. |
| `version` | string | OPTIONAL | Version of this config document. |
| `protectedCategories` | array of ProtectedCategory | OPTIONAL | Definitions of demographic or categorical groupings for monitoring. |
| `disparityMethods` | array of DisparityMethod | OPTIONAL | Statistical methods for disparity calculation. |
| `reportingSchedule` | ReportingSchedule | OPTIONAL | Automated reporting configuration. |
| `remediationTriggers` | array of RemediationTrigger | OPTIONAL | Conditions that trigger structured remediation review. |
| `extensions` | object | OPTIONAL | Extension data. Keys MUST be prefixed with `x-`. |
