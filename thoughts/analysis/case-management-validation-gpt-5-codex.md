# Case Management Boundary Validation

> **Superseded validation artifact.** This R-file is retained as derivation history. The controlling source of truth for case-management decisions is [`case-boundary-decision-report.md`](case-boundary-decision-report.md); when this file disagrees with that report, the report controls.

**Validated by:** GPT-5 Codex, with three subagent review passes  
**Date:** 2026-05-10  
**Target:** `work-spec/thoughts/analysis/case-management.md`

## Verdict

The core thesis is sound: `Case != WorkflowProcess`, and the current `WorkflowProcess`
shape is really a workflow/runtime process artifact. The repo supports this
diagnosis.

The document is not yet technically ready to execute as a landing plan. It
needs a stricter layer boundary, an explicit identity migration decision, a
concrete API compatibility matrix, and guardrail-aware route/schema/test
ordering before implementation starts.

## What Holds

- WOS positions itself as a workflow/governance standard, not a full
  case-management product ontology. See `README.md:19-30`.
- The current public `WorkflowProcess` is explicitly described as a running WOS
  workflow instance. See `specs/api/instance.md:10`.
- The runtime artifact carries workflow binding, lifecycle configuration,
  `caseState`, timers, active tasks, status, provenance cursor, and runtime
  state. See `schemas/wos-process.schema.json:5` and
  `crates/wos-core/src/instance.rs:19`.
- The proposed Case / CaseProcess separation would reduce product/API
  confusion if it is framed as a boundary correction, not a WOS rewrite.

## Blocking Issues

1. **The plan risks pulling product case management into WOS normative scope.**
   The document says case management lives above WOS, but then directs formal
   `/specs/cases/*` specs and rich product concepts like notes,
   communications, services, artifacts, and decisions. That conflicts with
   WOS' posture as a workflow governance/orchestration standard.

   Evidence: `case-management.md:204`, `case-management.md:392`,
   `case-management.md:519`, `README.md:21`, `README.md:30`,
   `POSITIONING.md:21`.

2. **Rights-impacting decisions outside a process need a hard conformance
   rule.** If `CaseDecision` can exist without a `CaseProcess`, the plan must
   say whether that is outside WOS conformance or whether it must still satisfy
   WOS due-process, provenance, notice, and appeal obligations.

   Evidence: `case-management.md:434`, `case-management.md:478`,
   `WOS-FEATURE-MATRIX.md:177`, `WOS-FEATURE-MATRIX.md:201`,
   `specs/kernel/spec.md:750`.

3. **The identity split is under-specified.** Current `WorkflowProcess.processId`
   already uses the reserved `_case_` TypeID family, and public
   `WosResourceUrn` accepts only `case | prov | gov | ai | assurance | x-*`
   families. A new first-class `Case.id` plus `CaseProcess.processId` needs an
   explicit ADR 0092 amendment or an owner decision explaining how old instance
   IDs become Case IDs, Process IDs, aliases, or paired IDs.

   Evidence: `case-management.md:160`, `case-management.md:582`,
   `schemas/wos-process.schema.json:39`,
   `schemas/api/_common.schema.json:18`,
   `thoughts/adr/0092-api-typeid-urn-identity.md:63`.

4. **The schema/spec landing sequence is not executable as written.** Adding
   `schemas/api/case.schema.json` requires a matching `specs/api/case.md` that
   names the schema file and `$id`. Exported API models must also appear as
   top-level `$ref` / `oneOf` entries. A formal `specs/cases/case.md` can exist,
   but it does not satisfy the current API-schema guardrails by itself.

   Evidence: `case-management.md:327`, `case-management.md:493`,
   `case-management.md:501`,
   `tests/schemas/test_wos_api_schema_discipline.py:145`,
   `tests/schemas/test_wos_api_schema_discipline.py:155`.

5. **`/instances` versus `/case-processes` remains an undecided compatibility
   contract.** Current OpenAPI and API specs are deeply rooted in `/instances`
   and `WorkflowProcess`; the plan alternates between introducing
   `/case-processes`, keeping `/instances`, or doing both. It must define alias
   behavior, deprecation headers, operationId stability, idempotency-key route
   scoping, generated type names, and subresource migration rules.

   Evidence: `case-management.md:143`, `case-management.md:910`,
   `specs/api/instance.md:82`, `api/wos-public-api.openapi.json:313`,
   `api/wos-public-api.openapi.json:5883`.

6. **Appeal semantics conflict with the target model.** Current appeal API says
   appeals are filed against completed instances, do not reopen the case, and
   live under `/instances/{id}/appeals`. The new plan says appeal is likely a
   new `CaseProcess` or related Case. That must be decided before landing API
   or schema work.

   Evidence: `specs/api/appeal.md:12`, `specs/api/appeal.md:38`,
   `case-management.md:668`.

## Additional Risks

- `CaseProcessLink` lists both `lifecycleState` and `status` without defining
  whether `status` is a new case-process taxonomy, a duplicate of public
  `LifecycleState`, or the raw runtime `InstanceStatus`.
- Case closure/archive needs to preserve legal holds, continuation of service,
  read-only process records, and appeal windows.
- The plan identifies mutation stories but does not map them to concrete
  routes, request/response shapes, idempotency rules, or tests: closure,
  reopen, split, merge, duplicate, artifact attachment, decision attachment,
  notes, communications, evidence lifecycle, and conflict detection all need
  route-level treatment.
- Several literal typos would trip agent execution if copied into tasks:
  `casease-processes`, `instae.md`, `casschema.json`, `processoutes`, `-pdate`,
  `cargo nexst`, and `case-process-boundary-e-state.md`.

## Guardrail Verification

Command:

```bash
uv run pytest tests/schemas/test_wos_api_schema_discipline.py -q
```

Result:

```text
14 passed, 1 failed
FAILED test_api_facts_record_kind_reserved_literals_match_kernel
Extra items in the right set:
'signatureAdmissionFailed'
```

The failure confirms that the API guardrail baseline is currently red:
`schemas/wos-workflow.schema.json` includes `signatureAdmissionFailed` in the
kernel `FactsTierRecord.recordKind` enum, while
`schemas/api/provenance.schema.json` does not expose the same reserved literal
in API `FactsRecordKind`.

## Recommended Next Step

Do not implement from `case-management.md` as-is. First revise it into a
guardrail-aware ADR and landing plan with these decisions made explicitly:

- WOS layer boundary: process substrate vs optional/product Case layer.
- Rights-impacting decisions outside process execution.
- Case and CaseProcess TypeID / URN families and migration semantics.
- `/instances` compatibility and `/case-processes` alias/deprecation behavior.
- Required `specs/api/*.md`, schema exports, OpenAPI, route registry, generated
  types, and route coverage ordering.
- Appeal migration rule under the new Case / CaseProcess model.
