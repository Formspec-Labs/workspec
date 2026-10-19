// Rust guideline compliant 2026-02-21

//! Native agent and shared policy values from the admitted Workflow.
//!
//! These values have no independent document identity. The owning loader in
//! `crate::ai_policy` joins them to actors and preserves their native locators.
//! Unknown policy properties fail; descriptive x-* metadata conveys no authority.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Descriptive extension properties with no executable meaning.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(transparent)]
pub struct Metadata(pub BTreeMap<String, Value>);
impl<'de> Deserialize<'de> for Metadata {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let values = BTreeMap::<String, Value>::deserialize(deserializer)?;
        if let Some(key) = values.keys().find(|key| !key.starts_with("x-")) {
            return Err(serde::de::Error::custom(format!(
                "unknown native policy property {key}"
            )));
        }
        Ok(Self(values))
    }
}

/// Increasing permission to operate without human control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AutonomyLevel {
    Manual,
    Assistive,
    Supervisory,
    Autonomous,
}

/// Declared model taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentType {
    Deterministic,
    Statistical,
    Generative,
}

/// Model selection policy, requiring its owning resolver when not pinned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ModelVersionPolicy {
    #[default]
    Pinned,
    Approved,
    Latest,
}

/// Structured capability; declaration does not authorize invocation or writes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    /// Unique capability id within its agent.
    pub id: String,
    /// Human-readable explanation.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Selected input definition reference.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_contract_ref: Option<String>,
    /// Selected output definition reference.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_contract_ref: Option<String>,
    /// FEL predicates checked by the invocation owner.
    #[serde(default)]
    pub preconditions: Vec<String>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}

/// Native per-agent policy; actor kind comes from the actor join.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDeclaration {
    /// Actor id, joined reciprocally by the loader.
    pub id: String,
    /// Required model taxonomy.
    pub agent_type: AgentType,
    /// Admitted model identity.
    pub model_identifier: String,
    /// Admitted model version.
    pub model_version: String,
    /// Human-readable explanation.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Structured capabilities.
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// Version-selection policy.
    #[serde(default)]
    pub model_version_policy: ModelVersionPolicy,
    /// Typed invocation binding, when configured.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoker: Option<crate::agent::InvokerSpec>,
    /// Agent-wide restrictions.
    #[serde(default)]
    pub deontic_constraints: Vec<DeonticConstraint>,
    /// Additional confidence restriction.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_floor: Option<ConfidenceFloor>,
    /// Confidence history requirements.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_decay: Option<ConfidenceDecay>,
    /// Complete fallback override; omission inherits the workflow chain.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_chain: Option<Vec<FallbackStep>>,
    /// Allowed autonomous target agents.
    #[serde(default)]
    pub cascading_invocations: Vec<String>,
    /// Versions admitted by approved selection.
    #[serde(default)]
    pub approved_versions: Vec<String>,
    /// Trusted calibration requirements.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<CalibrationConfig>,
    /// Authored limits and operational rules.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autonomy_policy: Option<AutonomyPolicy>,
    /// Restrictions at exact admitted lifecycle actions.
    #[serde(default)]
    pub action_overrides: Vec<ActionOverride>,
    /// Native monitoring requirements.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drift_monitoring: Option<DriftMonitoring>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}

/// Shared AI policy with defined fields only; reserved configurations fail.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiOversight {
    /// Shared ceiling, omitted meaning assistive.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_autonomy: Option<AutonomyLevel>,
    /// Workflow restrictions.
    #[serde(default)]
    pub deontic_constraints: Vec<DeonticConstraint>,
    /// Additional shared confidence restriction.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_floor: Option<ConfidenceFloor>,
    /// Complete shared fallback chain.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_chain: Option<Vec<FallbackStep>>,
    /// Processor-owned count requirements.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_constraints: Option<VolumeConstraints>,
    /// Required notice disclosures.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disclosure: Option<AgentDisclosure>,
    /// Descriptive metadata; reserved names are not extensions.
    #[serde(flatten)]
    pub metadata: Metadata,
}

/// Increasing restriction, independent of evaluation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ViolationAction {
    Flag,
    SwitchToAssistive,
    EscalateToHuman,
    Reject,
}

/// Disposition for null, never a substituted predicate truth value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NullBehavior {
    Pass,
    Deny,
    Escalate,
}

/// One closed, kind-specific rule at any native scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DeonticConstraint {
    /// Output field and value bounds.
    Permission(Permission),
    /// Forbidden output condition.
    Prohibition(Prohibition),
    /// Required output condition.
    Obligation(Obligation),
    /// Processor obligation to supply input.
    Right(Right),
}
impl DeonticConstraint {
    /// Read the human-readable rule id.
    pub fn id(&self) -> &str {
        match self {
            Self::Permission(v) => &v.id,
            Self::Prohibition(v) => &v.id,
            Self::Obligation(v) => &v.id,
            Self::Right(v) => &v.id,
        }
    }
}

/// Permission restricts fields and/or values; all restrictions compose.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    /// Scope-unique rule id.
    pub id: String,
    /// Omission is unrestricted; an empty list permits no output.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_fields: Option<Vec<String>>,
    /// Boolean value bound.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<String>,
    /// Required violation disposition.
    pub on_violation: ViolationAction,
    /// Optional null disposition.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub null_behavior: Option<NullBehavior>,
    /// Does not itself grant waiver authority.
    #[serde(default)]
    pub bypassable: bool,
    /// Descriptive supporting sources.
    #[serde(default)]
    pub citation_refs: Vec<String>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}

/// Prohibition violates when its condition is true.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prohibition {
    /// Scope-unique id.
    pub id: String,
    /// Forbidden boolean condition.
    pub condition: String,
    /// Required violation disposition.
    pub on_violation: ViolationAction,
    /// Human-readable explanation.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Optional null disposition.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub null_behavior: Option<NullBehavior>,
    /// Does not itself grant waiver authority.
    #[serde(default)]
    pub bypassable: bool,
    /// Descriptive supporting sources.
    #[serde(default)]
    pub citation_refs: Vec<String>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}

/// Obligation violates when its required condition is false.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Obligation {
    /// Scope-unique id.
    pub id: String,
    /// Required boolean condition.
    pub requirement: String,
    /// Required violation disposition.
    pub on_violation: ViolationAction,
    /// Human-readable explanation.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Optional null disposition.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub null_behavior: Option<NullBehavior>,
    /// Does not itself grant waiver authority.
    #[serde(default)]
    pub bypassable: bool,
    /// Descriptive supporting sources.
    #[serde(default)]
    pub citation_refs: Vec<String>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}

/// Right identifies input the processor must supply, including false or zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Right {
    /// Scope-unique id.
    pub id: String,
    /// Data pointer rooted at input or caseFile.
    pub entitlement: String,
    /// Human-readable explanation.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Descriptive supporting sources.
    #[serde(default)]
    pub citation_refs: Vec<String>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}

/// Confidence method describes reported values, not trusted calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfidenceMethod {
    ModelNative,
    Calibrated,
    Heuristic,
    Conformal,
    Declared,
}

/// Canonical untrusted confidence values returned with every agent output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceReport {
    /// Reported score, finite and in [0,1].
    #[serde(deserialize_with = "deserialize_score")]
    pub overall: f64,
    /// Required method; no invented default.
    pub method: ConfidenceMethod,
    /// Human-readable explanation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_optional_present")]
    pub explanation: Option<String>,
    /// Reported per-field scores.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "deserialize_field_scores")]
    pub field_level: Option<BTreeMap<String, f64>>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
fn deserialize_score<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let score = f64::deserialize(deserializer)?;
    if valid_score(score) {
        Ok(score)
    } else {
        Err(serde::de::Error::custom(
            "confidence score must be finite and in [0,1]",
        ))
    }
}
fn deserialize_field_scores<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<BTreeMap<String, f64>>, D::Error> {
    let scores = BTreeMap::<String, f64>::deserialize(deserializer)?;
    if scores.values().any(|score| !valid_score(*score)) {
        return Err(serde::de::Error::custom(
            "confidence field scores must be finite and in [0,1]",
        ));
    }
    Ok(Some(scores))
}
/// Reject explicit null while retaining omission through each field's default.
pub(crate) fn deserialize_optional_present<'de, D, T>(
    deserializer: D,
) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
impl ConfidenceReport {
    /// Validate every reported score without changing it.
    ///
    /// # Errors
    /// Returns a message for nonfinite or out-of-range reported values.
    pub fn validate(&self) -> Result<(), String> {
        if !valid_score(self.overall)
            || self
                .field_level
                .as_ref()
                .is_some_and(|v| v.values().any(|v| !valid_score(*v)))
        {
            return Err("confidence report scores must be finite and in [0,1]".into());
        }
        Ok(())
    }
}
/// Check a confidence number at the native owner boundary.
pub fn valid_score(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

/// A confidence restriction applied independently at each scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceFloor {
    /// Finite required threshold in [0,1].
    pub threshold: f64,
    /// Effect of falling strictly below threshold.
    pub on_violation: ConfidenceViolationAction,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// Supported confidence-floor effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfidenceViolationAction {
    EscalateToHuman,
    Reject,
}

/// Requirements for processor-computed confidence decay.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceDecay {
    /// Enabled decay requires maintained history/clock operations.
    pub enabled: bool,
    /// Positive half-life when configured.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_life: Option<String>,
    /// Multiplicative event reductions.
    #[serde(default)]
    pub triggers: Vec<DecayTrigger>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// One configured decay trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecayTrigger {
    /// Nonempty native event name.
    pub event: String,
    /// Finite multiplicative factor in [0,1].
    pub decay_factor: f64,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}

/// Retry scheduling algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackoffStrategy {
    Fixed,
    Linear,
    Exponential,
}

/// Native calibration requirements; evidence belongs to the processor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationConfig {
    /// Whether trusted calibration is required.
    pub required: bool,
    /// Positive recalibration interval when required.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
    /// Positive minimum sample count.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_samples: Option<u64>,
    /// Declared calibration method.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<CalibrationMethod>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// Native calibration method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CalibrationMethod {
    PlattScaling,
    Isotonic,
    Binning,
    Custom,
}

/// Authored agent ceiling and stateful rule declarations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutonomyPolicy {
    /// Optional hard ceiling.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_autonomy: Option<AutonomyLevel>,
    /// Authenticated restoration requirements.
    #[serde(default)]
    pub escalation: Vec<EscalationRule>,
    /// Operational reduction requirements.
    #[serde(default)]
    pub demotion: Vec<DemotionRule>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// Automatic operational reduction declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DemotionRule {
    /// Agent-local unique id.
    pub id: String,
    /// Boolean FEL condition over admitted case and operational state.
    pub condition: String,
    /// A level below autonomous.
    pub target_level: AutonomyLevel,
    /// Keeps the reduction until trusted recalibration.
    #[serde(default)]
    pub pending_recalibration: bool,
    /// Human-readable explanation.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// Authenticated, expiring restoration declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EscalationRule {
    /// Agent-local unique id.
    pub id: String,
    /// Boolean FEL condition over trusted context.
    pub condition: String,
    /// Requested restored level.
    pub target_level: AutonomyLevel,
    /// Native human actor slot, not a descriptive role.
    pub required_approver_ref: String,
    /// Positive approval duration.
    pub escalation_expiry: String,
    /// Human-readable explanation.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// Additional restrictions at one exact native lifecycle action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionOverride {
    /// Admitted lifecycle action JSON pointer.
    pub action_ref: String,
    /// Optional local ceiling; never persistently demotes the agent.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autonomy: Option<AutonomyLevel>,
    /// Positive supervisory review window.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_window: Option<String>,
    /// Additional confidence restriction.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_floor: Option<ConfidenceFloor>,
    /// Additional kind-specific rules.
    #[serde(default)]
    pub deontic_constraints: Vec<DeonticConstraint>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}

/// Shared count limits requiring maintained state and time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeConstraints {
    /// Maximum autonomous actions per hour, including zero.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_autonomous_per_hour: Option<u64>,
    /// Maximum autonomous actions per day, including zero.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_autonomous_per_day: Option<u64>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// Defined shared notice settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDisclosure {
    /// Whether agent assistance must be disclosed.
    pub disclose_that_agent_assisted: bool,
    /// Additional model identity disclosure.
    #[serde(default)]
    pub disclose_model_identity: bool,
    /// Additional confidence disclosure.
    #[serde(default)]
    pub disclose_confidence: bool,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}

/// Native per-agent monitoring configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftMonitoring {
    /// Same-agent monitor declarations.
    pub monitors: Vec<AgentMonitor>,
    /// Deployment requirements, when configured.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_sequence: Option<DeploymentSequence>,
    /// Explicit descriptive extension map with x-* keys.
    #[serde(default)]
    pub extensions: Metadata,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// One native monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMonitor {
    /// Positive measurement window.
    pub evaluation_window: String,
    /// Metric declarations.
    pub metrics: Vec<MonitorMetric>,
    /// Review measurement requirements.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rubber_stamp: Option<RubberStampConfig>,
    /// Alerts scoped to the enclosing agent.
    #[serde(default)]
    pub alert_thresholds: Vec<AlertThreshold>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// Metric declaration; measurement execution remains an owning operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorMetric {
    /// Metric name.
    pub name: String,
    /// Statistical method.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<MonitorMethod>,
    /// Finite optional threshold.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// Supported measurement algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MonitorMethod {
    Psi,
    Ks,
    Chi2,
    Accuracy,
    Custom,
}
/// Review measurement requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RubberStampConfig {
    /// Enables measured review monitoring.
    pub enabled: bool,
    /// Positive minimum review time.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_review_time: Option<String>,
    /// Finite agreement fraction.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_agreement_rate: Option<f64>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// Native alert, selecting exactly one action or same-agent demotion rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertThreshold {
    /// Boolean alert expression.
    pub condition: String,
    /// Alert severity.
    pub severity: AlertSeverity,
    /// Direct action, exclusive with policy_ref.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<DriftAction>,
    /// Same-agent demotion rule id.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_ref: Option<String>,
    /// Notification recipients.
    #[serde(default)]
    pub notify_roles: Vec<String>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}
/// Native alert severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}
/// Native operational alert action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DriftAction {
    Notify,
    DemoteToAssistive,
    DemoteToManual,
    Suspend,
}
/// Declared deployment requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentSequence {
    /// Requires actual deployment operations when true.
    pub enabled: bool,
    /// Positive shadow duration.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow_duration: Option<String>,
    /// Strictly between zero and one.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canary_percentage: Option<f64>,
    /// Positive canary duration.
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_optional_present")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canary_duration: Option<String>,
    /// Descriptive metadata.
    #[serde(flatten)]
    pub metadata: Metadata,
}

/// Closed fallback configuration; selecting it does not execute an operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FallbackStep {
    /// Finite retry of the same capability and admitted input.
    Retry {
        /// Retry count excludes the first invocation.
        max_retries: u32,
        /// Delay algorithm.
        backoff: BackoffStrategy,
        /// Positive initial interval.
        initial_interval: String,
        /// Descriptive metadata.
        #[serde(flatten)]
        metadata: Metadata,
    },
    /// Same capability on another admitted agent.
    AlternateAgent {
        /// Same-revision native agent id.
        alternate_agent_ref: String,
        /// Descriptive metadata.
        #[serde(flatten)]
        metadata: Metadata,
    },
    /// Configuration from an exact native createTask occurrence.
    EscalateToHuman {
        /// Native lifecycle action pointer; never a task-label alias.
        task_action_ref: String,
        /// Descriptive metadata.
        #[serde(flatten)]
        metadata: Metadata,
    },
    /// Terminal failure, never accepted agent output.
    Fail {
        /// Descriptive metadata.
        #[serde(flatten)]
        metadata: Metadata,
    },
}
