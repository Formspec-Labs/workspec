// Rust guideline compliant 2026-05-02

//! Studio-tier rule registry.

use crate::LintSeverity;

/// Studio tier per `studio/specs/readiness-validation.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StudioTier {
    /// Source vault rules (SV-LINT-*)
    S1,
    /// Policy object readiness (POM/PROV/EFF/AI-LINT/EQ/TERM)
    S2,
    /// Mapping readiness (MAP-LINT-*, EFF-LINT-004)
    S3,
    /// Workflow readiness (WF-LINT-*, EQ-LINT-001)
    S4,
    /// Scenario readiness (SC-LINT-*, EQ/ACC/JUR-LINT)
    S5,
    /// Publication readiness (PUB-LINT-*, ID/COMP/CHAIN/...)
    S6,
}

/// Graduation status of a Studio rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioGraduation {
    Draft,
    Tested,
    Stable,
    LoadBearing,
}

#[derive(Debug, Clone, Copy)]
pub struct StudioRule {
    pub id: &'static str,
    pub studio_tier: StudioTier,
    pub severity: LintSeverity,
    pub summary: &'static str,
    pub graduation: StudioGraduation,
    pub spec_ref: Option<&'static str>,
}

const fn rule(
    id: &'static str,
    tier: StudioTier,
    severity: LintSeverity,
    summary: &'static str,
    spec: &'static str,
) -> StudioRule {
    // F5.4 (2026-05-02): default graduation flipped from `Draft` to
    // `Tested` after inline tests landed for every rule (F5.1 + F5.3).
    // The graduation ladder per STUDIO-LINT-MATRIX.md is:
    //   Draft        — registered but unverified
    //   Tested       — has ≥1 positive + ≥1 negative test
    //   Stable       — fired in SNAP-slice conformance run + 3-day
    //                  parser stability + no negative-fixture
    //                  regressions (requires F4.3 conformance crate)
    //   LoadBearing  — production fire history
    // To register a not-yet-tested rule, use [`draft_rule`] (preserved
    // for future surface).
    StudioRule {
        id,
        studio_tier: tier,
        severity,
        summary,
        graduation: StudioGraduation::Tested,
        spec_ref: Some(spec),
    }
}

/// Register a rule that does NOT yet carry executable tests.
/// Reserved for future surface; every rule today is at `Tested`
/// after F5.4.
#[allow(dead_code)]
const fn draft_rule(
    id: &'static str,
    tier: StudioTier,
    severity: LintSeverity,
    summary: &'static str,
    spec: &'static str,
) -> StudioRule {
    StudioRule {
        id,
        studio_tier: tier,
        severity,
        summary,
        graduation: StudioGraduation::Draft,
        spec_ref: Some(spec),
    }
}

/// Every Studio readiness rule the engine knows about. 70 distinct
/// rule IDs across S1..S6 per `specs/readiness-validation.md` (was 69
/// pre-F5.6; +1 from `POM-LINT-DPV-001`). The catalog count is
/// asserted in the `registry_carries_at_least_seventy_rules` test
/// below.
pub fn all_studio_rules() -> &'static [StudioRule] {
    STUDIO_RULES
}

const STUDIO_RULES: &[StudioRule] = &[
        // ----- S1: Source vault -----
        rule(
            "SV-LINT-001",
            StudioTier::S1,
            LintSeverity::Error,
            "Every SourceCitation MUST resolve to a real SourceSection.",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-002",
            StudioTier::S1,
            LintSeverity::Error,
            "Citation excerpts MUST appear in the referenced SourceSection.",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-003",
            StudioTier::S1,
            LintSeverity::Error,
            "No PolicyObject relies solely on disputed/superseded SourceVersions.",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-004",
            StudioTier::S1,
            LintSeverity::Error,
            "Current SourceVersions MUST carry effectiveStart.",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-005",
            StudioTier::S1,
            LintSeverity::Error,
            "Section anchors MUST be unique within a SourceVersion.",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-006",
            StudioTier::S1,
            LintSeverity::Error,
            "Low-confidence ExtractedClaims (<0.5) MUST NOT be auto-approved.",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-007",
            StudioTier::S1,
            LintSeverity::Error,
            "PolicyObject citations MUST NOT target versionless SourceDocuments (per SA-MUST-source-001).",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-008",
            StudioTier::S1,
            LintSeverity::Error,
            "SourceVersion at lifecycleState current/preliminary/disputed MUST carry parsingResult (per SA-MUST-source-002 — tractable lint slice of the temporal progression invariant).",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-009",
            StudioTier::S1,
            LintSeverity::Error,
            "SourceVersion past uploaded MUST have parsingResult.status='ok' OR ('partial' WITH parsingWaiverRef) (per SA-MUST-source-003).",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-010",
            StudioTier::S1,
            LintSeverity::Error,
            "At most one SourceVersion per SourceDocument MAY hold lifecycleState='current' (per SA-MUST-source-005).",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-011",
            StudioTier::S1,
            LintSeverity::Error,
            "When SourceVersion.pageable=true, every SourceSection in that version MUST carry pageRange (per SA-MUST-source-011).",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-012",
            StudioTier::S1,
            LintSeverity::Error,
            "JSON-LD @context drift across consecutive json-ld SourceVersions surfaces a tier-S1 finding (per SA-MUST-source-052).",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-013",
            StudioTier::S1,
            LintSeverity::Error,
            "akoma-ntoso SourceVersions MUST extract <FRBRdate> into effectiveStart (per SA-MUST-source-081).",
            "specs/source-vault.md",
        ),
        rule(
            "SV-LINT-014",
            StudioTier::S1,
            LintSeverity::Error,
            "Multilingual SourceVersions MUST carry text in the authoritative (first) locale on every SourceSection (per SA-MUST-source-060).",
            "specs/source-vault.md",
        ),
        // ----- S3: Binding readiness (BIND-LINT family; new in I-A2) -----
        rule(
            "BIND-LINT-001",
            StudioTier::S3,
            LintSeverity::Error,
            "Bindings carrying ^x- extension keys MUST have a matching workspace.policy.extensionRegistry entry (per SA-MUST-bind-001).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-002",
            StudioTier::S3,
            LintSeverity::Error,
            "Bindings MUST NOT invent new seams; six canonical kernel seams per ADR-0077 (per SA-MUST-bind-002).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-003",
            StudioTier::S3,
            LintSeverity::Error,
            "ServiceBinding inputBindings[].caseFilePath MUST resolve to a workspace CaseFileMapping/DataElement (per SA-MUST-bind-011).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-004",
            StudioTier::S3,
            LintSeverity::Error,
            "ServiceBinding outputBindings[].target MUST resolve to a workspace object OR carry ignoredRationale (per SA-MUST-bind-012).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-005",
            StudioTier::S3,
            LintSeverity::Error,
            "ServiceBindings touching sensitive DataElements MUST carry body.sensitivityHandling OR a sensitivityWaiverRef (per SA-MUST-bind-013).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-006",
            StudioTier::S3,
            LintSeverity::Error,
            "ServiceBindings MUST declare body.errorHandling.onError ∈ {retry, fallback, fail-workflow, alert} (per SA-MUST-bind-014).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-010",
            StudioTier::S3,
            LintSeverity::Error,
            "EventBinding direction=consumed MUST identify body.source (per SA-MUST-bind-021).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-011",
            StudioTier::S3,
            LintSeverity::Error,
            "EventBinding direction=emitted MUST identify body.recipient (per SA-MUST-bind-022).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-012",
            StudioTier::S3,
            LintSeverity::Error,
            "EventBinding payloadShape fields with sensitive sensitivity MUST carry a redactionRules entry (per SA-MUST-bind-024).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-020",
            StudioTier::S3,
            LintSeverity::Error,
            "PolicyEngineBinding MUST declare non-empty body.inputContract.caseFilePaths (per SA-MUST-bind-032; tractable slice).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-021",
            StudioTier::S3,
            LintSeverity::Error,
            "PolicyEngineBinding declared engineReasonCodes[] MUST all have a reasonsMapping entry (per SA-MUST-bind-033).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-070",
            StudioTier::S5,
            LintSeverity::Error,
            "Bindings at lifecycleState ≥ approved MUST have ≥ 1 Scenario in exercisedByScenarios[] (per SA-MUST-bind-070).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-071",
            StudioTier::S5,
            LintSeverity::Error,
            "ServiceBinding with errorHandling.onError ≠ fail-workflow MUST have ≥ 2 Scenarios (happy + error path) (per SA-MUST-bind-071).",
            "specs/binding-and-integration.md",
        ),
        rule(
            "BIND-LINT-072",
            StudioTier::S5,
            LintSeverity::Error,
            "PolicyEngineBinding MUST have Scenarios covering both permit and deny outcomes (per SA-MUST-bind-072).",
            "specs/binding-and-integration.md",
        ),
        // ----- S4: WorkflowIntent extension cluster (I-A6) -----
        rule(
            "WF-LINT-009",
            StudioTier::S4,
            LintSeverity::Error,
            "WorkflowIntent element ids MUST be unique (per SA-MUST-wfi-003).",
            "specs/workflow-intent.md",
        ),
        rule(
            "WF-LINT-010",
            StudioTier::S4,
            LintSeverity::Error,
            "Element position references (phase.contains[*], exception.divertsFrom, manual-override.defaultPath) MUST resolve within the same WorkflowIntent (per SA-MUST-wfi-004).",
            "specs/workflow-intent.md",
        ),
        rule(
            "WF-LINT-011",
            StudioTier::S4,
            LintSeverity::Error,
            "Notice element MUST reference an approved+mapped NoticeRequirement (per SA-MUST-wfi-011).",
            "specs/workflow-intent.md",
        ),
        rule(
            "WF-LINT-012",
            StudioTier::S4,
            LintSeverity::Error,
            "Appeal element MUST carry appealRightRef referencing an existing AppealRight (per SA-MUST-wfi-012).",
            "specs/workflow-intent.md",
        ),
        rule(
            "WF-LINT-013",
            StudioTier::S4,
            LintSeverity::Error,
            "system-check element MUST carry serviceBindingRef referencing an existing ServiceBinding (per SA-MUST-wfi-013).",
            "specs/workflow-intent.md",
        ),
        // ----- S3: Mapping cluster (I-A7) -----
        rule(
            "MAP-LINT-009",
            StudioTier::S3,
            LintSeverity::Error,
            "WorkflowIntent at validationReady or beyond MUST NOT reference unmappedButApproved PolicyObjects without unmappedAcceptanceRef (per SA-MUST-map-004).",
            "specs/studio-to-wos-mapping.md",
        ),
        rule(
            "MAP-LINT-010",
            StudioTier::S3,
            LintSeverity::Error,
            "WorkflowIntent at scenarioTested or beyond MUST NOT reference PolicyObjects whose requiresSpecExtension ExtensionRecord is open (per SA-MUST-map-005).",
            "specs/studio-to-wos-mapping.md",
        ),
        rule(
            "MAP-LINT-011",
            StudioTier::S3,
            LintSeverity::Error,
            "ExtensionRecord MUST carry ≥ 1 motivatingPolicyObjectRef (per SA-MUST-map-021).",
            "specs/studio-to-wos-mapping.md",
        ),
        // ----- S6: Review-and-Approval cluster (I-A7) -----
        rule(
            "RA-LINT-001",
            StudioTier::S6,
            LintSeverity::Error,
            "ApprovalDecision reviewerRole MUST resolve to a workspace-defined role (per SA-MUST-ra-002).",
            "specs/review-and-approval.md",
        ),
        rule(
            "RA-LINT-002",
            StudioTier::S6,
            LintSeverity::Error,
            "ReviewerComment subjectRef MUST resolve to a workspace object (per SA-MUST-ra-012).",
            "specs/review-and-approval.md",
        ),
        // ----- S2: Provenance cluster (I-A8) -----
        rule(
            "PROV-LINT-005",
            StudioTier::S2,
            LintSeverity::Error,
            "AuthoringProvenanceRecord parentRecordIds[] MUST resolve within the same workspace (per SA-MUST-prov-005).",
            "specs/authoring-provenance.md",
        ),
        rule(
            "PROV-LINT-006",
            StudioTier::S2,
            LintSeverity::Error,
            "WorkflowIntent elements at reviewState=approved or beyond MUST carry originClass (per SA-MUST-prov-010).",
            "specs/authoring-provenance.md",
        ),
        rule(
            "PROV-LINT-007",
            StudioTier::S2,
            LintSeverity::Error,
            "PolicyObject originClass=approved-interpretation MUST carry ≥ 1 citation AND ≥ 1 reviewerResolution (per SA-MUST-prov-012).",
            "specs/authoring-provenance.md",
        ),
        // ----- S2: Policy-object readiness -----
        rule(
            "POM-LINT-001",
            StudioTier::S2,
            LintSeverity::Error,
            "Approved PolicyObject MUST carry citation or basis-assumption.",
            "specs/policy-object-model.md",
        ),
        rule(
            "POM-LINT-002",
            StudioTier::S2,
            LintSeverity::Error,
            "originClass=approved-interpretation MUST carry reviewerResolution.",
            "specs/policy-object-model.md",
        ),
        rule(
            "POM-LINT-003",
            StudioTier::S2,
            LintSeverity::Error,
            "Approved PolicyObject MUST declare an originClass.",
            "specs/policy-object-model.md",
        ),
        rule(
            "POM-LINT-007",
            StudioTier::S2,
            LintSeverity::Error,
            "No circular Supersession chains.",
            "specs/policy-object-model.md",
        ),
        rule(
            "POM-LINT-008",
            StudioTier::S2,
            LintSeverity::Error,
            "Every Conflict MUST be resolved or waived before downstream advance.",
            "specs/policy-object-model.md",
        ),
        rule(
            "POM-LINT-020",
            StudioTier::S2,
            LintSeverity::Error,
            "PolicyObjects at lifecycleState=approved (or downstream) MUST be covered by an ApprovalDecision with matching subjectRef (per SA-MUST-pom-020 / CM §1.15).",
            "specs/policy-object-model.md",
        ),
        rule(
            "POM-LINT-033",
            StudioTier::S4,
            LintSeverity::Error,
            "AppealRight.outcomeRef MUST equal the linked NoticeRequirement's outcomeRef (per SA-MUST-pom-033). Waiver path: body.waiverScope='separate-procedure' + body.waivedAt silences the rule.",
            "specs/policy-object-model.md",
        ),
        rule(
            "POM-LINT-040",
            StudioTier::S2,
            LintSeverity::Error,
            "Two approved Deadlines on the same trigger with different calendarDaysFromTrigger MUST be filed as a Conflict naming both subjects (per SA-MUST-pom-040; tractable lint-time slice of the broader contradiction-detection contract).",
            "specs/policy-object-model.md",
        ),
        rule(
            "POM-LINT-051",
            StudioTier::S2,
            LintSeverity::Warning,
            "Two deontic constraints (Permission/Prohibition/Obligation) sharing (subject, action) flagged as composition candidates unless body.compositionAttestation='reviewed' is recorded on at least one (per SA-MUST-pom-051).",
            "specs/policy-object-model.md",
        ),
        rule(
            "POM-LINT-DPV-001",
            StudioTier::S2,
            LintSeverity::Error,
            "DataElement carrying dpvSensitivity MUST also carry canonicalTermRef.",
            "specs/policy-object-model.md",
        ),
        rule(
            "PROV-LINT-002",
            StudioTier::S2,
            LintSeverity::Error,
            "Provenance chain MUST resolve to citation/assumption/attestation.",
            "specs/authoring-provenance.md",
        ),
        rule(
            "PROV-LINT-003",
            StudioTier::S2,
            LintSeverity::Error,
            "originClass=approved-interpretation MUST carry ReviewerResolution.",
            "specs/authoring-provenance.md",
        ),
        rule(
            "PROV-LINT-004",
            StudioTier::S2,
            LintSeverity::Error,
            "originClass=local-practice MUST carry an attestation.",
            "specs/authoring-provenance.md",
        ),
        rule(
            "EFF-LINT-001",
            StudioTier::S2,
            LintSeverity::Warning,
            "Redundant effectiveness duplicate (inline+ref).",
            "specs/effectiveness-and-applicability.md",
        ),
        rule(
            "EFF-LINT-002",
            StudioTier::S2,
            LintSeverity::Error,
            "Effectiveness widening disallowed (object widens source scope).",
            "specs/effectiveness-and-applicability.md",
        ),
        rule(
            "EFF-LINT-003",
            StudioTier::S2,
            LintSeverity::Error,
            "enjoined=true MUST carry enjoinedScope.",
            "specs/effectiveness-and-applicability.md",
        ),
        rule(
            "AI-LINT-001",
            StudioTier::S2,
            LintSeverity::Error,
            "AI-extracted PolicyObject MUST carry aiLineage block.",
            "specs/authoring-provenance.md",
        ),
        rule(
            "AI-LINT-002",
            StudioTier::S2,
            LintSeverity::Error,
            "AI-extracted PolicyObject promoted past extracted MUST have humanApprover.",
            "specs/authoring-provenance.md",
        ),
        rule(
            "EQ-LINT-002",
            StudioTier::S2,
            LintSeverity::Error,
            "Every ProtectedCategory MUST cite legalBasis.",
            "specs/policy-object-model.md",
        ),
        rule(
            "TERM-LINT-001",
            StudioTier::S2,
            LintSeverity::Error,
            "TerminologyMap entry MUST NOT point to a deprecated CanonicalTerm.",
            "specs/terminology-and-canonical-vocabulary.md",
        ),
        rule(
            "TERM-LINT-002",
            StudioTier::S2,
            LintSeverity::Warning,
            "DataElement canonicalTermRef=manual-pending awaits attestation.",
            "specs/terminology-and-canonical-vocabulary.md",
        ),
        rule(
            "TERM-LINT-003",
            StudioTier::S2,
            LintSeverity::Warning,
            "DataElement uses legacy sensitivity alias.",
            "specs/terminology-and-canonical-vocabulary.md",
        ),
        // ----- S3: Mapping readiness -----
        rule(
            "MAP-LINT-001",
            StudioTier::S3,
            LintSeverity::Error,
            "Every approved PolicyObject MUST have a Mapping.",
            "specs/studio-to-wos-mapping.md",
        ),
        rule(
            "MAP-LINT-002",
            StudioTier::S3,
            LintSeverity::Error,
            "mapsToWos targets MUST carry wosConceptId + wosJsonPath.",
            "specs/studio-to-wos-mapping.md",
        ),
        rule(
            "MAP-LINT-003",
            StudioTier::S3,
            LintSeverity::Error,
            "requiresSpecExtension MUST carry substantive ExtensionRecord.",
            "specs/studio-to-wos-mapping.md",
        ),
        rule(
            "MAP-LINT-004",
            StudioTier::S3,
            LintSeverity::Warning,
            "unmappedButApproved MUST carry substantive rationale.",
            "specs/studio-to-wos-mapping.md",
        ),
        rule(
            "MAP-LINT-005",
            StudioTier::S3,
            LintSeverity::Error,
            "No two PolicyObjects collide on the same target.",
            "specs/studio-to-wos-mapping.md",
        ),
        rule(
            "MAP-LINT-006",
            StudioTier::S3,
            LintSeverity::Error,
            "Workflow-bearing PolicyObjects MUST NOT be unmappedButApproved without override.",
            "specs/studio-to-wos-mapping.md",
        ),
        rule(
            "MAP-LINT-007",
            StudioTier::S3,
            LintSeverity::Error,
            "Workflow-bearing PolicyObjects MUST NOT have an open ExtensionRecord blocking advance.",
            "specs/studio-to-wos-mapping.md",
        ),
        rule(
            "MAP-LINT-008",
            StudioTier::S3,
            LintSeverity::Error,
            "x- targets MUST carry an extension-registry entry.",
            "specs/studio-to-wos-mapping.md",
        ),
        rule(
            "EFF-LINT-004",
            StudioTier::S3,
            LintSeverity::Warning,
            "Mapping effectiveness collision (overlapping conflicting scopes).",
            "specs/effectiveness-and-applicability.md",
        ),
        // ----- S4: Workflow readiness -----
        rule(
            "WF-LINT-001",
            StudioTier::S4,
            LintSeverity::Error,
            "Every adverse Outcome links a NoticeRequirement and AppealRight.",
            "specs/workflow-intent.md",
        ),
        rule(
            "WFI-SHAPE-001",
            StudioTier::S4,
            LintSeverity::Error,
            "WorkflowElement of kind=phase MUST carry a body block.",
            "specs/workflow-intent.md",
        ),
        rule(
            "WF-LINT-002",
            StudioTier::S4,
            LintSeverity::Error,
            "Every AppealRight has an appeal branch in the WorkflowIntent.",
            "specs/workflow-intent.md",
        ),
        rule(
            "WF-LINT-003",
            StudioTier::S4,
            LintSeverity::Error,
            "Every Deadline has a TimerMapping or explicit reviewObligation.",
            "specs/workflow-intent.md",
        ),
        rule(
            "WF-LINT-004",
            StudioTier::S4,
            LintSeverity::Error,
            "DecisionRule inputs collected before the rule fires.",
            "specs/workflow-intent.md",
        ),
        rule(
            "WF-LINT-005",
            StudioTier::S4,
            LintSeverity::Error,
            "Every actor has documented authority for every step.",
            "specs/workflow-intent.md",
        ),
        rule(
            "WF-LINT-006",
            StudioTier::S4,
            LintSeverity::Error,
            "Sensitive DataElements have a shape-valid RetentionPolicy on every collecting EvidenceRequirement (E8.4: ADR-0083 r2 closed shape; resolves per-EvidenceRequirement override OR workspace default keyed by DPV sensitivity).",
            "specs/policy-object-model.md",
        ),
        rule(
            "SA-WARN-pom-MIGRATE-RETENTION",
            StudioTier::S4,
            LintSeverity::Warning,
            "EvidenceRequirements still carrying the legacy singular `retentionPeriod` field. Migrate to typed `retentionPolicy` per ADR-0083 r2: lift the value into retentionPolicy.duration and add disposalAction. One-rev advisory; field removed in the next rev.",
            "specs/policy-object-model.md",
        ),
        rule(
            "WF-LINT-007",
            StudioTier::S4,
            LintSeverity::Error,
            "Every required EvidenceRequirement has a workflow collection step.",
            "specs/workflow-intent.md",
        ),
        rule(
            "WF-LINT-008",
            StudioTier::S4,
            LintSeverity::Error,
            "Every workflow step has a derivedFrom citation chain.",
            "specs/workflow-intent.md",
        ),
        rule(
            "EQ-LINT-001",
            StudioTier::S4,
            LintSeverity::Error,
            "Rights-impacting workflows MUST declare ≥3 ProtectedCategories.",
            "specs/policy-object-model.md",
        ),
        // ----- S5: Scenario readiness -----
        rule(
            "SC-LINT-001",
            StudioTier::S5,
            LintSeverity::Error,
            "Every adverse Outcome MUST have at least one Scenario.",
            "specs/scenario-authoring.md",
        ),
        rule(
            "SC-LINT-002",
            StudioTier::S5,
            LintSeverity::Error,
            "Every AppealRight MUST have a Scenario exercising the appeal branch.",
            "specs/scenario-authoring.md",
        ),
        rule(
            "SC-LINT-006",
            StudioTier::S5,
            LintSeverity::Warning,
            "Scenario SHOULD declare an explicit lifecycleState.",
            "specs/scenario-authoring.md",
        ),
        rule(
            "SC-LINT-003",
            StudioTier::S5,
            LintSeverity::Error,
            "Every Scenario carries expectedOutcome / expectedTrace.",
            "specs/scenario-authoring.md",
        ),
        rule(
            "SC-LINT-004",
            StudioTier::S5,
            LintSeverity::Error,
            "Failing Scenarios MUST be acceptedAsKnownGap or waived.",
            "specs/scenario-authoring.md",
        ),
        rule(
            "SC-LINT-005",
            StudioTier::S5,
            LintSeverity::Error,
            "Supersession-affected Scenarios MUST re-run before workflow advance.",
            "specs/scenario-authoring.md",
        ),
        rule(
            "EQ-LINT-003",
            StudioTier::S5,
            LintSeverity::Error,
            "Workflows declaring ProtectedCategory MUST have ≥1 equity-probe Scenario per dimension.",
            "specs/scenario-authoring.md",
        ),
        rule(
            "ACC-LINT-001",
            StudioTier::S5,
            LintSeverity::Error,
            "Notice-bearing workflows MUST have ≥1 accessibility-check Scenario per locale.",
            "specs/scenario-authoring.md",
        ),
        rule(
            "JUR-LINT-001",
            StudioTier::S5,
            LintSeverity::Error,
            "Multi-jurisdiction workflows MUST have ≥1 jurisdictional-variation Scenario per jurisdiction.",
            "specs/scenario-authoring.md",
        ),
        // ----- S6: Publication readiness -----
        rule(
            "PUB-LINT-001",
            StudioTier::S6,
            LintSeverity::Block,
            "No error/block findings remain unresolved at publication.",
            "specs/readiness-validation.md",
        ),
        rule(
            "PUB-LINT-002",
            StudioTier::S6,
            LintSeverity::Error,
            "Every required reviewer role has at least one ApprovalDecision.",
            "specs/review-and-approval.md",
        ),
        rule(
            "PUB-LINT-003",
            StudioTier::S6,
            LintSeverity::Block,
            "Compiled $wosWorkflow passes wos-workflow.schema.json (schema-pass gate).",
            "specs/compiler-contract.md",
        ),
        rule(
            "PUB-LINT-004",
            StudioTier::S6,
            LintSeverity::Block,
            "Compiled artifact passes wos-lint (lint-pass gate).",
            "specs/compiler-contract.md",
        ),
        rule(
            "PUB-LINT-005",
            StudioTier::S6,
            LintSeverity::Block,
            "Approval package contains all required artifacts.",
            "specs/compiler-contract.md",
        ),
        rule(
            "PUB-LINT-006",
            StudioTier::S6,
            LintSeverity::Error,
            "Every unmappedButApproved Mapping listed in release notes.",
            "specs/studio-to-wos-mapping.md",
        ),
        rule(
            "PUB-LINT-007",
            StudioTier::S6,
            LintSeverity::Block,
            "Emitted scenarios pass wos-conformance (conformance-pass gate).",
            "specs/compiler-contract.md",
        ),
        rule(
            "ID-LINT-001",
            StudioTier::S6,
            LintSeverity::Warning,
            "IdP role unmapped to workspace ReviewerRole.",
            "specs/identity-and-attestation.md",
        ),
        rule(
            "ID-LINT-002",
            StudioTier::S6,
            LintSeverity::Error,
            "Required-publication approver revoked before publication.",
            "specs/identity-and-attestation.md",
        ),
        rule(
            "ID-LINT-003",
            StudioTier::S6,
            LintSeverity::Error,
            "attestationLevel insufficient for action attempted.",
            "specs/identity-and-attestation.md",
        ),
        rule(
            "ID-LINT-004",
            StudioTier::S6,
            LintSeverity::Error,
            "IdentitySubject at lifecycleState ≥ approved MUST carry ≥ 1 temporally-valid activeAttestations[] entry (per ADR-0084 §2.2; SA-MUST-id-004).",
            "specs/identity-and-attestation.md",
        ),
        rule(
            "COMP-LINT-001",
            StudioTier::S6,
            LintSeverity::Error,
            "Workflow does not satisfy required compliance baseline controls.",
            "specs/workspace.md",
        ),
        rule(
            "COMP-LINT-002",
            StudioTier::S6,
            LintSeverity::Warning,
            "Compliance attestation expiring (<90 days).",
            "specs/workspace.md",
        ),
        rule(
            "CHAIN-LINT-001",
            StudioTier::S6,
            LintSeverity::Error,
            "AuthoringProvenanceRecord chain integrity broken.",
            "specs/authoring-provenance.md",
        ),
        rule(
            "CHAIN-LINT-002",
            StudioTier::S6,
            LintSeverity::Warning,
            "Workspace audit log not anchored within configured cadence.",
            "specs/authoring-provenance.md",
        ),
        rule(
            "EFF-LINT-005",
            StudioTier::S6,
            LintSeverity::Warning,
            "Sunset window: workflow depends on Effectiveness sunsetting in <90 days.",
            "specs/effectiveness-and-applicability.md",
        ),
        rule(
            "AI-LINT-003",
            StudioTier::S6,
            LintSeverity::Error,
            "Workflow with agent-typed actor lacks an agent-fallback Scenario.",
            "specs/scenario-authoring.md",
        ),
        rule(
            "CMP-LINT-010",
            StudioTier::S6,
            LintSeverity::Warning,
            "wos-version-deprecation pending (<90 days).",
            "specs/compiler-contract.md",
        ),
        rule(
            "CMP-LINT-011",
            StudioTier::S6,
            LintSeverity::Error,
            "wos-version-deprecation effective; migration required.",
            "specs/compiler-contract.md",
        ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_carries_at_least_seventy_rules() {
        let count = all_studio_rules().len();
        // Hard ratchet: the registry has 111 entries at HEAD
        // (110 + ID-LINT-004 from J3 — cardinality + temporal validity
        // per ADR-0084 §2.2). Adjust upward only when a new rule lands;
        // never downward without a deprecation cycle.
        assert_eq!(count, 111, "registry catalog count drifted; update J3 reconciliation");
    }

    #[test]
    fn rule_ids_unique() {
        let mut seen = std::collections::HashSet::new();
        for rule in all_studio_rules() {
            assert!(
                seen.insert(rule.id),
                "duplicate rule id: {}",
                rule.id
            );
        }
    }

    /// F5.4 graduation flip — every rule moved Draft → Tested after
    /// the F5.1+F5.3 test work. Stable / LoadBearing flips depend on
    /// F4.3 conformance replay + production fire history; deferred.
    #[test]
    fn every_rule_at_tested_or_better() {
        for rule in all_studio_rules() {
            assert!(
                !matches!(rule.graduation, StudioGraduation::Draft),
                "rule {} still at Draft graduation; F5.4 expected all rules to flip to Tested",
                rule.id
            );
        }
    }

    /// F5.5 severity audit — every rule's severity matches the spec
    /// defaults at `studio/specs/readiness-validation.md`:
    ///
    /// - S1 (lines ~90):  `error`
    /// - S2 (line ~105):  `error` (default), `warn` for some specific checks
    /// - S3 (line ~122):  `error` for collisions/missing,
    ///                    `warn` for `unmappedButApproved`
    /// - S4 (line ~139):  `error` (default), `warn` admitted for soft cases
    /// - S5 (line ~153):  `error` for critical, `warn` for soft gaps
    /// - S6 (line ~171):  `block` for publication-blockers,
    ///                    `error` for unresolved findings,
    ///                    `warn` for soft gaps
    ///
    /// This sentinel codifies the audit: every (tier, severity) pair
    /// the registry uses MUST be in the allowed-set per the spec.
    #[test]
    fn severity_matches_spec_defaults() {
        for rule in all_studio_rules() {
            let allowed: &[LintSeverity] = match rule.studio_tier {
                StudioTier::S1 => &[LintSeverity::Error],
                StudioTier::S2 | StudioTier::S3 | StudioTier::S4 | StudioTier::S5 => {
                    &[LintSeverity::Error, LintSeverity::Warning]
                }
                StudioTier::S6 => &[
                    LintSeverity::Block,
                    LintSeverity::Error,
                    LintSeverity::Warning,
                ],
            };
            assert!(
                allowed.contains(&rule.severity),
                "rule {} (tier {:?}) has severity {:?}; allowed for tier: {:?}",
                rule.id,
                rule.studio_tier,
                rule.severity,
                allowed
            );
        }
    }

    #[test]
    fn graduation_distribution_summary() {
        // Useful regression sentinel: count rules per graduation
        // tier so tooling consumers can verify a known shape.
        let mut at_tested = 0;
        let mut at_stable = 0;
        let mut at_load_bearing = 0;
        let mut at_draft = 0;
        for r in all_studio_rules() {
            match r.graduation {
                StudioGraduation::Draft => at_draft += 1,
                StudioGraduation::Tested => at_tested += 1,
                StudioGraduation::Stable => at_stable += 1,
                StudioGraduation::LoadBearing => at_load_bearing += 1,
            }
        }
        assert_eq!(at_draft, 0, "F5.4: zero rules at Draft");
        assert!(
            at_tested >= 70,
            "expected ≥70 rules at Tested (F5.6 inventory), got {at_tested}"
        );
        // Tighten when first rule graduates to Stable; the bound then
        // becomes `at_tested + at_stable >= 70`. F4.3 (real conformance
        // replay) is the natural promotion trigger.
        let _ = (at_stable, at_load_bearing);
    }

    /// `readiness-validation.md` § line 171 enumerates the S6
    /// publication-gate rules whose severity default is `block` —
    /// these MUST halt advancement to a published state regardless
    /// of waivers applied to lower severities.
    #[test]
    fn s6_publication_gate_rules_carry_block_severity() {
        const PUBLICATION_BLOCKERS: &[&str] = &[
            "PUB-LINT-001",
            "PUB-LINT-003",
            "PUB-LINT-004",
            "PUB-LINT-005",
            "PUB-LINT-007",
        ];
        for rule_id in PUBLICATION_BLOCKERS {
            let rule = all_studio_rules()
                .iter()
                .find(|r| r.id == *rule_id)
                .unwrap_or_else(|| panic!("missing publication-blocker rule {rule_id}"));
            assert_eq!(
                rule.severity,
                LintSeverity::Block,
                "{rule_id} must carry LintSeverity::Block per readiness-validation.md §6"
            );
            assert_eq!(
                rule.studio_tier,
                StudioTier::S6,
                "{rule_id} must remain in tier S6"
            );
        }
    }
}
