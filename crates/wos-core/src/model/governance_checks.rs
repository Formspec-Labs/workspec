// Rust guideline compliant 2026-02-21
//! Shared static governance declarations and authored lifecycle locations.
use super::due_process::{DueProcessExecution, TransitionLocation};
use super::governance::{
    AdverseDecisionPolicy, AuditConfig, CounterfactualTierConfig, ExplanationLevel,
    ReasoningTierConfig,
};
use super::kernel::{ImpactLevel, KernelDocument};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Semantic tags and exact transition locations from the authored lifecycle.
#[derive(Debug, Default)]
pub struct LifecycleIndex {
    /// Tags declared on states or transitions, for general reference checks.
    pub all_tags: HashSet<String>,
    /// Tags declared on transitions, for transition audit requirements.
    pub transition_tags: HashSet<String>,
    locations: HashMap<String, Vec<TransitionLocation>>,
}
impl LifecycleIndex {
    /// Index nested states, parallel regions, and ForEach bodies without executing them.
    pub fn new(kernel: &KernelDocument) -> Self {
        let mut index = Self::default();
        super::kernel::visit_states(kernel, |path, state| index.record(state, path));
        index
    }
    fn record(&mut self, state: &super::kernel::State, path: &str) {
        self.all_tags.extend(state.tags.iter().cloned());
        for (transition_index, transition) in state.transitions.iter().enumerate() {
            for tag in &transition.tags {
                self.all_tags.insert(tag.clone());
                self.transition_tags.insert(tag.clone());
                self.locations
                    .entry(tag.clone())
                    .or_default()
                    .push(TransitionLocation {
                        state_pointer: path.to_owned(),
                        transition_index,
                    });
            }
        }
    }
    /// Return actual authored transitions bearing a semantic tag.
    pub fn transition_locations(&self, tag: &str) -> &[TransitionLocation] {
        self.locations.get(tag).map_or(&[], Vec::as_slice)
    }
}
/// Reason a static declaration fails independently of runtime evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceIssueKind {
    /// Rights-impacting policy lacks individualized explanations.
    ExplanationLevel,
    /// The selected policy does not require counterfactual evidence.
    CounterfactualRequired,
    /// Selected reference execution lacks required counterfactual inputs.
    CounterfactualInputs,
    /// A required audit tier is absent or does not cover its required tag.
    AuditCoverage,
    /// An audit declaration names a tag absent from actual transitions.
    UnknownAuditTag,
    /// An authored section cannot deserialize into its existing typed model.
    MalformedDeclaration,
}
/// Static governance failure mapped separately by lint and runtime.
#[derive(Debug)]
pub struct GovernanceIssue {
    /// Maintained diagnostic identifier, independent of presentation.
    pub rule_id: &'static str,
    /// Canonical pointer relative to the embedded governance block.
    pub path: String,
    /// Semantic failure category for runtime refusal mapping.
    pub kind: GovernanceIssueKind,
    /// Human-readable declaration requirement.
    pub message: &'static str,
}
/// Results of decoding and checking canonical static governance declarations.
#[derive(Debug)]
pub struct GovernanceChecks {
    /// Independent declaration failures; malformed sections never imply success.
    pub issues: Vec<GovernanceIssue>,
    /// Existing typed audit model for further native-profile restrictions.
    pub audit: AuditConfig,
}

/// Check canonical declarations using existing typed policy and execution models.
///
/// Sections decode independently so malformed input retains its specific findings.
/// This generic route preserves impact-level and transition-tag applicability.
pub fn check_governance(
    kernel: &KernelDocument,
    governance: &Value,
    lifecycle: &LifecycleIndex,
) -> GovernanceChecks {
    let rights = kernel.impact_level == Some(ImpactLevel::RightsImpacting);
    let mut issues = if rights {
        explanation_issues(governance)
    } else {
        Vec::new()
    };
    if let Some(execution) = governance.pointer("/dueProcess/execution") {
        issues.extend(execution_issues(execution));
    }
    let reasoning = governance
        .pointer("/audit/reasoningTier")
        .cloned()
        .map(serde_json::from_value::<ReasoningTierConfig>);
    let counterfactual = governance
        .pointer("/audit/counterfactualTier")
        .cloned()
        .map(serde_json::from_value::<CounterfactualTierConfig>);
    if lifecycle.transition_tags.contains("determination") {
        if let Some(issue) = reasoning_issue(reasoning.as_ref(), lifecycle) {
            issues.push(issue);
        }
    }
    if rights && lifecycle.transition_tags.contains("adverse-decision") {
        if let Some(issue) = counterfactual_issue(counterfactual.as_ref(), lifecycle) {
            issues.push(issue);
        }
    }
    GovernanceChecks {
        issues,
        audit: AuditConfig {
            reasoning_tier: reasoning.and_then(Result::ok),
            counterfactual_tier: counterfactual.and_then(Result::ok),
        },
    }
}

/// Check the selected reference profile without relaxing its evidence declarations.
///
/// The selected profile requires individualized counterfactual explanations even
/// when the workflow omits an impact level. It uses the same generic predicates.
pub fn check_reference_governance(
    kernel: &KernelDocument,
    governance: &Value,
    lifecycle: &LifecycleIndex,
) -> GovernanceChecks {
    let mut checks = check_governance(kernel, governance, lifecycle);
    if kernel.impact_level != Some(ImpactLevel::RightsImpacting) {
        checks.issues.extend(explanation_issues(governance));
        let tier = governance
            .pointer("/audit/counterfactualTier")
            .cloned()
            .map(serde_json::from_value::<CounterfactualTierConfig>);
        if let Some(issue) = counterfactual_issue(tier.as_ref(), lifecycle) {
            checks.issues.push(issue);
        }
    }
    if governance.pointer("/dueProcess/execution").is_none() {
        checks.issues.push(GovernanceIssue {
            rule_id: "G-005",
            path: "/dueProcess/execution".into(),
            kind: GovernanceIssueKind::CounterfactualInputs,
            message: "selected reference profile requires its execution configuration",
        });
    }
    checks
}

fn explanation_issues(governance: &Value) -> Vec<GovernanceIssue> {
    let mut issues = Vec::new();
    let adverse = governance
        .pointer("/dueProcess/adverseDecisionPolicy")
        .cloned()
        .and_then(|value| serde_json::from_value::<AdverseDecisionPolicy>(value).ok());
    if adverse.as_ref().and_then(|policy| policy.explanation_level)
        != Some(ExplanationLevel::Individualized)
    {
        issues.push(GovernanceIssue {rule_id:"G-004", path:"/dueProcess/adverseDecisionPolicy/explanationLevel".into(),kind:GovernanceIssueKind::ExplanationLevel,
            message:"rights-impacting kernel requires explanationLevel 'individualized' in governance dueProcess"});
    }
    if !adverse
        .as_ref()
        .is_some_and(|policy| policy.counterfactual_required)
    {
        issues.push(GovernanceIssue {
            rule_id: "G-005",
            path: "/dueProcess/adverseDecisionPolicy/counterfactualRequired".into(),
            kind: GovernanceIssueKind::CounterfactualRequired,
            message: "rights-impacting policy must require counterfactual evidence",
        });
    }
    issues
}

fn execution_issues(value: &Value) -> Vec<GovernanceIssue> {
    let Ok(execution) = serde_json::from_value::<DueProcessExecution>(value.clone()) else {
        return vec![GovernanceIssue {
            rule_id: "G-005",
            path: "/dueProcess/execution".into(),
            kind: GovernanceIssueKind::MalformedDeclaration,
            message: "selected reference execution must match its canonical typed configuration",
        }];
    };
    let mut issues = Vec::new();
    if execution.determinations.is_empty() {
        issues.push(GovernanceIssue {
            rule_id: "G-005",
            path: "/dueProcess/execution/determinations".into(),
            kind: GovernanceIssueKind::CounterfactualInputs,
            message: "selected reference execution requires determination inputs with positive and negative counterfactuals",
        });
    }
    for (index, determination) in execution.determinations.iter().enumerate() {
        for (polarity, alternatives) in [
            ("positive", &determination.explanation.positive),
            ("negative", &determination.explanation.negative),
        ] {
            if alternatives.is_empty() {
                issues.push(GovernanceIssue {
                    rule_id: "G-005",
                    path: format!("/dueProcess/execution/determinations/{index}/explanation/{polarity}"),
                    kind: GovernanceIssueKind::CounterfactualInputs,
                    message: "selected reference determination requires nonempty counterfactual inputs",
                });
            }
        }
    }
    issues
}
fn reasoning_issue(
    tier: Option<&Result<ReasoningTierConfig, serde_json::Error>>,
    lifecycle: &LifecycleIndex,
) -> Option<GovernanceIssue> {
    audit_issue(
        "G-014",
        "reasoningTier",
        "determination",
        tier.map(|value| value.as_ref().map(|v| v.required_for_tags.as_slice())),
        lifecycle,
    )
}
fn counterfactual_issue(
    tier: Option<&Result<CounterfactualTierConfig, serde_json::Error>>,
    lifecycle: &LifecycleIndex,
) -> Option<GovernanceIssue> {
    audit_issue(
        "G-015",
        "counterfactualTier",
        "adverse-decision",
        tier.map(|value| value.as_ref().map(|v| v.required_for_tags.as_slice())),
        lifecycle,
    )
    .or_else(|| {
        tier.and_then(|value| value.as_ref().ok())
            .filter(|value| !value.require_protected_characteristics)
            .map(|_| GovernanceIssue {
                rule_id: "G-015",
                path: "/audit/counterfactualTier".into(),
                kind: GovernanceIssueKind::AuditCoverage,
                message: "counterfactual audit requires protected-characteristic evidence",
            })
    })
}
fn audit_issue(
    rule_id: &'static str,
    name: &str,
    required_tag: &str,
    tags: Option<Result<&[String], &serde_json::Error>>,
    lifecycle: &LifecycleIndex,
) -> Option<GovernanceIssue> {
    let kind = match tags {
        Some(Err(_)) => Some(GovernanceIssueKind::MalformedDeclaration),
        None => Some(GovernanceIssueKind::AuditCoverage),
        Some(Ok(tags))
            if !tags.iter().any(|tag| tag == required_tag)
                || tags
                    .iter()
                    .enumerate()
                    .any(|(index, tag)| tag.trim().is_empty() || tags[..index].contains(tag)) =>
        {
            Some(GovernanceIssueKind::AuditCoverage)
        }
        Some(Ok(tags))
            if tags
                .iter()
                .any(|tag| !lifecycle.transition_tags.contains(tag)) =>
        {
            Some(GovernanceIssueKind::UnknownAuditTag)
        }
        Some(Ok(_)) => None,
    };
    kind.map(|kind|GovernanceIssue {rule_id,path:format!("/audit/{name}"),kind,message:"workflow transition audit tier must cover unique declared transition tags and required evidence"})
}
