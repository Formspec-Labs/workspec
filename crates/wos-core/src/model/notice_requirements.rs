// Rust guideline compliant 2026-02-21
//! Shared notice declaration and installed-capability checks.
//!
//! Static authoring uses [`check_notice_declaration`]. Selected execution uses
//! [`check_notice_support`] with capabilities installed by the owning provider.
//! These pure checks authenticate no receipts and grant no historical evidence.
//! Callers derive consequences from actual binding use and retain their own
//! original content, recipient, case, result, clock and policy association checks.

use super::governance::NoticeTiming;
use serde::{Deserialize, Serialize};

/// Evidence, scope and clock anchor required by one selected notice binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoticeRequirement {
    /// Required observation meaning; callback names do not establish it.
    pub observation: NoticeObservation,
    /// Evidence scope retained in every derived clock or effect permission.
    pub scope: NoticeScope,
    /// Original qualifying time used by the existing period/calendar owner.
    pub anchor: NoticeAnchor,
}

/// Distinct observation meanings; transport acceptance never implies receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoticeObservation {
    /// Authenticated transport acceptance, without recipient receipt authority.
    TransportAccepted,
    /// Documented provider delivery, without a reading or acknowledgment claim.
    ProviderDelivered,
    /// Observed opening, without a reading or acknowledgment claim.
    OpenObserved,
    /// Actual recipient acknowledgment, requiring its installed qualifier.
    RecipientAcknowledged,
}

/// Scope of notice evidence and all facts derived from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoticeScope {
    /// Evidence qualified for the selected production observation.
    Production,
    /// Explicit software fixture evidence, without legal delivery qualification.
    SoftwareFixture,
}

/// Selected clock anchor; periods and calendars remain in existing policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoticeAnchor {
    /// Ordinary status starts no governed deadline or adverse consequence.
    None,
    /// Reuse the original immutable qualifying observation time.
    QualifiedObservation,
}

/// Actual notice use derived by the owner from resolved native bindings.
///
/// This is not serialized selection input. Owners check every actual use;
/// an ordinary-status declaration cannot override deadline or adverse policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeConsequence {
    /// Informational status with no governed clock or protected consequence.
    OrdinaryStatus,
    /// Response period begins at the selected original observation.
    ResponseDeadline,
    /// Required notice and grace authorize the original proposed adverse effect.
    AdverseEffect,
    /// Filing window uses the original adverse notice and proposal.
    AppealWindow,
}

/// Currently implemented provider capabilities selected by their owning adapter.
///
/// This type is deliberately not deserializable evidence. Installing a provider
/// does not authenticate an individual result. No production recipient qualifier
/// exists in this supported set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeCapability {
    /// SMTP acceptance, without recipient receipt or acknowledgment.
    Smtp,
    /// Explicit local SMTP capture, limited to software fixture scope.
    LocalSmtpTestCapture,
}

/// Located declaration or selected-provider refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeRequirementIssue {
    /// Stable diagnostic identifier shared by selected owners.
    pub rule_id: &'static str,
    /// JSON pointer supplied by the caller, extended to the affected field.
    pub path: String,
    /// Human-readable requirement or unsupported capability.
    pub message: &'static str,
}

/// Check adverse notice timing independently of provider installation.
#[must_use]
pub fn check_adverse_notice_timing(
    timing: Option<NoticeTiming>,
    path: &str,
) -> Option<NoticeRequirementIssue> {
    (timing != Some(NoticeTiming::BeforeEffective)).then(|| NoticeRequirementIssue {
        rule_id: "G-001",
        path: path.to_owned(),
        message: "required adverse notice must explicitly precede effect with beforeEffective timing",
    })
}

/// Check authored meaning without assuming an installed provider.
///
/// `consequence` comes from actual resolved binding use. For adverse effects,
/// `timing` is the authored adverse policy timing. Its refusal locates `path`
/// itself because timing belongs to policy rather than the requirement object.
#[must_use]
pub fn check_notice_declaration(
    requirement: &NoticeRequirement,
    consequence: NoticeConsequence,
    timing: Option<NoticeTiming>,
    path: &str,
) -> Vec<NoticeRequirementIssue> {
    let mut issues = Vec::new();
    let expected_anchor = if consequence == NoticeConsequence::OrdinaryStatus {
        NoticeAnchor::None
    } else {
        NoticeAnchor::QualifiedObservation
    };
    if requirement.anchor != expected_anchor {
        issues.push(NoticeRequirementIssue {
            rule_id: "NOTICE-003",
            path: format!("{path}/anchor"),
            message: "ordinary status requires no anchor; consequence-bearing notice requires qualifiedObservation",
        });
    }
    if consequence == NoticeConsequence::AdverseEffect {
        issues.extend(check_adverse_notice_timing(timing, path));
    }
    issues
}

/// Check declarations and actual installed capability before selected execution.
///
/// Capability matching is exact: neither SMTP nor local capture satisfies any
/// other observation. Capture supports only software scope. Original result,
/// message association and immutable observation-time checks remain mandatory
/// in the owning adapter/runtime.
#[must_use]
pub fn check_notice_support(
    requirement: &NoticeRequirement,
    consequence: NoticeConsequence,
    timing: Option<NoticeTiming>,
    installed: &[NoticeCapability],
    path: &str,
) -> Vec<NoticeRequirementIssue> {
    let mut issues = check_notice_declaration(requirement, consequence, timing, path);
    let supported = installed.iter().any(|capability| {
        matches!(
            (capability, requirement.observation, requirement.scope),
            (
                NoticeCapability::Smtp,
                NoticeObservation::TransportAccepted,
                _
            ) | (
                NoticeCapability::LocalSmtpTestCapture,
                NoticeObservation::TransportAccepted,
                NoticeScope::SoftwareFixture
            )
        )
    });
    if !supported {
        issues.push(NoticeRequirementIssue {
            rule_id: "NOTICE-004",
            path: format!("{path}/observation"),
            message: "no installed notice provider establishes the selected observation and scope",
        });
    }
    issues
}

#[cfg(test)]
mod tests;
