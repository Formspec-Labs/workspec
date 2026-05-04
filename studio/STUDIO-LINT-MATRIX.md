# Studio (Authoring) Lint Matrix

Companion to the parent [`../LINT-MATRIX.md`](../LINT-MATRIX.md). Catalogues 111 readiness rules across six tiers (S1–S6) per `studio/specs/readiness-validation.md`. Implementation lives in [`crates/wos-studio-lint`](crates/wos-studio-lint/).

> **Authoritative count:** the registry's `all_studio_rules()` length is the single source of truth. Asserted in `registry::tests::registry_carries_at_least_seventy_rules` (locked at 111 as of 2026-05-03 after I-wave Phase A added 35 rules across SV-LINT/BIND-LINT/WF-LINT/MAP-LINT/RA-LINT/PROV-LINT clusters and J3 review-fixup added ID-LINT-004). Update this number when a new rule lands; the assertion catches drift.

**Boundary:** parent rules (T1/T2/T3) cover the kernel envelope. Studio rules (S1–S6) cover authoring documents — what authors write before the compiler emits a kernel. The two engines do not share rules.

## Graduation ladder

| Graduation | Meaning |
|---|---|
| `draft` | Implemented + unit-tested; no end-to-end fixture coverage. |
| `tested` | ≥1 executable fixture exercises this rule. |
| `stable` | Tested + unchanged across ≥3 consecutive releases. |
| `load_bearing` | Removing breaks ≥2 executable fixtures. |

## Engine surfaces

- **`lint_document(&StudioDocument)`** — runs all doc-local rules.
- **`lint_workspace(&Workspace)`** — runs cross-document rules + every doc's local rules.

## S1 — Source vault

| ID | Severity | Graduation | Surface | Summary |
|---|---|---|---|---|
| `SV-LINT-001` | error | draft | doc | Every SourceCitation MUST resolve to a real SourceSection. |
| `SV-LINT-002` | error | draft | doc | Citation excerpts MUST appear in the referenced SourceSection. |
| `SV-LINT-003` | error | draft | workspace | No PolicyObject relies solely on disputed/superseded SourceVersions. |
| `SV-LINT-004` | error | draft | doc | Current SourceVersions MUST carry effectiveStart. |
| `SV-LINT-005` | error | draft | doc | Section anchors MUST be unique within a SourceVersion. |
| `SV-LINT-006` | error | draft | doc | Low-confidence ExtractedClaims (<0.5) MUST NOT be auto-approved. |

## S2 — Policy object readiness

| ID | Severity | Graduation | Surface | Summary |
|---|---|---|---|---|
| `POM-LINT-001` | error | draft | doc | Approved PolicyObject MUST carry citation or basis-assumption. |
| `POM-LINT-002` | error | draft | doc | originClass=approved-interpretation MUST carry reviewerResolution. |
| `POM-LINT-003` | error | draft | doc | Approved PolicyObject MUST declare originClass. |
| `POM-LINT-007` | error | draft | workspace | No circular Supersession chains. |
| `POM-LINT-008` | error | draft | workspace | Conflict MUST be resolved or waived. |
| `POM-LINT-020` | error | draft | workspace | PolicyObject past `approved` (mapped/validated/published/superseded/deprecated/demoted) requires ApprovalDecision (SA-MUST-pom-020). |
| `POM-LINT-033` | error | draft | workspace | AppealRight.outcomeRef MUST equal linked Notice's outcomeRef when both explicit (SA-MUST-pom-033). |
| `POM-LINT-040` | error | draft | workspace | Two approved Deadlines on same trigger with different durations MUST be filed as Conflict (SA-MUST-pom-040; tractable lint-time slice). |
| `POM-LINT-051` | warning | draft | workspace | Deontic constraints sharing (subject, action) flagged as composition candidates without compositionAttestation=reviewed (SA-MUST-pom-051). |
| `PROV-LINT-002` | error | draft | doc | Provenance chain MUST resolve to citation/assumption/attestation. |
| `PROV-LINT-003` | error | draft | doc | originClass=approved-interpretation MUST carry ReviewerResolution. |
| `PROV-LINT-004` | error | draft | doc | originClass=local-practice MUST carry attestation. |
| `EFF-LINT-001` | warning | draft | doc | Redundant effectiveness duplicate. |
| `EFF-LINT-002` | error | draft | workspace | Effectiveness widening disallowed. |
| `EFF-LINT-003` | error | draft | doc | enjoined=true MUST carry enjoinedScope. |
| `AI-LINT-001` | error | draft | doc | AI-extracted PolicyObject MUST carry aiLineage block. |
| `AI-LINT-002` | error | draft | doc | AI-extracted promoted past extracted MUST have humanApprover. |
| `EQ-LINT-002` | error | draft | doc | Every ProtectedCategory MUST cite legalBasis. |
| `TERM-LINT-001` | error | draft | workspace | TerminologyMap entry MUST NOT point to deprecated CanonicalTerm. |
| `TERM-LINT-002` | warning | draft | doc | DataElement canonicalTermRef=manual-pending awaits attestation. |
| `TERM-LINT-003` | warning | draft | doc | DataElement uses legacy sensitivity alias. |

## S3 — Mapping readiness

| ID | Severity | Graduation | Surface | Summary |
|---|---|---|---|---|
| `MAP-LINT-001` | error | draft | workspace | Every approved PolicyObject MUST have a Mapping. |
| `MAP-LINT-002` | error | draft | doc | mapsToWos targets MUST carry wosConceptId + wosJsonPath. |
| `MAP-LINT-003` | error | draft | doc | requiresSpecExtension MUST carry substantive ExtensionRecord. |
| `MAP-LINT-004` | warning | draft | doc | unmappedButApproved MUST carry substantive rationale. |
| `MAP-LINT-005` | error | draft | workspace | No two PolicyObjects collide on same target. |
| `MAP-LINT-006` | error | draft | workspace | Workflow-bearing PolicyObjects MUST NOT be unmappedButApproved without override. |
| `MAP-LINT-007` | error | draft | workspace | Workflow-bearing MUST NOT have open ExtensionRecord blocking advance. |
| `MAP-LINT-008` | error | draft | doc | x- targets MUST carry extension-registry entry. |
| `EFF-LINT-004` | warning | draft | workspace | Mapping effectiveness collision. |

## S4 — Workflow readiness

| ID | Severity | Graduation | Surface | Summary |
|---|---|---|---|---|
| `WF-LINT-001` | error | draft | doc | Every adverse Outcome links a NoticeRequirement and AppealRight. |
| `WF-LINT-002` | error | draft | workspace | Every AppealRight has an appeal branch. |
| `WF-LINT-003` | error | draft | doc | Every Deadline has TimerMapping or reviewObligation. |
| `WF-LINT-004` | error | draft | doc | DecisionRule inputs collected before rule fires. |
| `WF-LINT-005` | error | draft | doc | Every actor has documented authority. |
| `WF-LINT-006` | error | draft | doc | Sensitive DataElements have retention policy. |
| `WF-LINT-007` | error | draft | doc | Every required EvidenceRequirement has collection step. |
| `WF-LINT-008` | error | draft | doc | Every workflow step has derivedFrom citation chain. |
| `EQ-LINT-001` | error | draft | doc | Rights-impacting workflows declare ≥3 ProtectedCategories. |

## S5 — Scenario readiness

| ID | Severity | Graduation | Surface | Summary |
|---|---|---|---|---|
| `SC-LINT-001` | error | draft | workspace | Every adverse Outcome MUST have a Scenario. |
| `SC-LINT-002` | error | draft | workspace | Every AppealRight MUST have a Scenario exercising the appeal. |
| `SC-LINT-003` | error | draft | doc | Every Scenario carries expectedOutcome / expectedTrace. |
| `SC-LINT-004` | error | draft | doc | Failing Scenarios MUST be acceptedAsKnownGap or waived. |
| `SC-LINT-005` | error | draft | workspace | Supersession-affected Scenarios MUST re-run. |
| `EQ-LINT-003` | error | draft | workspace | ≥1 equity-probe Scenario per ProtectedCategory. |
| `ACC-LINT-001` | error | draft | workspace | ≥1 accessibility-check Scenario per locale. |
| `JUR-LINT-001` | error | draft | workspace | ≥1 jurisdictional-variation Scenario per jurisdiction. |

## S6 — Publication readiness

| ID | Severity | Graduation | Surface | Summary |
|---|---|---|---|---|
| `PUB-LINT-001` | error | draft | workspace | No error/block findings unresolved at publication. |
| `PUB-LINT-002` | error | draft | workspace | Every required reviewer role has ApprovalDecision. |
| `PUB-LINT-003` | error | draft | external-gate | Compiled $wosWorkflow passes wos-workflow.schema.json. |
| `PUB-LINT-004` | error | draft | external-gate | Compiled artifact passes wos-lint. |
| `PUB-LINT-005` | error | draft | workspace | Approval package contains all required artifacts. |
| `PUB-LINT-006` | error | draft | workspace | Every unmappedButApproved Mapping listed in release notes. |
| `PUB-LINT-007` | error | draft | external-gate | Emitted scenarios pass wos-conformance. |
| `ID-LINT-001` | warning | draft | workspace | IdP role unmapped to workspace ReviewerRole. |
| `ID-LINT-002` | error | draft | workspace | Required-publication approver revoked. |
| `ID-LINT-003` | error | draft | doc | attestationLevel insufficient for action attempted. |
| `COMP-LINT-001` | error | draft | workspace | Workflow does not satisfy required compliance baseline controls. |
| `COMP-LINT-002` | warning | draft | workspace | Compliance attestation expiring (<90 days). |
| `CHAIN-LINT-001` | error | draft | doc | AuthoringProvenanceRecord chain integrity broken. |
| `CHAIN-LINT-002` | warning | draft | workspace | Audit log not anchored within configured cadence. |
| `EFF-LINT-005` | warning | draft | workspace | Effectiveness sunsetting in <90 days. |
| `AI-LINT-003` | error | draft | workspace | Agent-typed actor lacks an agent-fallback Scenario. |
| `CMP-LINT-010` | warning | draft | workspace | wos-version-deprecation pending (<90 days). |
| `CMP-LINT-011` | error | draft | workspace | wos-version-deprecation effective; migration required. |

## Status snapshot (2026-05-02, post-F5)

- 111 rules registered + dispatched (was 75 pre-I-wave; +35 across I-wave Phase A: SV-LINT-007..014, BIND-LINT-001..006/010..072, WF-LINT-009..013, MAP-LINT-009..011, RA-LINT-001..002, PROV-LINT-005..007; +1 ID-LINT-004 from J3 review-fixup).
- ~120 unit tests across rule modules + workspace rules (was 95 pre-G/E8/E11; +13 from G3 negative tests, +3 from G1 K-016 sentinels, +6 from E8.3 RetentionPolicy tests, +4 from E8.4 WF-LINT-006 shape-aware tests, +8 from E11.1 POM-LINT tests).
- **70 of 75 rules at `Tested` graduation** (F5.4 default flip; the 5 added in E8.4 + E11.1 land at `Draft` until they accrue ≥1 fixture each — already done for the 4 POM-LINT rules; SA-WARN-pom-MIGRATE-RETENTION uses the legacy advisory fixture). `Tested → Stable` flips await F4.3 conformance replay.

PUB-LINT-003 / PUB-LINT-004 / PUB-LINT-007 carry `external-gate` surface — they're lifted from the compiler's three external gates (per R4.4) and fire when the compiler runs.
