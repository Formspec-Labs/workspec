---
title: WOS Agent Config
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

> **Absorbed (ADR 0076 D-1).** Agent Config content lives on each per-agent declaration inside the merged `$wosWorkflow` envelope. Schema home: [`schemas/wos-workflow.schema.json#/$defs/AgentDeclaration`](../../schemas/wos-workflow.schema.json) — `modelIdentifier`, `modelVersion`, `modelVersionPolicy`, `autonomy`, `confidenceFloor`, `confidenceDecay`, `fallbackChain`, `capabilities`, `driftMonitoring` properties. Standalone `wos-agent-config` schema and `$wosAgentConfig` marker are retired; this prose remains as normative reference for per-agent operational semantics.

# WOS Agent Config v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Sidecar to:** WOS AI Integration Specification v1.0

---

## Abstract

The WOS Agent Config is a sidecar document that provides detailed operational configuration for an agent declared in a WOS AI Integration Document. It separates operational parameters -- endpoint configuration, credential references, approved model version lists, calibration requirements, autonomy escalation and demotion policies, and per-action overrides -- from the integration document's governance declarations. This separation allows operational parameters to be updated independently (e.g., rotating credentials, approving a new model version) without modifying the governance document.

---

## Status of This Document

This document is a **draft specification**. It is a sidecar to the WOS AI Integration Specification v1.0 and does not modify AI integration processing semantics.

---

## 1. Document Structure

### 1.1 Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `$wosAgentConfig` | string | REQUIRED | Document type marker. MUST be `"1.0"`. |
| `targetAgent` | string | REQUIRED | Agent identifier within the AI Integration Document this config targets. |
| `targetIntegration` | string (URI) | OPTIONAL | URI of the AI Integration Document, if not co-located. |
| `version` | string | OPTIONAL | Version of this config document. |
| `endpoint` | EndpointConfig | OPTIONAL | Agent endpoint configuration. |
| `approvedVersions` | array of string | OPTIONAL | Approved model version list for `approved` model version policy. |
| `calibration` | CalibrationConfig | OPTIONAL | Calibration requirements and schedule. |
| `autonomyPolicy` | AutonomyPolicy | OPTIONAL | Escalation and demotion rules for dynamic autonomy. |
| `actionOverrides` | array of ActionOverride | OPTIONAL | Per-action autonomy and constraint overrides. |
| `extensions` | object | OPTIONAL | Extension data. Keys MUST be prefixed with `x-`. |

### 1.2 Endpoint Configuration

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `url` | string (URI) | REQUIRED | Agent service endpoint. |
| `credentialRef` | string | OPTIONAL | Reference to credential store entry. |
| `timeout` | string (duration) | OPTIONAL | Maximum invocation time (ISO 8601). |
| `healthCheckUrl` | string (URI) | OPTIONAL | Health check endpoint for availability monitoring. |

### 1.3 Calibration Configuration

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `required` | boolean | REQUIRED | Whether calibration is required for this agent. |
| `frequency` | string (duration) | CONDITIONAL | Re-calibration frequency. Required when `required` is `true`. |
| `minimumSamples` | integer | OPTIONAL | Minimum reviewed outputs for valid calibration. |
| `method` | enum | OPTIONAL | `plattScaling`, `isotonic`, `binning`, or `custom`. Default: `binning`. |

When calibration has expired (the frequency period has elapsed without re-calibration), the agent's effective autonomy MUST be capped at `assistive` regardless of its configured level.

### 1.4 Autonomy Policy

Autonomy policies define conditions under which the agent's autonomy level is dynamically escalated or demoted.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `escalation` | array of EscalationRule | OPTIONAL | Conditions under which autonomy may be elevated. |
| `demotion` | array of DemotionRule | OPTIONAL | Conditions under which autonomy is automatically reduced. |
| `maxAutonomy` | enum | OPTIONAL | Maximum autonomy level this agent may reach. |

#### 1.4.1 Demotion Rule

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Stable identifier, unique within the parent `demotion` array. Referenced by Drift Monitor `AlertThreshold.policyRef` (see WOS Drift Monitor §1.4.1). |
| `condition` | string | REQUIRED | FEL expression evaluated against agent operational state and case data. |
| `targetLevel` | enum | REQUIRED | `supervisory`, `assistive`, or `manual`. |
| `pendingRecalibration` | boolean | OPTIONAL | When `true`, agent stays demoted until recalibration meets escalation conditions. |
| `description` | string | OPTIONAL | Human-readable rationale included in audit records. |

A Drift Monitor `AlertThreshold` MAY reference a DemotionRule by `id` via `policyRef`. When it does, the named rule's structured semantics fire instead of the implementation-defined `action` enum, and the resolved `id` appears in the `autonomyDemotion` provenance record.

### 1.5 Action Overrides

Per-action autonomy and constraint overrides for specific workflow actions.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `actionRef` | string | REQUIRED | Reference to the action in the kernel lifecycle. |
| `autonomy` | enum | OPTIONAL | Override autonomy level for this action. |
| `reviewWindow` | string (duration) | OPTIONAL | Override review window for supervisory actions. |
| `confidenceFloor` | number (0-1) | OPTIONAL | Override confidence floor for this action. |
| `deonticConstraints` | DeonticConstraints | OPTIONAL | Additional deontic constraints specific to this action. |
