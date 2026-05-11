You are taking over architecture work in the Formspec-Labs/work-spec repo.

Your task is to design and plan a boundary refactor that separates true case management from workflow/process execution, without restarting or weakening WOS.

Context discovered so far
=========================

We analyzed the current work-spec architecture and found:

1. WOS is correctly and coherently positioned as a Workflow Orchestration Standard:
   - It governs sensitive workflows.
   - It defines lifecycle topology, states, transitions, guards, events, actors, case data, provenance, durable execution, governance, AI oversight, signatures, custody, advanced governance, and assurance.
   - This is valuable and should be preserved.

2. The current model conflates two concepts:
   - A real-world Case: the durable domain object representing a matter, request, investigation, benefit application, grant, complaint, permit, intervention, appeal, inspection, etc.
   - A workflow/process instance: the runtime execution of a WOS WorkflowDocument.

3. The current `CaseInstance` model is actually a workflow instance:
   - It contains `definition_url`, `definition_version`, lifecycle `configuration`, `case_state`, active tasks, timers, pending events, status, governance state, volume counters, and provenance position.
   - Its own docs describe it as a running workflow instance.
   - This is fine for a workflow runtime artifact, but dangerous as the root product abstraction for case management.

4. The public API currently exposes `CaseInstance` as the public projection of a running WOS workflow instance:
   - It carries workflow binding, lifecycle posture, active configuration, and `caseState`.
   - Tasks, timers, holds, governance, related cases, and provenance are subresources.
   - This is sensible as a runtime/process API, but too narrow for a true case-management product API.

5. The repo already recognizes adaptive case management through Advanced Governance:
   - DCR-style constraint zones support work where next actions are not predetermined.
   - However, constraint zones are overlays on kernel compound states, not the primary case abstraction.

6. The implementation is mature enough that a restart would be wasteful:
   - There are mature crates for core, lint, conformance, runtime, Formspec binding, export, etc.
   - There is significant schema, lint, conformance, and runtime infrastructure.
   - The correct move is a boundary/layering refactor, not a rewrite.

Consultant recommendation
=========================

Do NOT start over.

Do NOT rewrite the WOS kernel.

Do NOT absorb all case-management concepts into WOS.

Do perform a targeted architectural refactor:

    Case != CaseInstance

    Case is the durable domain aggregate.
    CaseProcess, WorkflowInstance, or GovernedProcessInstance is the WOS runtime/process instance attached to a Case.

Preferred vocabulary:

    Case:
      Durable real-world matter being resolved.

    WorkflowDocument:
      Author-time WOS process definition.

    CaseProcess:
      Runtime execution of a WorkflowDocument attached to a Case.
      This is what current `CaseInstance` mostly represents.

    CaseArtifact:
      Durable item attached to a Case: form response, evidence, document, note, message, decision, signature, external record, generated notice, etc.

    CaseDecision:
      Durable determination, recommendation, appeal outcome, override, closure decision, amendment, correction, rescission, reinstatement, etc.

    CaseState:
      Durable case-domain data and summary state.

    ProcessState:
      Runtime workflow/process state: lifecycle configuration, timers, active tasks, pending events, transition status, governance state, etc.

Target architecture
===================

The end-state architecture should look like this:

    Case
      ├── id
      ├── caseType
      ├── title / summary
      ├── status
      ├── subjects
      ├── participants
      ├── organizations
      ├── issues / needs / allegations / goals
      ├──  ├── notes
      ├── communications
      ├── decisions / determinations
      ├── services / interventions
      ├── relatedCases
      ├── risk / priority / impact
      ├── processes[]
      ├── timeline
      ├── provenanceRefs
      ├── createdAt / updatedAt / closedAt
      └── tenant / scope

    CaseProcess
      ├── id
      ├── caseId
      ├── workflowUrl
      ├── workflowVersion
      ├── lifecycleState
      ├── configuration
      ├── activeTasks
      ├── timers
      ├── pendingEvents
      ├── governanceState
      ├── processCaseState / caseStateProjection
      ├── provenancePosition
      ├── createdAt / updatedAt
      └── status

    CaseArtifact
      ├── id
      ├── caseId
      ├── processId?
      ├── artifactType
      ├── source
      ├── custodyRef?
      ├── verificationLevel?
      ├âme
      ├── rationaleRefs
      ├── authorityRefs
      ├── appealRights?
      ├── effectiveAt?
      ├── supersedesDecisionId?
      ├── createdBy
      ├── createdAt
      └── provenanceRefs

Important design principle:

    WOS processes should be instruments operating on or within a Case.
    They should not be the Case.

The product API should eventually be rooted in `/cases`, not `/instances`.

Suggested API direction:

    GET  /api/v1/cases
    POST /api/v1/cases
    GET  /api/v1/cases/{caseId}
    GET  /api/v1/cases/{caseId}/processes
    POST /api/v1/cases/{caseId}/processes
    GET  /api/v1/cases/{caseId}/artifacts
    GET  /api/v1/cases/{caseId}/decisions
    GET  /api/v1/cases/{caseId}/communications
    GET  /api/v1/cases/{caseId}/timeline
    GET  /api/v1/cases/{caseId}/related

Existing `/instances` or `CaseInstance` endpoints should be reframed as process/runtime endpoints:

    GET  /api/v1/case-processes/{processId}
    POST /api/v1/casease-processes/{processId}/suspend
    POST /api/v1/case-processes/{processId}/resume
    POST /api/v1/case-processes/{processId}/terminate
    POST /api/v1/case-processes/{processId}/migrate

or, if lower churn is desired:

    Keep `/api/v1/instances` temporarily, but document it as process-runtime API, not case-management API.

Deliverable sequence
====================

Follow the repo’s normal architecture sequence.

Step 1 — ADR
--------

Create a new ADR under:

    thoughts/adr/00XX-case-process-boundary.md

Working title:

    ADR 00XX — Case / Process Boundary and Case Aggregate Introduction

The ADR should include:1. Status
   - Proposed initially.

2. Context
   - WOS currently has strong workflow/process semantics.
   - `CaseInstance` currently represents a running workflow instance.
   - This creates a product-domain risk when building case management.
   - True cases can have zero, one, or many workflows/processes.
   - A workflow process can complete while the case remains open.
   - A case can close, reopen, split, merge, appeal, or spawn additional processes.

3. Decision
   - Introduce `Case` as a first-class durable domain aggregate.
   - Reframe current `CaseInstance` as `CaseProcess`, `WorkflowInstance`, or `GovernedProcessInstance`.
   - Prefer `CaseProcess` unless owner chooses otherwise.
   - A Case MAY have zero or more CaseProcesses.
   - A CaseProcess MUST reference exactly one Case.
   - A CaseProcess MUST bind to exactly one WorkflowDocument version at creation, unless migrated through the existing migration semantics.
   - WOS remains the governed workflow/process substrate.
   - Case management lives one layer above WOS.
   - Workflows interact with Case through explicit bindings/mutations/artifacts/decisions, not by treating workflow `caseState` as the whole case.

4. Consequences
   - Reduces risk of bloated lifecycle state machines.
   - Prevents `caseState` from becoming a junk drawer.
   - Makes multiple processes per case possible.
   - Allows manual/ad hoc casework outside workflows.
   - Preserves WOS kernel investments.
   - Requires new schemas, API resources, generated types, and tests.
   - May require temporary aliases for `CaseInstance`.

5. Alternatives considered
   - Keep `CaseInstance` as the case root:
     Rejected because it conflates process execution with real-world case context.
   - Rename only:
     Rejected because the structural relationship must change, not only the name.
   - Put all Case fields into WOS:
     Rejected because WOS would bloat into a case-management product ontology.
   - Start over:
     Rejected because WOS kernel, lint, conformance, runtime, and provenance infrastructure are valuable and coherent.
   - Introduce a separate Case aggregate above WOS:
     Chosen.

6. Non-goals
   - Do not rewrite the WOS lifecycle kernel.
   - Do not remove existing workflow/conformance semantics.
   - Do not design a full CRM.
   - Do not build a complete UI in this ADR.
   - Do not make every note, communication, or artifact a workflow transition.
   - Do not introduce open-ended GraphQL-style includes.
   - Do not force immediate internal renaming if aliases can reduce churn.

7. Naming decision required
   - Ask owner to choose:
     a. `CaseProcess`
     b. `WorkflowInstance`
     c. `GovernedProcessInstance`
   - Recommendation: `CaseProcess`.

Step 2 — thoughts/specs end-state document
------------------------------------------

Create:

    thoughts/specs/YYYY-MM-DD-case-process-boundary-e-state.md

This is not the formal W3C-style spec yet. It should be a detailed target-state design note.

It should include:

1. Problem statement
   - Current case runtime object is process-centric.
   - Product-level case management needs a durable aggregate independent of any one workflow.

2. Definitions
   - Case
   - CaseProcess
   - WorkflowDocument
   - CaseArtifact
   - CaseDecision
   - CaseRelationship
   - CaseTimelineEvent
   - CaseState
   - ProcessState

3. Architecture diagram in text
   - Case has many CaseProcesses.
   - CaseProcess references one WorkflowDocument.
   - CaseArtifacts and CaseDecisions may be linked to a process but belong to the Case.
   - Provenance can be per-process and per-case.

4. Data ownership
   - Case owns durable domain context.
   - WOS process owns lifecycle/runtime state.
   - Artifacts own evidence/document/response payloads or payload refs.
   - Decisions own outcome/rationale/authority.
   - Provenance records explain who/what/when/why and remain queryable.

5. Interaction model
   - Workflow can read selected case fields through explicit binding.
   - Workflow can write case mutations only through explicit governed output paths:
     - CaseStateMutation
     - CaseArtifact creation
     - CaseDecision creation
     - CaseTimelineEvent append
   - Workflow cannot silently mutate arbitrary case fields.

6. Public API target
   - `/cases` becomes product root.
   - `/case-processes` or `/instances` remains runtime/process root.
   - Include closed aggregation seams only.

7. Migration stance
   - Because this is pre-release, prefer clean naming where possible.
   - But to minimize code churn, allow Rust/internal aliases temporarily:
       pub type CaseInstance = CaseProcess;
   - Public API should be corrected earlier than internal implementation.

8. Compatibility stance
   - Identify whether existing `$wosCaseInstance` marker remains for now.
   - Recommended: keep runtime artifact marker initially, add `caseId`, and document it as legacy-named process artifact until a later ADR renames the marker.
   - Avoid large marker rename unless owner wants a greenfield break.

Step 3 — thoughts/plan landing plan
---------------------------------

Create:

    thoughts/plans/YYYY-MM-DD-case-process-boundary-landing.md

The plan should be phased, with each phase independently testable.

Suggested phases:

Phase A — ADR + vocabulary
  - Land ADR.
  - Update high-level docs to state:
      Case != CaseInstance.
      CaseProcess  the process runtime object.
  - Add glossary entries.

Phase B — Formal specs
  - Add W3C-style `/specs/cases/case.md`.
  - Add `/specs/cases/case-process.md` or update `/specs/api/instae.md` to clarify process semantics.
  - Add `/specs/cases/case-artifacts.md` if separate.
  - Add `/specs/cases/case-decisions.md` if separate.

Phase C — Schemas
  - Add `schemas/api/casschema.json`.
  - Add `schemas/api/case-artifact.schema.json` if not embedded in case schema.
  - Add `schemas/api/case-decision.schema.json` if not embedded.
  - Add process/link fields to existing instance schema:
      `caseId`
      `processKind` or `workflowBinding`
      possibly rename public `$defs/CaseInstance` to `CaseProcess` or add alias.
  - Preserve closed taxonomy discipline.
  - Use TypeID-in-URN identity discipline.
  - Ensure tenant consistency between Case and CaseProcess.

Phase D — API docs and OpenAPI
  - Add case routes.
  - Add processoutes or alias existing instance routes.
  - Update OpenAPI references.
  - Add route coverage tests if the repo has API guardrails for route coverage.

Phase E — Rust/core model updates
  - Add typed model structs:
      Case
      CaseProcessLink
    CaseArtifact
      CaseDecision
      CaseRelationship
      CaseTimelineEvent
  - Decide whether to rename `CaseInstance` internally.
  - If not renaming immediately, add:
      pub type CaseProcess = CaseInstance;
    or the inverse depending on owner decision.
  - Add `case_id` to runtime/process instance.
  - Enforce tenant consistency.

Phase F — Runtime/API behavior
  - Creating a process requires a Case or an explicit create-and-attach mode.
  - Intake handoff can:
      create new Case + CaseProcess,
      attach new CaseProcess to existing Case,
    defer without creating process,
      attach artifact to existing Case without process if policy allows.
  - Process completion does not automatically close Case unless explicit closure policy says so.
  - Case closure rejects active processes unless closure policy suspends/terminates them or marks case closed-with-active-exceptions.

Phase G — Portal/generated types
  -pdate generated SDK/types.
  - Update case portal models:
      product views should consume Case.
      workflow runtime panels consume CaseProcess.
  - Avoid product pages rooted only in lifecycle configuration.

Phase H — Tests
  - Add schema test
  - Add Rust unit tests.
  - Add conformance or API fixtures if appropriate.
  - Add regression tests preventing `Case` from requiring a workflow.

Formal spec requirements
========================

Create W3C-style specs under `/specs/cases/`.

Minimum formal spec:

    specs/cases/case.md

Optional supporting specs:

    specs/cases/case-process.md
    specs/cases/case-artifact.md
    specs/cases/case-decision.md

Formal specs should use WOS style:

- title
- version
- date
- status
- abstract
- status of this document
- conformance
- normative sections
- non-normative examples
- MUST / SHOULD / MAY language
- processing requirements
- references

Spec: Case
----------

Normative requirements to capture:

1. A Case is a durable domain aggregate representing a real-world matter.
2. A Case MUST have a stable identifier.
3. A Case MUST have a tenant/scope boundary.
4. A Case MAY exist without any CaseProcess.
5. A Case MAY have zero, one, or many CaseProcesses.
6. A CaseProcess MUST NOT be the sole representation of a Case.
7. A Case status MUST NOT be inferred solely from any one CaseProcess lifecycle state.
8. A Case MAY be open while all processes are completed.
9. A Case MAY be closed only when closure policy is satisfied.
10. A Case MAY be reopened if reopening policy permits.
11. A Case MAY contain artifacts, decisions, notes, communications, participants, subjects, relationships, and timeline events independent of process execution.
12. A Case MAY link to related cases.
13. Case relationships MUST distinguish directional and symmetric relations.
14. Case mutations MUST produce provenance or provenance references.
15. Case access control MUST be evaluated separately from process task assignment.
16. Case tenant MUST match attached CaseProcess tenant.

Spec: CaseProcess
-----------------

Normative requirements to capture:

1. A CaseProcess is a runtime execution of a WorkflowDocument attached to a Case.
2. A CaseProcess MUST reference exactly one Case.
3. A CaseProcess MUST bind to exactly one WorkflowDocument URL/version at creation.
4. A CaseProcess lifecycle state is process runtime state, not case status.
5. CaseProcess completion MUST NOT imply Case closure unless explicit case closure policy is configured.
6. CaseProcess termination MUST NOT delete the Case.
7. CaseProcess migration MUST preserve the Case link.
8. CaseProcess events MAY create case artifacts, decisions, timeline entries, or state mutations only through explicit governed output mappings.
9. CaseProcess MUST NOT silently mutate case-domain data outside declared bindings.
10. CaseProcess active tasks belong to the process but may be projected on Case views.
11. CaseProcess provenance is process-scoped but may be aggregated into case timeline/provenance views.
12. CaseProcess tenant MUST equal Case tenant.

Spec: CaseArtifact
------------------

Normative requirements to capture:

1. A CaseArtifact belongs to exactly one Case.
2. A CaseArtifact MAY be produced by a CaseProcess.
3. A CaseArtifact MAY exist without any CaseProcess.
4. Artifact payload MAY be inline or referenced through a payload/custody ref.
5. Artifact type taxonomy should be closed-with-vendor-extension.
6. Artifacts SHOULD carry verification/custody metadata when used in rights-impacting workflows.
7. Artifact visibility MAY differ by role/participant.
8. Deleting or replacing an artifact SHOULD create provenance and should preserve audit lineage where required.

Spec: CaseDecision
------------------

Normative requirements to capture:

1. A CaseDecision belongs to exactly one Case.
2. A CaseDecision MAY be produced by a CaseProcess.
3. A CaseDecision MUST carry decision type and outcome.
4. Rights-impacting decisions SHOULD/MUST carry rationale, authority references, notice/appeal fields depending on impact level.
5. Decisions MAY be amended, corrected, rescinded, superseded, reinstated.
6. Decision lineage MUST be explicit; never overwrite prior decisions silently.
7. A case may have multiple decisions over time.

JSON schema requirements
========================

Add schemas with closed taxonomy discipline consistent with ADR 0082 style.

Minimum:

    schemas/api/case.schema.json

Possible split:

    schemas/api/case.schema.json
    schemas/api/case-artifact.schema.json
    schemas/api/case-decision.schema.json

The schema should include `$defs` for:

- Case
- CasePage
- CaseCreateRequest
- CaseUpdateRequest, only if mutation route is included
- CaseStatus
- CaseType
- CaseSubject
- CaseParticipant
- CaseRelationship
- CaseRelationshipKind
- CaseProcessLink
- CaseArtifactSummary
- CaseDecisionSummary
- CaseTimelineEvent
- CaseClosurePolicy or CaseClosureState, if needed

Case fields, minimum:

    id
    tenant
    caseType
    title?
    summary?
    status
    subjects[]
    participants[]
    relatedCases[]
    processes[]
    artifactsSummary?
    decisionsSummary?
    createdAt
    updatedAt
    closedAt?
    reopenedAt?
    provenanceRefs?

CaseStatus initial taxonomy:

    open
    pending
    on-hold
    closed
    reopened
    archived

But explore whether this should be:

    active
    suspended
    closed
    archived

Important: Do NOT simply reuse workflow lifecycle `active | suspended | migrating | completed | terminated | stalled`.

Case status is not process status.

CaseRelationshipKind should include at least the current related-case vocabulary where appropriate:

    parent
    child
    sibling
    predecessor
    successor
    appeals
    appealed-by
    related
    supersedes

Consider additional case-native values only if needed:

    duplicate-of
    duplicated-by
    split-from
    split-into
    merged-into
    merged-from

Use closed-with-vendor-extension style if consistent with current API discipline.

CaseProcessLink fields:

    processId
    workflowUrl
    workflowVersion
    lifecycleState
    status
    startedAt
    updatedAt
    completedAt?
    role?
    primary?

Process link should be a summary. Full process runtime remains in process/instance schema.

Add or update process schema fields:

    caseId: required for CaseProcess
    workflowUrl
    workflowVersion
    lifecycleState
    configuration
    caseStateProjection or processCaseState
    createdAt
    updatedAt

If current schema still names this `$defs/CaseInstance`, decide whether to:
  - rename to `$defs/CaseProcess`, or
  - add `$defs/CaseProcess` and keep `$defs/CaseInstance` as deprecated alias/projection.

Recommended low-risk option:
  - Add `$defs/CaseProcess`.
  - Keep `$defs/CaseInstance` only as a temporary compatibility alias or legacy name.
  - Update docs to say `CaseInstance` is legacy process-runtime naming.

Edge cases to explore before implementation
===========================================

Before writing the ADR, explicitly explore the following edge cases and decide invariants.

1. Case with zero processes
   - A staff member creates a case manually.
   - No workflow is active yet.
   - This MUST be valid.

2. Case with one process
   - Simple case where one workflow fully manages lifecycle.
   - This should still be easy.

3. Case with multiple parallel processes
   - Example: eligibility review, document request, and signature workflow active at the same time.
   - Case remains open until closure policy says otherwise.

4. Case with sequential processes
   - Intake completes.
   - Eligibility process starts.
   - Appeal process starts later.
   - Processes should be linked by case, not fused into one giant lifecycle.

5. Process completes but Case stays open
   - Common case.
   - Completion of intake or signature process should not automatically close the case.

6. Case closure with active processes
   - Decide whether this is rejected by default.
   - Recommended default: reject closure while active processes exist unless explicit closure policy handles suspension/termination/exceptions.

7. Case reopen
   - Reopening a closed case should not mutate old process outcomes.
   - It may create a new process or reopen case status.

8. Case split
   - One case becomes multiple cases.
   - Original artifacts/decisions need lineage.
   - CaseRelationship kinds may need `split-from` / `split-into`.

9. Case merge
   - Multiple cases combine into one.
   - Preserve source case IDs and provenance.
   - Avoid destructive merge.

10. Duplicate cases
   - Case A is duplicate of Case B.
   - Does this close A? Link A to B?
   - Keep lineage.

11. Appeal
   - Appeal should likely be a new CaseProcess attached to the same Case or a related Case, depending on domain.
   - It should not erase original decision.

12. Supersession / amendment / correction / rescission
   - ADR 0066 concepts should compose with CaseDecision lineage.
   - New decisions should supersede or amend old decisions, not overwrite them.

13. Intake handoff
   - Existing intake semantics include accepted, attachToExistingCase, deferred.
   - Attach-to-existing should attach artifacts/processes to an existing Case, not merely mutate a workflow instance.

14. Deferred intake
   - Intake may create an artifact or pending handoff without creating a CaseProcess.
   - Decide whether it creates a Case in pending status or remains outside Case until accepted.

15. Cross-case correlation / fan-out
   - Existing correlation-group behavior should target processes, cases, or both?
   - Decide whether case-level correlationKey is separate from process-level event fan-out.

16. Related cases
   - Directional vs symmetric links.
   - `parent/child`, `predecessor/successor`, `appeals/appealed-by`, `supersedes`, `related`.
   - Avoid assuming bidirectionality unless explicit.

17. Process migration
   - Process migration must preserve caseId.
   - Case binding versioning should be explicit.
   - Migration should not silently reinterpret case data.

18. Workflow reads case data
   - Declare `reads`.
   - Missing data should either fail, defer, or create task/request depending on binding policy.

19. Workflow writes case data
   - Writes should be explicit:
       CaseStateMutation
       CaseArtifact creation
       CaseDecision creation
       Timeline append
   - Avoid arbitrary writes into `caseState`.

20. Conflict between processes
   - Two processes attempt to write same case field or decision.
   - Need conflict policy:
       last-write-wins is probably unacceptable for rights-impacting cases.
       Prefer versioned mutations with provenance and conflict detection.

21. Notes and communications
   - A caseworker note should not require a workflow transition.
   - A phone call should not require a workflow transition.
   - But both should be provenance/timeline visible if configured.

22. Evidence/document lifecycle
   - Evidence may be uploaded independently of workflow.
   - A workflow may request, validate, or consume evidence.
   - Evidence belongs to Case, not process.

23. Permissions
   - Case-level read/write permission differs from task assignment.
   - A user may see a task but not all case artifacts.
   - A supervisor may see case but not perform process task.
   - Access control must not be inferred from active workflow actor assignment alone.

24. Tenant / organization scoping
   - Case and CaseProcess tenants must match.
   - Cross-tenant links should be rejected unless a federation profile explicitly permits them.

25. Participant privacy / redaction
   - Some participants may see only their own submissions.
   - Case artifacts may be redacted by role.
   - Schema should allow visibility metadata or defer to access-control policy.

26. AI agents
   - Agents may act within CaseProcess.
   - Agents must not write directly to Case outside governed output path.
   - Agent-suggested notes/decisions should remain distinguishable from verified facts.

27. DCR constraint zones
   - Constraint zones remain valid inside WOS process states.
   - But ad hoc case activity may also exist outside a process.
   - Do not force all adaptive work into DCR.

28. Case status vs process status
   - Do not reuse `completed`, `terminated`, `stalled` from process status as case status.
   - A case can be `open` with no active process.
   - A process can be `completed` while case is `open`.

29. Case archival
   - Archived case should probably reject new processes unless reopened.
   - Process records remain queryable.

30. Generated SDK and API guardrails
   - Updating schemas requires generated types and response conformance tests.
   - Avoid adding open maps where ADR 0082 closed-taxonomy discipline applies.

31. Backward compatibility
   - Project appears pre-release, so greenfield corrections are acceptable.
   - But avoid high-churn renames unless owner chooses.
   - Use aliases if needed.

32. Naming collisions
   - `caseState` currently means workflow process business data.
   - Decide whether new Case has `state`, `caseData`, `caseSummary`, or `caseFacts`.
   - Recommended: avoid naming the aggregate payload simply `caseState` unless strongly defined.

33. External systems of record
   - Case may mirror external matter IDs.
   - Artifact/decision may reference external records.
   - Do not assume WOS is always source of truth for every field.

34. Timeline
   - Case timeline aggregates:
       case-created
       artifact-added
       decision-issued
       process-started
       task-completed
       process-completed
       case-closed
       case-reopened
   - But provenance remains the authoritative audit detail.

35. Closure policy
   - Case closure should be explicit.
   - Possible modes:
       manual
       when-all-required-processes-complete
       when-decision-issued
       external-system-driven
   - Do not infer closure from arbitrary process completion.

Invariants to enforce
=====================

The ADR/specs should establish these invariants unless strong evidence says otherwise:

1. A Case MAY have zero or more CaseProcesses.
2. A CaseProcess MUST reference exactly one Case.
3. A CaseProcess MUST bind to one WorkflowDocument URL/version at creation.
4. A CaseProcess lifecycle state MUST NOT be treated as Case status.
5. Case closure MUST be explicit or policy-derived, never accidental.
6. Process completion MUST NOT imply Case closure by default.
7. Case tenant and CaseProcess tenant MUST match.
8. Workflow writes to Case MUST use declared governed outputs.
9. Case artifacts and decisions MUST preserve lineage and provenance.
10. Case notes, communications, and artifacts MAY exist outside workflow execution.
11. Case relationships MUST distinguish directional from bidirectional links.
12. Prior decisions/artifacts MUST NOT be overwritten silently.
13. Process migration MUST preserve Case linkage.
14. Runtime process state and durable case-domain state MUST remain separable.
15. Public product views SHOULD be rooted in Case, not process configuration.

Files to inspect first
======================

Start by inspecting these repo files:

- README.md
- specs/kernel/spec.md
- specs/api/instance.md
- schemas/wos-case-instance.schema.json
- schemas/api/instance.schema.json
- crates/wos-core/src/instance.rs
- crates/wos-core/src/model/kernel.rs
- specs/advanced/advanced-governance.md
- WOS-IMPLEMENTATION-STATUS.md
- TODO.md
- any OpenAPI/API route specs under specs/api/
- any case portal generated type bindings if present
- any server routes for instances/processes if present

Reasoning to preserve in ADR
============================

Use this reasoning in the ADR and design docs:

- WOS is strongest when treated as a governed process substrate.
- Case management is a broader product/domain concern than workflow execution.
- Treating `CaseInstance` as the root Case will force real-world case complexity into lifecycle states and `caseState`.
- That produces brittle workflows, giant status taxonomies, overloaded `caseState`, and UI/API confusion.
- The system already has strong runtime/process machinery; preserve it.
- The missing abstraction is not more workflow sophistication; it is a durable Case aggregate above WOS.
- The correct separation is:
      Formspec collects structured data/evidence.
      WOS governs procedural movement, transitions, accountability, AI oversight, and provenance.
      Case aggregates the real-world matter being resolved.
      Trellis/custody mechanisms anchor evidence and audit artifacts where applicable.
- This preserves existing investment while preventing future product drift.

Acceptance criteria
===================

This work is done when:

1. ADR exists and clearly decides Case/Process separation.
2. thoughts/specs end-state document exists and explains target architecture.
3. thoughts/plans landing plan exists and enumerates file-level changes.
4. Formal W3C-style `/specs/cases/case.md` exists.
5. Formal process/case-process spec or updated instance spec exists.
6. JSON schema(s) exist for Case and related structures.
7. Existing instance/process schema is updated or clearly aliased.
8. Public docs no longer imply that a Case is simply a workflow instance.
9. Tests prove:
   - Case can exist with zero processes.
   - Case can have multiple processes.
   - Process completion does not close Case by default.
   - Case closure with active processes is rejected or governed by explicit policy.
   - Process migration preserves caseId.
   - Tenant mismatch between Case and CaseProcess is rejected.
   - Artifacts/decisions can attach to Case independently of process.
   - Related-case links preserve directionality.
10. Existing WOS kernel tests still pass.
11. API/schema guardrails still pass.
12. Generated types are updated if schemas changed.

Suggested test commands
=======================

Use the repo’s current conventions. At minimum, run the applicable subset of:

    python3 -m pytest tests/schemas -q
    cargo check --workspace
    cargo nextest run -p wos-core --lib
    cargo nextest run -p wos-runtime --lib
    cargo nexst run -p wos-lint
    cargo nextest run -p wos-conformance

If API guardrails exist, run the ADR 0082 API contract guardrail suite as well.

Potential owner decisions
=========================

Before finalizing the ADR, identify these decisions explicitly:

1. Preferred name:
   - CaseProcess
   - WorkflowInstance
   - GovernedProcessInstance

   Recommendation: CaseProcess.

2. Public API path:
   - `/api/v1/case-processes`
   - keep `/api/v1/instances` as runtime API
   - both, with one alias/deprecated

   Recommendation: introduce `/case-processes`; keep `/instances` temporarily only if churn is too high.

3. Runtime marker:
   - Keep `$wosCaseInstance` temporarily.
   - Rename to `$wosProcessInstance` now.
   - Add alias marker support.

   Recommendation: keep marker temporarily, but document as legacy-named process artifact. Rename later only if owner wants greenfield break.

4. Case status taxonomy:
   - minimal: `open | on-hold | closed | archived`
   - richer: `open | pending | on-hold | closed | reopened | archived`

   Recommendation: start minimal but include `reopened` if reopening is first-class.

5. Schema split:
   - one `case.schema.json` with `$defs`
   - multiple schemas for case/artifact/decision

   Recommendation: one `case.schema.json` initially, with `$defs`, unless size becomes unwieldy.

6. Whether CaseArtifact and CaseDecision become separate top-level API resources immediately.

   Recommendation: define them formally now; API can expose summaries first and full endpoints in a later plan phase.

Implementation caution
======================

Do not implement code first.

The desired order is:

1. ADR
2. thoughts/specs end-state
3. thoughts/plans landing plan
4. formal `/specs`
5. schemas
6. code/tests/docs/generated types

Avoid solving this by only renaming symbols. The problem is structural.

Avoid solving this by putting every Case field into WOS. WOS should remain the process/governance standard.

Avoid breaking the existing workflow kernel. This is a boundary correction, not a restart.
