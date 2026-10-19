---
title: WOS Agent Config
version: 1.0.0-draft.2
date: 2026-10-19
status: draft
---

> These settings are native properties of each `agents[]` declaration in a `$wosWorkflow`. Their owning schema is [`AgentDeclaration`](../../schemas/wos-workflow.schema.json#/$defs/AgentDeclaration). This specification defines operational behavior; it does not define a separate document or an alternate accepted format.

# WOS Agent Config v1.0

**Version:** 1.0.0-draft.2
**Date:** 2026-10-19
**Editors:** Formspec Working Group
**Embedded settings for:** WOS AI Integration Specification v1.0

---

## Abstract

This specification defines per-agent operational settings inside the Workflow: approved model versions, calibration requirements, autonomy limits and per-action restrictions. These settings participate in the Workflow's revision and verification identity. Deployment may rotate credentials and bind an invoker through external configuration, but that operation cannot silently change authored permissions, model-selection rules or review requirements.

---

## Status of This Document

This document is a **draft specification**. It elaborates the native agent fields used by the WOS AI Integration Specification v1.0. The behavior, composition and conformance rules in AI Integration §15 apply to these settings.

---

## 1. Document Structure

### 1.1 Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Native agent id, joined to an `actors[]` entry of kind `agent`. Defined once by AI Integration §3.1. |
| `invoker` | InvokerSpec | OPTIONAL | Adapter selection and binding described in §1.2. Required for an execution that invokes this agent. |
| `approvedVersions` | array of string | OPTIONAL | Approved model version list for `approved` model version policy. |
| `calibration` | CalibrationConfig | OPTIONAL | Calibration requirements and schedule. |
| `autonomyPolicy` | AutonomyPolicy | OPTIONAL | Escalation and demotion rules for dynamic autonomy. |
| `actionOverrides` | array of ActionOverride | OPTIONAL | Per-action autonomy and constraint overrides. |

These properties occur directly on the agent declaration, alongside the identity, capability and policy fields defined by AI Integration §3.1. `$wosAgentConfig`, `targetAgent`, `targetIntegration`, an independent `version`, and a nested copy of the declaration are invalid. Additional descriptive metadata uses `x-*` properties and conveys no runtime authority.

### 1.2 Endpoint Configuration

`agents[].invoker` selects the maintained adapter and its declared native binding fields. Adapter-specific deployment settings, such as credentials, timeouts and health checks, resolve through that adapter's supported configuration reference. The adapter receives the admitted agent's `modelIdentifier`, `modelVersion` and model-selection policy. There is no second authored `invoker.model` selector: any provider-specific mapping belongs to the maintained adapter configuration/resolution operation and must verify the selected model against that admitted identity. A missing or unverifiable mapping is unsupported execution. There is no second `endpoint` object with potentially conflicting connection settings. Binding validation MUST fail if a required adapter or configuration reference is missing. It MUST NOT select an unrelated adapter or broaden the agent's permissions.

### 1.3 Calibration Configuration

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `required` | boolean | REQUIRED | Whether calibration is required for this agent. |
| `frequency` | string (positive duration) | CONDITIONAL | Re-calibration frequency. Required when `required` is `true`. |
| `minimumSamples` | positive integer | OPTIONAL | Minimum reviewed outputs for valid calibration. |
| `method` | enum | OPTIONAL | `plattScaling`, `isotonic`, `binning`, or `custom`. Default: `binning`. |

When required calibration is missing or has expired (the frequency period has elapsed without re-calibration), the agent's effective autonomy MUST be capped at `assistive`. Calibration evidence, completion time and pending state belong to the maintained calibration operation and use the processor's clock. An event or model output cannot assert calibration success. If that operation is unavailable, required calibration is unsupported execution, not an assumed fresh result.

### 1.4 Autonomy Policy

Autonomy policies define conditions under which processor-owned current autonomy may be restored or reduced. Authored limits and operational state compose exactly as AI Integration §5.3; an approval cannot raise an authored ceiling.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `escalation` | array of EscalationRule | OPTIONAL | Conditions under which autonomy may be elevated. |
| `demotion` | array of DemotionRule | OPTIONAL | Conditions under which autonomy is automatically reduced. |
| `maxAutonomy` | enum | OPTIONAL | `manual`, `assistive`, `supervisory`, or `autonomous`; an upper limit composed with workflow, action and impact limits. |

#### 1.4.1 Demotion Rule

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Stable identifier, unique within the parent `demotion` array. Referenced by Drift Monitor `AlertThreshold.policyRef` (see WOS Drift Monitor §1.4.1). |
| `condition` | string | REQUIRED | FEL expression evaluated against agent operational state and case data. |
| `targetLevel` | enum | REQUIRED | `supervisory`, `assistive`, or `manual`. |
| `pendingRecalibration` | boolean | OPTIONAL | When `true`, the reduction persists until the maintained calibration operation records successful recalibration. Clearing it does not itself elevate the retained current level. |
| `description` | string | OPTIONAL | Human-readable rationale included in audit records. |

A Drift Monitor `AlertThreshold` MAY reference a DemotionRule by `id` via `policyRef`. The reference resolves only within the enclosing agent's `autonomyPolicy.demotion` in the same Workflow revision. Both the alert and demotion conditions must hold before that rule applies. Missing or ambiguous references are errors. The `autonomyDemotion` record identifies the agent, Workflow revision and exact rule occurrence as well as its readable id. Drift Monitor §1.4 defines alert handling.

#### 1.4.2 Escalation Rule

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique within this agent's escalation rules. |
| `condition` | string (FEL) | REQUIRED | Boolean condition evaluated against processor-owned operational state and admitted case data. |
| `targetLevel` | enum | REQUIRED | `assistive`, `supervisory` or `autonomous`; cannot exceed the authored ceiling or an active reduction. |
| `requiredApproverRef` | string | REQUIRED | Native human actor id whose authenticated binding may approve. Must resolve to an `actors[]` entry of type `human`; its descriptive `role` is not an authority grant. |
| `escalationExpiry` | string (duration) | REQUIRED | Positive ISO 8601 duration for the approval. |
| `description` | string | OPTIONAL | Rationale retained with the authorization decision. |

An approval binds the exact rule, agent and Workflow revision, plus the authenticated human and selected actor slot. The maintained actor-binding and authorization operation verifies that human may act for `requiredApproverRef`; string equality against event or model claims is insufficient. AI Integration §5.4 defines expiry and authorization. A processor lacking those stateful operations MUST report unsupported execution; it may not accept event fields as an approval substitute.

### 1.5 Action Overrides

Per-action autonomy and constraint overrides for specific workflow actions.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `actionRef` | JSON Pointer | REQUIRED | Exact locator of the lifecycle action in this Workflow revision. Unique within this agent's overrides. |
| `autonomy` | enum | OPTIONAL | Override autonomy level for this action. |
| `reviewWindow` | string (duration) | OPTIONAL | Override review window for supervisory actions. |
| `confidenceFloor` | ConfidenceFloor | OPTIONAL | Additional threshold and effect, using the same object as workflow and agent scopes (AI Integration §7.4). |
| `deonticConstraints` | array of DeonticConstraint | OPTIONAL | Additional tagged constraints using AI Integration §4; they cannot replace parent rules. |

The executing action selects its override through its admitted native locator. A claimed action id inside model output or an untrusted event is not this selection. Unresolvable locators, duplicate overrides and unsupported required action policy are errors. Each `confidenceFloor` adds its own threshold and effect; all applicable floors remain active and combine under AI Integration §7.4. `autonomy` can restrict effective autonomy, never raise an applicable cap. Omission adds no override. Review windows and dynamic escalation or demotion require the maintained review/timer operations; absence of those operations is reported as unsupported execution rather than ignored policy.
