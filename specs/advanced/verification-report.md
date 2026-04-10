---
title: WOS Verification Report
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS Verification Report v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Sidecar to:** WOS Advanced Governance Specification v1.0

---

## Abstract

The WOS Verification Report is a sidecar document that records the results of formal verification (SMT) of deontic constraints and governance rules. Each report captures which constraints were verified, the verification method, the result (proven-safe, proven-unsafe, inconclusive), any counterexamples, solver metadata, and the verification timestamp. Reports are immutable provenance artifacts.

---

## Status of This Document

This document is a **draft specification**. It is a sidecar to the WOS Advanced Governance Specification v1.0 and does not modify governance processing semantics.

---

## 1. Document Structure

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `$wosVerificationReport` | string | REQUIRED | Document type marker. MUST be `"1.0"`. |
| `targetWorkflow` | string (URI) | REQUIRED | URI of the WOS Kernel Document whose constraints were verified. |
| `version` | string | OPTIONAL | Version of this report. |
| `timestamp` | string (datetime) | REQUIRED | When the verification was performed. |
| `solver` | SolverInfo | REQUIRED | Information about the verification tool. |
| `results` | array of ConstraintResult | REQUIRED | Per-constraint verification results. |
| `summary` | VerificationSummary | OPTIONAL | Aggregate summary. |
| `extensions` | object | OPTIONAL | Extension data. Keys MUST be prefixed with `x-`. |
