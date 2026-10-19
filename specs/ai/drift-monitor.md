---
title: WOS Drift Monitor Config
version: 1.0.0-draft.2
date: 2026-10-19
status: draft
---

> Drift monitoring is authored inside the native `$wosWorkflow`: per-agent monitoring uses `agents[].driftMonitoring`, while the undefined workflow-wide `aiOversight.driftDetection` setting is reserved and rejected at admission (AI Integration §9.3). The owning definitions are in [`wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json). There is no standalone monitor document or alternate accepted identity.

# WOS Drift Monitor Config v1.0

**Version:** 1.0.0-draft.2
**Date:** 2026-10-19
**Editors:** Formspec Working Group
**Embedded settings for:** WOS AI Integration Specification v1.0

---

## Abstract

This specification defines native agent monitoring parameters: evaluation windows, metrics, alert thresholds, review monitoring and deployment sequence settings. They share the enclosing Workflow's revision and verification identity. A monitoring change cannot silently change an admitted agent's autonomy or bypass its authored limits.

---

## Status of This Document

This document is a **draft specification**. It elaborates the native settings used by the WOS AI Integration Specification v1.0. AI Integration §15 defines their behavior, composition and conformance requirements.

---

## 1. Document Structure

The drift monitoring configuration is expressed as the `driftMonitoring` property on each `agents[*]` entry of a `$wosWorkflow` document. All properties described in this section appear under `agents[*].driftMonitoring`. The enclosing agent determines who is monitored; there is no second `agentRef`, `targetAgent`, `targetIntegration` or `targetWorkflow` selector. `$wosDriftMonitor` and standalone document identity fields are invalid.

### 1.1 Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `monitors` | array of AgentMonitor | REQUIRED | Per-agent monitoring configurations. |
| `deploymentSequence` | DeploymentSequence | OPTIONAL | Shadow/canary/production deployment sequence policy. |
| `extensions` | object | OPTIONAL | Extension data. Keys MUST be prefixed with `x-`. |

### 1.2 Agent Monitor

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `evaluationWindow` | string (duration) | REQUIRED | ISO 8601 duration of the evaluation window. |
| `metrics` | array of MonitorMetric | REQUIRED | Metrics to track. |
| `rubberStamp` | RubberStampConfig | OPTIONAL | Rubber-stamp detection configuration. |
| `alertThresholds` | array of AlertThreshold | OPTIONAL | Conditions that generate alerts. |

#### 1.2.1 Rubber-Stamp Configuration

`RubberStampConfig` is a closed object using the enclosing monitor's `evaluationWindow`. It has required boolean `enabled`, optional positive duration `minReviewTime`, and optional finite `maxAgreementRate` in [0,1]. When enabled, at least one threshold is required. A review below the minimum duration or an aggregate agreement rate strictly above the maximum is a monitoring signal, not proof of reviewer misconduct or authority to change an outcome. The maintained review-measurement operation supplies the observations. A processor without that operation must report enabled monitoring as unsupported; emitted sample data is not measured reviewer behavior.

### 1.3 Monitor Metrics

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | string | REQUIRED | Metric name (e.g., `agreementRate`, `confidenceDistribution`, `modificationRate`). |
| `method` | enum | OPTIONAL | Statistical method: `psi` (Population Stability Index), `ks` (Kolmogorov-Smirnov), `chi2`, `accuracy`, `custom`. |
| `threshold` | number | OPTIONAL | Threshold value for drift detection. |

### 1.4 Alert Threshold

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `condition` | string | REQUIRED | FEL expression that triggers the alert. |
| `severity` | enum | REQUIRED | `info`, `warning`, or `critical`. |
| `action` | enum | CONDITIONAL | `notify`, `demoteToAssistive`, `demoteToManual` or `suspend`, defined below. Required when `policyRef` is absent. |
| `policyRef` | string | CONDITIONAL | A `DemotionRule.id` in the enclosing agent's native `autonomyPolicy.demotion` (Agent Config §1.4). Excludes `action`. |
| `notifyRoles` | array of string | OPTIONAL | Roles to notify when this threshold is crossed. |

#### 1.4.1 `policyRef` resolution semantics

Exactly one of `policyRef` and `action` MUST be declared. When `policyRef` is present, processors MUST:

1. Select the enclosing native agent in the admitted Workflow revision.
2. Resolve exactly one rule with that `id` in the same agent's `autonomyPolicy.demotion`. An identically named rule belonging to another agent is not a match. Duplicate ids are configuration errors.
3. When the alert condition is true, evaluate the selected rule's condition against processor-owned operational state and admitted case data. Apply its `targetLevel` and `pendingRecalibration` behavior only if that condition is true. A false condition records that the alert fired without applying the demotion; null, non-boolean and evaluation failures are errors and prevent automatic invocation until resolved.
4. Record the agent, Workflow revision, exact native rule locator and readable rule id in the resulting `autonomyDemotion` provenance. Maintain the operational reduction under AI Integration §5.
5. Refuse invalid configuration or execution with an unresolved required reference. Do not fall back to another agent, a default action or a retired sidecar.

Without `policyRef`, `notify` records the alert and sends any declared role notifications through the maintained notification operation; it does not change autonomy. `demoteToAssistive` and `demoteToManual` add a processor-owned operational ceiling of the named level. `suspend` prevents further invocations of the enclosing agent until an authenticated operational action clears the suspension. These actions cannot increase effective autonomy or erase other active reductions. Required notification, demotion, suspension and later restoration operations must be supported; otherwise execution is explicitly unsupported. Emitting provenance alone does not execute the action.

### 1.5 Deployment Sequence

`DeploymentSequence` is a closed object with required boolean `enabled` and optional `shadowDuration`, `canaryPercentage` and `canaryDuration`. When enabled, both durations MUST be positive ISO 8601 durations supported by the maintained deployment/timer operations, and the finite canary percentage MUST be greater than zero and less than one (a fraction, not a number out of 100). These fields declare requirements; they do not perform deployment. Missing deployment, measurement or selection operations make enabled execution unsupported.

For `rights-impacting` and `safety-impacting` workflows, model version changes SHOULD follow a three-phase deployment sequence:

| Phase | Description |
|-------|-------------|
| `shadow` | New version runs in parallel. Outputs not used. Compared to production for drift analysis. |
| `canary` | New version handles a small percentage of invocations with strict guardrails and elevated review sampling. |
| `production` | New version replaces prior version after shadow and canary demonstrate acceptable performance. |

### 1.6 Example

The `agents[*].driftMonitoring` block embedded in a `$wosWorkflow` document. This example demonstrates configuration; it does not claim that an invoker, metric collector or demotion operation ran:

```json
{
  "$wosWorkflow": "1.0",
  "url": "https://example.gov/workflows/document-extraction",
  "version": "1.0.0",
  "title": "Document Extraction Drift Monitor",
  "impactLevel": "operational",
  "actors": [
    { "id": "documentExtractor", "type": "agent" }
  ],
  "lifecycle": {
    "initialState": "start",
    "states": {
      "start": { "type": "atomic" },
      "done": { "type": "final" }
    }
  },
  "agents": [
    {
      "id": "documentExtractor",
      "agentType": "statistical",
      "modelIdentifier": "document-extraction",
      "modelVersion": "1.0.0",
      "autonomyPolicy": {
        "demotion": [{
          "id": "demoteToAssistiveOnDegradedAccuracy",
          "condition": "agent.metrics.accuracy < 0.90",
          "targetLevel": "assistive",
          "pendingRecalibration": true
        }]
      },
      "driftMonitoring": {
        "monitors": [
          {
            "evaluationWindow": "P30D",
            "metrics": [
              { "name": "accuracy", "method": "accuracy", "threshold": 0.92 },
              { "name": "confidenceDistribution", "method": "psi", "threshold": 0.2 }
            ],
            "rubberStamp": {
              "enabled": true,
              "minReviewTime": "PT30S",
              "maxAgreementRate": 0.95
            },
            "alertThresholds": [
              {
                "condition": "accuracy < 0.90",
                "severity": "critical",
                "policyRef": "demoteToAssistiveOnDegradedAccuracy"
              }
            ]
          }
        ],
        "deploymentSequence": {
          "enabled": true,
          "shadowDuration": "P14D",
          "canaryPercentage": 0.05,
          "canaryDuration": "P7D"
        }
      }
    }
  ]
}
```
