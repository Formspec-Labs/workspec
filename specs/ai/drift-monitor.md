---
title: WOS Drift Monitor Config
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

> **Absorbed (ADR 0076 D-1).** Drift monitoring content lives in two slots of the merged `$wosWorkflow` envelope: per-agent monitoring at [`AgentDeclaration.driftMonitoring`](../../schemas/wos-workflow.schema.json) and workflow-wide oversight at [`AIOversight.driftDetection`](../../schemas/wos-workflow.schema.json). Standalone `wos-drift-monitor` schema and `$wosDriftMonitor` marker are retired; this prose remains as normative reference for drift detection semantics.

# WOS Drift Monitor Config v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Sidecar to:** WOS AI Integration Specification v1.0

---

## Abstract

The WOS Drift Monitor Config is a sidecar document that provides detailed drift detection and monitoring configuration for agents in a WOS workflow. It separates monitoring parameters -- drift detection methods, evaluation windows, alert thresholds, rubber-stamp monitoring, and deployment sequence policies -- from the AI Integration Document's governance declarations. This separation allows monitoring parameters to be tuned independently of governance structure.

---

## Status of This Document

This document is a **draft specification**. It is a sidecar to the WOS AI Integration Specification v1.0 and does not modify AI integration processing semantics.

---

## 1. Document Structure

The drift monitoring configuration is expressed as the `driftMonitoring` property on each `agents[*]` entry of a `$wosWorkflow` document. All properties described in this section appear under `agents[*].driftMonitoring`.

### 1.1 Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `monitors` | array of AgentMonitor | REQUIRED | Per-agent monitoring configurations. |
| `deploymentSequence` | DeploymentSequence | OPTIONAL | Shadow/canary/production deployment sequence policy. |
| `extensions` | object | OPTIONAL | Extension data. Keys MUST be prefixed with `x-`. |

### 1.2 Agent Monitor

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `agentRef` | string | REQUIRED | Agent identifier to monitor. |
| `evaluationWindow` | string (duration) | REQUIRED | ISO 8601 duration of the evaluation window. |
| `metrics` | array of MonitorMetric | REQUIRED | Metrics to track. |
| `rubberStamp` | RubberStampConfig | OPTIONAL | Rubber-stamp detection configuration. |
| `alertThresholds` | array of AlertThreshold | OPTIONAL | Conditions that generate alerts. |

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
| `action` | enum | OPTIONAL | `notify`, `demoteToAssistive`, `demoteToManual`, or `suspend`. Implementation-defined when no `policyRef` is given. |
| `policyRef` | string | OPTIONAL | Reference to a `DemotionRule.id` declared in an Agent Config sidecar (see WOS Agent Config §1.4). |
| `notifyRoles` | array of string | OPTIONAL | Roles to notify when this threshold is crossed. |

#### 1.4.1 `policyRef` resolution semantics

When `policyRef` is present, processors MUST resolve it as follows:

1. Locate the Agent Config sidecar whose `targetAgent` matches the parent `AgentMonitor.agentRef` and whose `targetIntegration` (or co-located AI Integration Document) targets the same workflow as this monitor's `targetWorkflow`.
2. Find the `DemotionRule` whose `id` equals `policyRef` within that sidecar's `autonomyPolicy.demotion` array.
3. When the alert fires, the named DemotionRule's structured semantics (`condition` / `targetLevel` / `pendingRecalibration` / `description`) take precedence over the `action` enum. The provenance record (`autonomyDemotion`, see AI Integration §5.5) MUST cite the resolved `DemotionRule.id` so audit consumers know exactly which named rule fired, not merely that *some* demotion occurred.
4. If the `policyRef` cannot be resolved (no matching sidecar or no matching `DemotionRule.id`), processors MUST emit a configuration warning in provenance and fall back to the implementation-defined `action` enum behavior. An unresolvable `policyRef` is a misconfiguration, not a runtime error — the alert still fires.

When `policyRef` is absent, the existing `action` enum behavior (`notify` / `demoteToAssistive` / `demoteToManual` / `suspend`) is implementation-defined as before.

### 1.5 Deployment Sequence

For `rights-impacting` and `safety-impacting` workflows, model version changes SHOULD follow a three-phase deployment sequence:

| Phase | Description |
|-------|-------------|
| `shadow` | New version runs in parallel. Outputs not used. Compared to production for drift analysis. |
| `canary` | New version handles a small percentage of invocations with strict guardrails and elevated review sampling. |
| `production` | New version replaces prior version after shadow and canary demonstrate acceptable performance. |

### 1.6 Example

The `agents[*].driftMonitoring` block embedded in a `$wosWorkflow` document:

```json
{
  "$wosWorkflow": "1.0",
  "url": "https://example.gov/workflows/document-extraction",
  "version": "1.0.0",
  "title": "Document Extraction Drift Monitor",
  "impactLevel": "operational",
  "actors": [
    { "id": "system", "type": "system" }
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
      "driftMonitoring": {
        "monitors": [
          {
            "agentRef": "documentExtractor",
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
                "action": "demoteToAssistive",
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
