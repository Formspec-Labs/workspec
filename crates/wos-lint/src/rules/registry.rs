// Rust guideline compliant 2026-02-21

//! Rule metadata registry for all currently-implemented lint rules.
//!
//! The registry is the single source of truth for "what rules exist in code"
//! — separate from [`LINT-MATRIX.md`], which is the source of truth for
//! "what rules the spec normatively requires." Gaps between the two are
//! rule-coverage gaps.
//!
//! Every rule currently starts at [`Graduation::Draft`] with an empty
//! [`RuleMetadata::fixtures`] list. Promotion through the ladder
//! (Draft → Tested → Stable → LoadBearing) is handled by follow-up work
//! described in `thoughts/plans/2026-04-16-wos-rule-coverage-conformance.md`.
//!
//! # Adding a new rule
//!
//! When introducing a new lint diagnostic, add a [`RuleMetadata`] entry to
//! [`ALL_LINT_RULES`] with `graduation: Graduation::Draft` and an empty
//! `fixtures` slice. Promotion is performed in a separate change once the
//! rule has at least one passing fixture.

use crate::diagnostic::Severity;

/// Verification tier a rule belongs to.
///
/// See [`LINT-MATRIX.md`] for the authoritative tier catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Single-document structural checks (`wos-lint`).
    T1,
    /// Cross-document resolution and FEL AST analysis (`wos-lint --project`).
    T2,
    /// Dynamic runtime conformance (`wos-conformance`).
    T3,
}

/// Maturity level of a rule's fixture coverage.
///
/// The ladder goes `Draft → Tested → Stable → LoadBearing`. Rules promote
/// monotonically. See the rule-coverage plan for the promotion criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Graduation {
    /// No passing fixture exercises this rule yet.
    Draft,
    /// At least one passing fixture exercises this rule.
    Tested,
    /// `Tested`, plus unchanged across 3+ consecutive releases.
    Stable,
    /// Removing the rule would break at least one reference-implementation
    /// test. Requires spec_ref + suggested_fix + fixtures + ratchet check.
    LoadBearing,
}

/// Static metadata describing one lint rule.
///
/// Metadata is compile-time — all fields are `'static`. `spec_ref` and
/// `suggested_fix` are `Option` because only `LoadBearing` rules are
/// required to supply them.
#[derive(Debug, Clone, Copy)]
pub struct RuleMetadata {
    /// Rule identifier (e.g. `"K-001"`).
    pub id: &'static str,
    /// Verification tier.
    pub tier: Tier,
    /// Severity of the primary diagnostic this rule emits.
    pub severity: Severity,
    /// One-line summary of what the rule checks.
    pub summary: &'static str,
    /// Fixture paths (relative to `fixtures/`) that exercise this rule.
    pub fixtures: &'static [&'static str],
    /// Graduation ladder state.
    pub graduation: Graduation,
    /// Canonical spec reference (e.g. `"kernel/spec.md#§5.3"`), if recorded.
    pub spec_ref: Option<&'static str>,
    /// Imperative remediation hint, if recorded.
    pub suggested_fix: Option<&'static str>,
}

/// Return the full static registry of currently-implemented lint rules.
///
/// Ordering is by rule id to keep diffs readable; downstream code should
/// not rely on the ordering.
pub fn all_lint_rules() -> &'static [RuleMetadata] {
    ALL_LINT_RULES
}

/// Every lint rule currently implemented in `wos-lint`.
///
/// Entries are derived from the concrete `Diagnostic::{error,warning,info}`
/// call sites in `tier1.rs`, `tier2.rs`, `fel_analysis.rs`, and
/// `schema_doc.rs`. Rules listed in `LINT-MATRIX.md` but not yet emitted by
/// code are intentionally absent — the registry describes present reality,
/// not the normative catalog.
static ALL_LINT_RULES: &[RuleMetadata] = &[
    // --- AG (Advanced Governance) -------------------------------------
    RuleMetadata {
        id: "AG-008",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Side-effect tools at `autonomous` autonomy MUST declare a `sideEffectPolicy`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-010",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Verifiable constraints MUST satisfy all SMT subset restrictions (parse failures).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-011",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "`let` bindings in verifiable expressions MUST NOT be recursive.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-012",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Quantifiers MUST quantify over finite domains (non-standard every/some arity).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-013",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Verifiable arithmetic MUST be linear (no variable*variable products).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-014",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Verifiable subset MUST NOT include extension function calls.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-017",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Shadow mode is RECOMMENDED for rights-impacting workflows.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- AI (AI Integration) -------------------------------------------
    RuleMetadata {
        id: "AI-007",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Cascading autonomous agents MUST be declared via `cascadingInvocations`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-018",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "`autonomous` actions MUST have associated deontic constraints.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-020",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "`supervisory` actions MUST define `reviewWindow`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-023",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Every agent invocation MUST have a reachable path to completion without any agent.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-024",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Escalation conditions MUST be valid FEL referencing `@agent` context.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-026",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Escalation MUST have `escalationExpiry`; agent reverts when expired.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-031",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Agent output contract MUST apply same rules as human-facing form.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // AI-041: Tested via inline JSON in the `ai041_*` unit tests in
    // crates/wos-lint/tests/tier1_rules.rs (e.g.
    // `ai041_fallback_chain_without_terminal_action_flagged`).
    // The linked T3 fixture exists but is intentionally excluded from the
    // conformance trace-parity harness (see the doc comment in
    // crates/wos-conformance/tests/trace_parity.rs — the fixture has no
    // `kernel` document and is lint-only), so the evidence is indirect.
    // (See 2026-04-18 review.)
    RuleMetadata {
        id: "AI-041",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Fallback chain MUST terminate in `escalateToHuman` or `fail`; MUST NOT cycle.",
        fixtures: &["crates/wos-conformance/fixtures/AI-041-negative-fallback-cycle.json"],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-042",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Agent config MUST disclose training data characteristics.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-043",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Agent config MUST disclose optimization objective.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-046",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "`rights-impacting` workflows MUST have `discloseThatAgentAssisted: true`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-049",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Narrative records MUST have `authoritative: false`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-056",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Autonomy is an action-site property, not an agent property.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- CM (Correspondence Metadata) ---------------------------------
    RuleMetadata {
        id: "CM-001",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Entry template `id` values MUST be unique within the sidecar.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- DM (Drift Monitor) -------------------------------------------
    RuleMetadata {
        id: "DM-002",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Rights/safety workflows SHOULD follow shadow/canary/production sequence.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- G (Governance) -----------------------------------------------
    RuleMetadata {
        id: "G-001",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Due process MUST be enforced for `rights-impacting` or `safety-impacting` kernels.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-003",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Notice MUST include specific determination, reason codes, and appeal instructions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-004",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Explanation level MUST be `individualized` when kernel impact is `rights-impacting`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-005",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Adverse decisions MUST include positive and negative counterfactuals when rights-impacting.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-008",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "`continuationOfServices: true` requires kernel topology to freeze adverse impacts.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-009",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Transitions tagged `adverse-decision` MUST trigger due process policy enforcement.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-011",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Review protocol tags MUST match tags declared in the target kernel.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-014",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Reasoning tier MUST be present for `determination`-tagged transitions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-015",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Counterfactual tier MUST be present for `adverse-decision` transitions in rights-impacting workflows.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-022",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "`excludedOwner` MUST override `potentialOwner` when actor appears in both.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-023",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "SLA evaluation SHOULD use business calendar when BC sidecar is present.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-024",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Determination-tagged transitions MUST verify the actor has valid delegation.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-027",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Sub-delegation MUST respect `maxDelegationDepth`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-028",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Hold policies MUST attach to kernel states tagged `hold`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-029",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "`resumeTrigger` event name MUST reference an event in the target kernel.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-031",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "`resolutionDateRef` MUST refer to a field path in the kernel's case state.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-033",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Parameter `values` SHOULD cover every resolution date (no coverage gap).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-034",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "`targetWorkflow` MUST match the `url` of the target kernel document.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-035",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "`targetGovernance` MUST reference a valid governance document.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-036",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "`independenceConstraint` MUST describe a mechanism preventing self-review.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-037",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Assertion `id` values MUST be unique within the library.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-038",
        tier: Tier::T1,
        severity: Severity::Warning,
        summary: "Assertions of type `arithmetic`/`range`/`temporal` SHOULD include `expression`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-039",
        tier: Tier::T1,
        severity: Severity::Warning,
        summary: "Assertions of type `source-grounded`/`consistency` SHOULD include `fields`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-040",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "`consistency` assertions `referenceStage` MUST refer to an earlier pipeline stage.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-041",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Pipeline-stage assertion ids MUST exist in the targeted assertion library.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-042",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "FEL expressions in assertion `expression` fields MUST be syntactically valid.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-043",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "FEL expressions in delegation `conditions` MUST be syntactically valid.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-044",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Delegation `expirationDate` MUST be strictly after `effectiveDate`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-045",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Delegation `revokedDate` MUST be on or after `effectiveDate`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-046",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Delegation `delegator`/`delegate` MUST reference declared kernel actors.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-047",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Parameter `values` entries MUST be in ascending `effectiveDate` order.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-048",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Binding `id` MUST match the key under which it appears in the `bindings` map.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-050",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Resolved parameter value MUST be type-consistent with declared `type`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-053",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Sub-delegation is only permitted if the original delegation explicitly allows it.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-055",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "`expectedDuration` MUST be an ISO 8601 duration or the literal `\"indefinite\"`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-056",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "Binding `resolutionDateRef` MUST reference a field path in kernel case state.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-057",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Binding `values` entries MUST be in ascending `effectiveDate` order.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-058",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Each Holiday entry MUST specify exactly one of `date` or `rule`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-059",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Operating hours `end` MUST be strictly after `start`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-060",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Business Calendar target requires SLA evaluation in business days.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-062",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Adverse-decision templates MUST cover determination, reasons, rights, and instructions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-063",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Notification template refs MUST resolve to a template in a targeting sidecar.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-065",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Notification template section `id` values MUST be unique within a template.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- I (Integration Profile) --------------------------------------
    RuleMetadata {
        id: "I-001",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "`outputBinding` JSONPath MUST NOT use filter expressions or recursive descent.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- K (Kernel + Lifecycle Detail + Correspondence Metadata) -----
    RuleMetadata {
        id: "K-001",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Final states MUST NOT have outgoing transitions.",
        fixtures: &["crates/wos-conformance/fixtures/K-001-negative-final-transitions.json"],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-002",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Compound states MUST have `initialState` and `states`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-003",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Parallel states MUST have `regions`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-004",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "`cancellationPolicy` MUST only appear on `parallel` states.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-005",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "`historyState` MUST only appear on `compound` states.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-006",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Transition `target` MUST reference an existing state.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-007",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Event names MUST NOT use the `$` prefix (kernel-reserved).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-008",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Parallel state outgoing transitions MUST use `$join` as event.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-009",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Actor identifiers MUST be unique.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-010",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "createTask `assignTo` MUST reference a declared kernel actor.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-012",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Guards MUST be valid FEL expressions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-013",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Milestone conditions MUST be valid FEL expressions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-014",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Milestone `id` values MUST be unique.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-015",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "`setData` path MUST reference a declared `caseFile.fields` entry.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-017",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "FEL guards MUST NOT reference related case state.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-019",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "FEL functions MUST be declared built-ins or registered extensions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-021",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Provenance `actorId` MUST reference a declared actor.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-022",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Digest present implies algorithm recorded in extensions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-029",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "`startTimer` MUST specify exactly one of `duration` or `deadline`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-030",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Extension keys MUST be `x-` prefixed.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-037",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "Fail-fast `$join` fires only on an error final state.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-048",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Non-standard case relationship `type` values MUST use `x-` prefix.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // K-EXT-002: Tested via inline JSON in the `k_ext_002_*` unit tests in
    // crates/wos-lint/src/rules/tier2.rs (e.g. `k_ext_002_root_level_x_wos_key_flagged`).
    // The two linked `fixtures/validation/x-wos-*.json` files are authoring
    // artifacts, not executed by the conformance harness. (See 2026-04-18 review.)
    RuleMetadata {
        id: "K-EXT-002",
        tier: Tier::T2,
        severity: Severity::Warning,
        summary: "`x-wos-*` namespace is reserved for future normative WOS use.",
        fixtures: &[
            "fixtures/validation/x-wos-reserved-warn.json",
            "fixtures/validation/x-vendor-custom-ok.json",
        ],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- SCHEMA-DOC (schema documentation coverage) -------------------
    RuleMetadata {
        id: "SCHEMA-DOC-001",
        tier: Tier::T1,
        severity: Severity::Error,
        summary: "Schema leaf properties MUST carry sufficient `description` and `examples`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- VR (Verification Report) -------------------------------------
    RuleMetadata {
        id: "VR-003",
        tier: Tier::T2,
        severity: Severity::Error,
        summary: "`counterexample` MUST be present when result is `proven-unsafe`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for rule in ALL_LINT_RULES {
            assert!(
                seen.insert(rule.id),
                "duplicate rule id in registry: {}",
                rule.id
            );
        }
    }

    #[test]
    fn registry_ids_are_sorted() {
        let ids: Vec<&str> = ALL_LINT_RULES.iter().map(|r| r.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "registry entries must be sorted by id");
    }
}
