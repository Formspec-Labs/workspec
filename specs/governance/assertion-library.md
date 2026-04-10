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
