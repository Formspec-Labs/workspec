# Studio Spec: Binding and Integration

**Status:** draft (Stage 2 of [Implementation Roadmap](../VISION.md#17-implementation-roadmap))
**Date:** 2026-04-30
**Concept-model anchor:** [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.20–1.23 (binding entities).
**PRD anchor:** [`../VISION.md`](../VISION.md) §9.4 (Workflow Builder), §6 (Mapping Contract). Companion PRD §3, §5 (binding capabilities).
**Depends on:** [`policy-object-model.md`](policy-object-model.md), [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md), [`workflow-intent.md`](workflow-intent.md), [`compiler-contract.md`](compiler-contract.md).

## Scope

This spec defines the Studio surface for **integrating workflows with real systems**: API calls, event channels, external policy engines, and structured decision logic. WOS already has the integration profile (OpenAPI / Arazzo / CloudEvents / OPA / Cedar / XACML are native bindings per `WOS-FEATURE-MATRIX.md` §12). Studio's job is to give non-technical reviewers a way to author and review these bindings without reading WOS JSON.

This spec defines four PolicyObject kinds:

- `ServiceBinding` — workflow step ↔ OpenAPI operation or Arazzo step.
- `EventBinding` — workflow event ↔ kernel event with CloudEvents extension attributes.
- `PolicyEngineBinding` — workflow check ↔ external OPA / Cedar / XACML decision.
- `DecisionTable` — multi-row decision logic that compiles to a chained-FEL-guard sequence (an extension to the existing `DecisionRule` kind).

Each binding declares which of the **six canonical kernel seams** it attaches through (per [`../../CLAUDE.md`](../../CLAUDE.md): `actorExtension`, `contractHook`, `provenanceLayer`, `lifecycleHook`, `custodyHook`, `extensions`/`x-`).

## Out of scope

- **DMN export.** Per audit findings: WOS rejects DMN as an expression-language authority (`CLAUDE.md:76`). DecisionTable compiles to chained FEL guards; it does NOT emit DMN. One-way DMN import is parked (`TODO.md:368`) until a customer asks.
- **OpenTelemetry telemetry.** Per `CLAUDE.md:103` "Audit ⊥ observability": OTel is the operator channel; Studio is the audit channel. This spec has no OTel binding.
- **AsyncAPI.** Superseded by CloudEvents in WOS 1.0; not adopted here.
- **The runtime adapter implementations** (Restate, Temporal, Camunda) — these consume the artifact; Studio produces it.
- **API discovery / spec import** — bringing in an OpenAPI document is workspace tooling; this spec defines the binding surface, not the import UX.

## Terminology

- **Binding** — a Studio object that connects a workflow element to an external system or capability.
- **Integration profile** — the WOS-side surface (`$wosWorkflow.integration` or transition-attached binding refs) that a binding compiles to. Specified in `WOS-FEATURE-MATRIX.md` §12 and referenced by Studio without redefinition.
- **Seam** — one of the six canonical kernel seams (ADR-0077). Every binding attaches at exactly one seam.
- **Hit policy** (DecisionTable specific) — the rule that determines which row's outcome wins when multiple rows match. Studio adopts a small set: `first-match`, `priority`, `unique`, `output-merge`. (Note: this is *not* DMN's hit policy enum verbatim; it is a Studio-native construct that expresses similar semantics.)
- **Restrictor** — terminology from `specs/ai/ai-integration.md §4.6`: an external policy decision composes restrictively (deny overrides permit). Used in PolicyEngineBinding semantics.

## Data model

### `ServiceBinding`

```text
ServiceBinding {
  id, kind: "ServiceBinding",
  body: {
    operationRef,            // OpenAPI operationId | Arazzo stepId | custom binding ref
    operationKind,           // openapi | arazzo | custom
    apiSpecRef,              // pointer to the imported API spec
    inputBindings[],         // [{caseFilePath, requestPath, transform?}]
    outputBindings[],        // [{responsePath, target, targetKind, transform?}]
                             //   targetKind: caseFile-update | decision-input | event-emission | validation-finding
    errorHandling,           // {onError: retry|fallback|fail-workflow|alert, retryPolicy?, fallbackBindingRef?}
    sensitivityHandling,     // when inputs/outputs include sensitive DataElements: redaction rules + retention
    sequencePosition?        // when part of an Arazzo sequence: the step's position
  },
  citations[], provenance, mappingState (always mapsToWos), lifecycleState,
  workspaceId, version
}
```

A `ServiceBinding` attaches to a WorkflowIntent step via [`workflow-intent.md`](workflow-intent.md) — specifically to elements of kind `system-check`, `data-collection`, `evidence-request`, or `notice` (when the notice is delivered via API).

### `EventBinding`

```text
EventBinding {
  id, kind: "EventBinding",
  body: {
    eventName,               // e.g., "application.submitted"
    direction,               // consumed | emitted
    payloadShape[],          // [{fieldName, fieldType, sensitivity}]
    cloudEventsExtensions: { // load-bearing for WOS interop
      woscausationeventid,   // causal chain
      woscorrelationkey      // case correlation
    },
    channel?,                // when known: routing channel id
    bindsTo {                // workflow attachment
      kind: trigger | transition | action,
      ref                    // workflow element id
    }
  },
  citations[], provenance, mappingState (mapsToWos), lifecycleState,
  workspaceId, version
}
```

Event names follow a dotted convention (`subject.verb` or `subject.verb.subject2`). The CloudEvents extension attributes are required because WOS uses them for case correlation and causal chains (`COMPLETED.md` NB.3).

### `PolicyEngineBinding`

```text
PolicyEngineBinding {
  id, kind: "PolicyEngineBinding",
  body: {
    engineKind,              // opa | cedar | xacml | custom
    engineEndpointRef,       // pointer to the engine's invocation endpoint
    policyRef,               // engine-side policy identifier
    inputContract: {
      caseFilePaths[],       // which case fields the engine evaluates
      additionalContext?     // any non-case context required (e.g., role, time, jurisdiction)
    },
    outputNormalization: {   // engine output → standard {decision, reasons, obligations}
      decisionMapping,       // permit/deny/not-applicable
      reasonsMapping,        // structured reason codes → plain language
      obligationsMapping     // engine-emitted obligations → workflow follow-ups
    },
    composition: "deny-overrides" // restrictor semantics per ai-integration.md §4.6
  },
  citations[], provenance, mappingState (mapsToWos), lifecycleState,
  workspaceId, version
}
```

A `PolicyEngineBinding` attaches at a transition guard or output-validation boundary. Its decision composes restrictively — engine deny overrides Studio-side permits.

### `DecisionTable` (extension to `DecisionRule`)

A `DecisionTable` is a structured form of a `DecisionRule` (see [`policy-object-model.md`](policy-object-model.md)) for multi-row authoring. The kind discriminator is `kind: "DecisionRule", body.form: "table"`.

```text
DecisionTable (DecisionRule with form = "table") {
  body: {
    form: "table",
    inputs[],                // [{name, dataElementRef, dataType}]
    outputs[],               // [{name, target, targetKind}]
    rows[],                  // each row carries an input pattern + outputs + sourceCitation
    hitPolicy,               // first-match | priority | unique | output-merge
    completenessRequirement, // all-inputs-covered | partial-allowed-with-default
    fallback?                // when no row matches: outcome or escalate-to-review
  }
}
```

Each row is reviewer-reviewable independently; a row's `sourceCitation` is preserved through the whole lifecycle. Compilation produces a chained-FEL-guard sequence (see [`compiler-contract.md`](compiler-contract.md)). **The table is NOT emitted as DMN.**

## Lifecycle

All four kinds follow the standard PolicyObject lifecycle from [`policy-object-model.md`](policy-object-model.md) §"Lifecycle":

```text
draft → reviewed → approved → mapsToWos → validated → published → superseded
```

`ServiceBinding` and `EventBinding` and `PolicyEngineBinding` are always `mapsToWos` (never authoring-only). `DecisionTable` follows the same lifecycle as ordinary `DecisionRule` — usually `mapsToWos`.

## Normative Contract

### Seam attachment

- **`SA-MUST-bind-001`** — Every Binding (Service / Event / PolicyEngine) MUST declare its kernel seam: one of `actorExtension | contractHook | provenanceLayer | lifecycleHook | custodyHook | extensions`. The default mapping by kind:
  - `ServiceBinding` → `contractHook` (validates/transforms case mutations during external service interaction).
  - `EventBinding (consumed)` → `lifecycleHook` (reacts to incoming events).
  - `EventBinding (emitted)` → `lifecycleHook` (emits on state transition).
  - `PolicyEngineBinding` → `contractHook` (decision injection at guard boundary).
  Bindings that need an `x-` extension MUST also carry a `specs/registry/extension-registry.md` entry per [`studio-to-wos-mapping.md`](studio-to-wos-mapping.md) `SA-MUST-map-014`. *(schema-pending.)*
- **`SA-MUST-bind-002`** — Bindings MUST NOT invent new seams. The six canonical seams are the closed set per ADR-0077. *(lint-pending: tier-S3 readiness rule.)*

### ServiceBinding integrity

- **`SA-MUST-bind-010`** — A `ServiceBinding`'s `inputBindings[]` MUST cover every required input of the referenced API operation. Missing inputs MUST be flagged as tier-S3 ValidationFindings (`BIND-LINT-001`). *(lint-pending: requires API-spec parsing.)*
- **`SA-MUST-bind-011`** — Every `inputBindings[].caseFilePath` MUST resolve to a `CaseFileMapping` in the workspace. Dangling case-file paths MUST be flagged as tier-S3 ValidationFindings. *(lint-pending.)*
- **`SA-MUST-bind-012`** — Every `outputBindings[].target` MUST resolve to a workspace object (CaseFileMapping, DecisionRule input, EventBinding emission, or ValidationFinding subject). Outputs that don't resolve MUST be flagged as `output-ignored-without-rationale`. *(lint-pending.)*
- **`SA-MUST-bind-013`** — A `ServiceBinding` whose `inputBindings` reference DataElements with `sensitivity ∈ {pii, phi, restricted}` MUST carry `sensitivityHandling` (redaction rules + retention) — or a documented waiver. *(lint-pending: tier-S3 cross-cutting with `SA-MUST-pom-037`.)*
- **`SA-MUST-bind-014`** — A `ServiceBinding` MUST declare `errorHandling.onError` (one of `retry | fallback | fail-workflow | alert`). Bindings without explicit error handling MUST be flagged as tier-S3 ValidationFindings (`BIND-LINT-005`). *(lint-pending.)*
- **`SA-MUST-bind-015`** — When `operationKind = arazzo`, the binding's `sequencePosition` MUST be set, and the workflow MUST tolerate pause/resume across the sequence boundary (per `MD-INVENTORY.md` Arazzo binding spec). *(runtime-pending.)*

### EventBinding integrity

- **`SA-MUST-bind-020`** — Every `EventBinding` MUST carry `cloudEventsExtensions.woscausationeventid` and `woscorrelationkey`. These are not optional — WOS uses them for case correlation. *(schema-pending.)*
- **`SA-MUST-bind-021`** — When `direction = consumed`, the binding MUST identify a source/system that emits the event. Consumed events without a source MUST be flagged as tier-S3 ValidationFindings (`BIND-LINT-010`). *(lint-pending.)*
- **`SA-MUST-bind-022`** — When `direction = emitted`, the binding MUST identify a recipient/channel. Emitted events without a recipient MUST be flagged as tier-S3 ValidationFindings (`BIND-LINT-011`). *(lint-pending.)*
- **`SA-MUST-bind-023`** — Workflow transitions that reference an event name (per `workflow-intent.md`) MUST have a corresponding `EventBinding` in the workspace. Workflows referencing undefined events MUST be flagged as tier-S4 ValidationFindings (`WF-LINT-011`). *(lint-pending.)*
- **`SA-MUST-bind-024`** — Every `payloadShape` field of `sensitivity ∈ {pii, phi, restricted}` MUST appear with a redaction rule when emitted (preventing PII leakage in event channels). *(lint-pending.)*

### PolicyEngineBinding integrity

- **`SA-MUST-bind-030`** — Every `PolicyEngineBinding` MUST normalize the engine's response into `{decision, reasons[], obligations[]}` per the WOS PolicyDecision contract (`WOS-IMPLEMENTATION-STATUS.md`). Raw engine responses MUST NOT bleed into the workflow. *(schema-pending.)*
- **`SA-MUST-bind-031`** — `composition` MUST be `deny-overrides` (the only allowed value in v1; per `specs/ai/ai-integration.md §4.6`). Other composition modes are deferred. *(schema-pending: enum.)*
- **`SA-MUST-bind-032`** — The binding's `inputContract.caseFilePaths[]` MUST be a complete declaration of every case field the engine reads. Engines that read undeclared fields create privacy/audit hazards; this declaration is what makes the binding auditable. *(lint-pending.)*
- **`SA-MUST-bind-033`** — `outputNormalization.reasonsMapping` MUST translate engine reason codes into reviewer-readable plain language. Bindings with unmapped reason codes MUST be flagged as tier-S3 ValidationFindings. *(lint-pending.)*

### DecisionTable integrity

- **`SA-MUST-bind-040`** — Every `DecisionTable` MUST declare `hitPolicy` and `completenessRequirement`. Tables without explicit hit policy or completeness MUST be flagged as tier-S2 ValidationFindings (`POM-LINT-009`). *(schema-pending.)*
- **`SA-MUST-bind-041`** — Every row MUST carry at least one `sourceCitation` or be backed by an approved `Assumption`. Rows without source backing are unsupported decision logic. *(lint-pending: tier-S2 cross-cutting with `SA-MUST-pom-004`.)*
- **`SA-MUST-bind-042`** — When `hitPolicy = unique`, the implementation MUST verify that at most one row matches any given input. Detected overlaps without explicit hit policy MUST be flagged as tier-S2 ValidationFindings (`POM-LINT-010`). *(lint-pending: requires symbolic input-range analysis.)*
- **`SA-MUST-bind-043`** — When `completenessRequirement = all-inputs-covered`, the implementation MUST verify that every expected input combination matches at least one row OR a fallback is declared. Detected gaps MUST be flagged as tier-S2 ValidationFindings (`POM-LINT-011`). *(lint-pending.)*
- **`SA-MUST-bind-044`** — DecisionTables compiling to chained-FEL-guard sequences MUST emit FEL expressions that match the table's hit-policy semantics deterministically. *(runtime-pending: compiler in [`compiler-contract.md`](compiler-contract.md).)*

### Decision tables and adverse outcomes

- **`SA-MUST-bind-050`** — A DecisionTable whose outputs include an Outcome with `polarity = adverse` AND `triggersDueProcess = true` MUST satisfy the same notice/appeal linkage rules as any other DecisionRule. (Cross-cutting `SA-MUST-pom-030`.) *(lint-pending.)*

## Composition

### Attachment point

Bindings attach at the workspace level (PolicyObjects per `policy-object-model.md`) and reference workflow elements (per `workflow-intent.md`). They are read by:

- The Studio→WOS compiler (Stage 5; per [`compiler-contract.md`](compiler-contract.md)) — to emit integration-profile entries in the artifact.
- The readiness engine — to fire tier-S3 / S4 binding-related findings.
- The Validation Center — to surface binding-coverage status.
- Scenarios — to exercise binding paths.

### Precedence

When a workflow step has multiple potential bindings (e.g., a step that could be either a ServiceBinding or a manual TaskMapping), reviewer judgment governs. Studio does not auto-pick. The companion PRD §4 AI flow recommends; humans approve.

When multiple ServiceBindings target the same workflow step (rare; usually a Studio mistake), the spec rejects at validation: `BIND-LINT-002` `multiple-bindings-on-step`.

When a PolicyEngineBinding's denial conflicts with a Studio-side DecisionRule's permit, the engine wins (`deny-overrides`). The conflict is logged, not suppressed.

### Conflict handling

Two ServiceBindings declaring conflicting `inputBindings` for the same `caseFilePath → requestPath` (different transforms) is a tier-S3 collision. The spec rejects.

A DecisionTable whose `hitPolicy = unique` matching multiple rows on the same input is a tier-S2 finding (`POM-LINT-010`); the table MUST be edited or hit-policy MUST be changed.

### Versioning / migration

- Adding a new binding kind (e.g., a future `MessageQueueBinding`): schema-breaking; coordinated with WOS integration profile additions.
- Adding fields to existing binding bodies: non-breaking if optional.
- Changing `composition = deny-overrides` to alternative compositions: schema-breaking; requires `specs/ai/ai-integration.md §4.6` revision.

## Conformance

### Schema validation (Stage 3)

- Per-kind body shape (discriminated `oneOf` on `kind`).
- Required-fields enforcement (CloudEvents extension attrs, hitPolicy, composition).
- Seam enum for `bindsTo.seam`.

### Lint rules (Stage 4)

Tier-S3 (Mapping readiness) rules planned:

- `BIND-LINT-001` — ServiceBinding inputs cover required API operation inputs (`SA-MUST-bind-010`).
- `BIND-LINT-002` — no multiple bindings on the same step.
- `BIND-LINT-003` — output-bindings resolve (`SA-MUST-bind-012`).
- `BIND-LINT-004` — sensitive-data ServiceBindings carry sensitivity handling (`SA-MUST-bind-013`).
- `BIND-LINT-005` — error-handling explicit (`SA-MUST-bind-014`).
- `BIND-LINT-010` — consumed events have a source (`SA-MUST-bind-021`).
- `BIND-LINT-011` — emitted events have a recipient (`SA-MUST-bind-022`).
- `WF-LINT-011` — workflow transitions reference defined events (`SA-MUST-bind-023`).
- `BIND-LINT-020` — PolicyEngineBindings normalize to standard contract (`SA-MUST-bind-030`).
- `BIND-LINT-021` — PolicyEngineBindings declare full case-field input contract (`SA-MUST-bind-032`).
- `POM-LINT-009/010/011` — DecisionTable hit-policy and completeness rules.

### Runtime conformance fixtures (Stage 4–5)

- ServiceBinding with missing required inputs blocks workflow advance.
- EventBinding without CloudEvents extension attrs is rejected at schema.
- PolicyEngineBinding with deny composes correctly (deny overrides permit).
- DecisionTable with overlapping rows under `hitPolicy = unique` is flagged.
- DecisionTable compiles to deterministic chained FEL guards.

### Current limitations

- The OpenAPI / Arazzo spec parsers (for input-coverage analysis per `SA-MUST-bind-010`) are Stage-4 work; today the rule is reviewer-driven.
- Symbolic input-range analysis for `hitPolicy = unique` overlap detection is an active research area; v1 may rely on testing rather than static analysis.
- Cross-binding consistency (e.g., the same external endpoint referenced by two ServiceBindings with different `errorHandling`) is not yet checked.

## WOS mappings

| Studio binding kind | Mapping state | WOS path |
|---|---|---|
| `ServiceBinding` | `mapsToWos` | `$.integration.bindings[*]` (binding type: `openapi-call` / `arazzo-step`) per `WOS-FEATURE-MATRIX.md` §12.1, §12.3 |
| `EventBinding (consumed)` | `mapsToWos` | `$.integration.bindings[*]` (binding type: `event-consume`) per `WOS-FEATURE-MATRIX.md` §12.2; CloudEvents extension attrs project as `wos*` fields |
| `EventBinding (emitted)` | `mapsToWos` | `$.integration.bindings[*]` (binding type: `event-emit`) per `WOS-FEATURE-MATRIX.md` §12.2 |
| `PolicyEngineBinding` | `mapsToWos` | `$.integration.bindings[*]` (binding type: `policy-engine`) per `WOS-FEATURE-MATRIX.md` §12.5; output normalized to `{decision, reasons, obligations}` |
| `DecisionTable` (= DecisionRule with form=table) | `mapsToWos` | `$.lifecycle.transitions[*].guard` (chained FEL expression sequence) per `wos-workflow.schema.json` |

The compiler (Stage 5) is responsible for producing the chained-FEL-guard sequence from a DecisionTable; this spec specifies the **input contract** to that compilation, not the FEL-emission algorithm.

## Examples

### Example 1: ServiceBinding for federal data broker eligibility check

A SNAP redetermination workflow has a `system-check` step: "Verify household income against federal IRS data broker."

```text
ServiceBinding {
  body: {
    operationRef: "POST /eligibility/income-verify",
    operationKind: "openapi",
    apiSpecRef: "fed-broker-2026.openapi.json",
    inputBindings: [
      { caseFilePath: "household.members[*].ssn",      requestPath: "request.householdMembers[*].taxpayerId" },
      { caseFilePath: "household.income.monthlyGross", requestPath: "request.declaredMonthlyIncome" }
    ],
    outputBindings: [
      { responsePath: "response.matchResult",      target: "incomeVerification.outcome",      targetKind: "decision-input" },
      { responsePath: "response.discrepancyAmount", target: "household.income.discrepancy",     targetKind: "caseFile-update" }
    ],
    errorHandling: {
      onError: "fallback",
      retryPolicy: { attempts: 3, backoff: "exponential" },
      fallbackBindingRef: "manual-income-verification-task"
    },
    sensitivityHandling: {
      redaction: ["ssn"],         // redact SSNs in audit log entries
      retention: "7y"             // PII retention per workspace policy
    }
  },
  bindsTo: { kind: "step", ref: "system-check.income-verification" },
  seam: "contractHook"
}
```

Tier-S4 readiness: this step has `system-check` kind in WorkflowIntent → `WF-LINT-007` requires a ServiceBinding. ✓

### Example 2: EventBinding for `application.submitted`

```text
EventBinding {
  body: {
    eventName: "application.submitted",
    direction: "consumed",
    payloadShape: [
      { fieldName: "applicantId",      fieldType: "string",  sensitivity: "pii" },
      { fieldName: "submittedAt",      fieldType: "timestamp" },
      { fieldName: "applicationFormRef", fieldType: "string" }
    ],
    cloudEventsExtensions: {
      woscausationeventid: "${prior.eventId}",
      woscorrelationkey:  "${case.id}"
    },
    channel: "snap-applications-queue",
    bindsTo: { kind: "trigger", ref: "intake-phase.start" }
  },
  seam: "lifecycleHook"
}
```

Workflow transition references `application.submitted` as trigger → `WF-LINT-011` resolves. ✓

### Example 3: PolicyEngineBinding for fraud screening

```text
PolicyEngineBinding {
  body: {
    engineKind: "opa",
    engineEndpointRef: "https://policy.dhs.state.gov/v1/data/snap/fraud-screening",
    policyRef: "snap.fraud_screening.v3",
    inputContract: {
      caseFilePaths: [
        "applicant.priorClaims",
        "household.address",
        "application.declaredIncome",
        "matches.federalDataBroker"
      ],
      additionalContext: { jurisdiction: "state-CA" }
    },
    outputNormalization: {
      decisionMapping: { permit: "approve-screening", deny: "refer-to-investigation", "not-applicable": "no-screening-required" },
      reasonsMapping: {
        "RC-001": "High prior-claim volume",
        "RC-002": "Address pattern match with known fraud cluster",
        "RC-003": "Income-bracket mismatch with declared employment"
      },
      obligationsMapping: {
        "OBL-investigate": { workflowStep: "fraud-investigation-task" }
      }
    },
    composition: "deny-overrides"
  },
  bindsTo: { kind: "transition.guard", ref: "intake-to-eligibility-check" },
  seam: "contractHook"
}
```

Engine deny → workflow routes to `fraud-investigation-task` (deny-overrides composition); reviewer sees plain-language reasons via the `reasonsMapping`. ✓

### Example 4: DecisionTable for income-bracket eligibility

```text
DecisionTable (= DecisionRule with form="table") {
  body: {
    form: "table",
    inputs: [
      { name: "householdSize", dataElementRef: "household.size", dataType: "integer" },
      { name: "monthlyIncome", dataElementRef: "household.income.monthlyGross", dataType: "money" }
    ],
    outputs: [
      { name: "result", target: "eligibility.outcome", targetKind: "decision-output" }
    ],
    rows: [
      { id: 1, when: { householdSize: ">= 1", monthlyIncome: "<= 1473" }, then: { result: "eligible" }, sourceCitation: { sourceVersion: "FNS-IM-2026-3", sectionAnchor: "§B.4" } },
      { id: 2, when: { householdSize: ">= 2", monthlyIncome: "<= 1984" }, then: { result: "eligible" }, sourceCitation: { sourceVersion: "FNS-IM-2026-3", sectionAnchor: "§B.4" } },
      { id: 3, when: { householdSize: ">= 3", monthlyIncome: "<= 2495" }, then: { result: "eligible" }, sourceCitation: { sourceVersion: "FNS-IM-2026-3", sectionAnchor: "§B.4" } },
      { id: 4, when: { householdSize: "*",    monthlyIncome: "*"     },   then: { result: "not-eligible" } }
    ],
    hitPolicy: "first-match",
    completenessRequirement: "all-inputs-covered",
    fallback: { result: "manual-review" }
  }
}
```

Compiles to a chained FEL guard sequence: `if (householdSize >= 1 && monthlyIncome <= 1473) return "eligible"; else if ... else return "not-eligible"`. **No DMN export.**

## Open issues

- **OpenAPI/Arazzo spec ingestion.** This spec defines binding shape; the import pipeline (parsing the spec, surfacing operations to AI for binding suggestion) is workspace tooling, deferred.
- **Engine-specific OPA/Cedar dialect handling.** All three engines have different policy languages; the binding's `policyRef` is a string, not a structured policy. Cross-engine portability is not in scope.
- **Symbolic input-range overlap detection** for DecisionTable `hitPolicy = unique`. Active research; v1 may rely on scenario testing.
- **Multi-step ServiceBindings beyond Arazzo.** What if a workflow step needs a custom orchestration not expressible in Arazzo? Custom binding type is sketched but not specified in detail.
- **Evented-vs-synchronous boundary.** Some workflow steps could be either; the spec relies on reviewer judgment to pick.

## Cross-references

- Concept model: [`../CONCEPT-MODEL.md`](../CONCEPT-MODEL.md) §1.20–1.23.
- Companion PRD: integration capabilities (§3, §5).
- Upstream: [`policy-object-model.md`](policy-object-model.md), [`workflow-intent.md`](workflow-intent.md), [`workspace.md`](workspace.md).
- Downstream: [`compiler-contract.md`](compiler-contract.md), [`readiness-validation.md`](readiness-validation.md), [`scenario-authoring.md`](scenario-authoring.md).
- WOS: `WOS-FEATURE-MATRIX.md` §12 (Integration Profile rows 12.1–12.5), `specs/ai/ai-integration.md §4.6` (policy-engine composition), `wos-workflow.schema.json` `$.integration.bindings`.
- ADRs: ADR-0077 (named seams invariant), ADR-0073 (case initiation handoff for Formspec coprocessor).
- Repo conventions: [`../../CONVENTIONS.md`](../../CONVENTIONS.md).
