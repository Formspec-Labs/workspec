---
title: WOS Drift Monitor Config
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

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

### 1.1 Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `$wosDriftMonitor` | string | REQUIRED | Document type marker. MUST be `"1.0"`. |
| `targetWorkflow` | string (URI) | REQUIRED | URI of the WOS Kernel Document this monitor applies to. |
| `version` | string | OPTIONAL | Version of this config document. |
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

### 1.4 Deployment Sequence

For `rights-impacting` and `safety-impacting` workflows, model version changes SHOULD follow a three-phase deployment sequence:

| Phase | Description |
|-------|-------------|
| `shadow` | New version runs in parallel. Outputs not used. Compared to production for drift analysis. |
| `canary` | New version handles a small percentage of invocations with strict guardrails and elevated review sampling. |
| `production` | New version replaces prior version after shadow and canary demonstrate acceptable performance. |

### 1.5 Example

```json
{
  "$wosDriftMonitor": "1.0",
  "targetWorkflow": "https://agency.gov/workflows/benefits-adjudication",
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
          "action": "demoteToAssistive"
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
```
