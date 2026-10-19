// Rust guideline compliant 2026-02-21

//! Strict native AI admission and shared policy exercise context.
//!
//! Loading never changes the candidate or executes an action. The read view has
//! no independent authored identity and is deliberately not serializable.
//! Admission, supplied-output policy decisions and execution readiness differ.

use crate::model::ai::*;
use crate::model::kernel::ImpactLevel;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Native policy error with its original authored locator.
#[derive(Debug)]
pub struct AiPolicyError {
    pointer: String,
    message: String,
}
impl AiPolicyError {
    /// Identify the native source location.
    pub fn pointer(&self) -> &str {
        &self.pointer
    }
    /// Read the precise admission or evaluation refusal.
    pub fn message(&self) -> &str {
        &self.message
    }
    pub(crate) fn new(pointer: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            pointer: pointer.into(),
            message: message.into(),
        }
    }
}
impl fmt::Display for AiPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.pointer, self.message)
    }
}
impl std::error::Error for AiPolicyError {}

/// Immutable native AI read view; raw source remains unchanged.
#[derive(Debug, Clone)]
pub struct NativeAiPolicy {
    /// Selected workflow URL.
    workflow_url: String,
    /// Selected workflow revision.
    workflow_version: String,
    /// Native impact level.
    impact_level: ImpactLevel,
    /// Strict native declarations in authored order.
    agents: Vec<AgentDeclaration>,
    /// Strict shared policy.
    oversight: AiOversight,
    source: Value,
    kernel_snapshot: Value,
    action_pointers: BTreeSet<String>,
}
impl NativeAiPolicy {
    /// Read the admitted workflow identity.
    pub fn workflow_url(&self) -> &str {
        &self.workflow_url
    }
    /// Read the admitted workflow revision.
    pub fn workflow_version(&self) -> &str {
        &self.workflow_version
    }
    /// Read the admitted native impact level.
    pub fn impact_level(&self) -> ImpactLevel {
        self.impact_level
    }
    /// Read declarations without granting mutation of admitted policy.
    pub fn agents(&self) -> &[AgentDeclaration] {
        &self.agents
    }
    /// Read shared restrictions without granting mutation of admitted policy.
    pub fn oversight(&self) -> &AiOversight {
        &self.oversight
    }
    /// Read the exact admitted candidate values without reconstructing them.
    pub fn source(&self) -> &Value {
        &self.source
    }
    /// Find an admitted agent by native actor id.
    pub fn agent(&self, id: &str) -> Option<&AgentDeclaration> {
        self.agents.iter().find(|a| a.id == id)
    }
    /// Resolve one actual lifecycle action occurrence, excluding metadata lookalikes.
    ///
    /// # Errors
    /// Rejects invalid pointers or locations outside native lifecycle action lists.
    pub fn action(&self, pointer: &str) -> Result<&Value, AiPolicyError> {
        pointer_segments(pointer).map_err(|e| AiPolicyError::new(pointer, e))?;
        if !self.action_pointers.contains(pointer) {
            return Err(AiPolicyError::new(
                pointer,
                "not an actual native lifecycle action occurrence",
            ));
        }
        self.source
            .pointer(pointer)
            .ok_or_else(|| AiPolicyError::new(pointer, "unresolved native action"))
    }
    /// Read every admitted rule and its native occurrence without parsing again.
    pub fn rule_definitions(&self) -> Vec<(String, &DeonticConstraint)> {
        let mut rules = Vec::new();
        for (index, rule) in self.oversight.deontic_constraints.iter().enumerate() {
            rules.push((format!("/aiOversight/deonticConstraints/{index}"), rule));
        }
        for (agent_index, agent) in self.agents.iter().enumerate() {
            for (index, rule) in agent.deontic_constraints.iter().enumerate() {
                rules.push((
                    format!("/agents/{agent_index}/deonticConstraints/{index}"),
                    rule,
                ));
            }
            for (action_index, action) in agent.action_overrides.iter().enumerate() {
                for (index, rule) in action.deontic_constraints.iter().enumerate() {
                    rules.push((format!("/agents/{agent_index}/actionOverrides/{action_index}/deonticConstraints/{index}"), rule));
                }
            }
        }
        rules
    }

    /// Bind policy to every semantically represented field of the admitted kernel.
    ///
    /// # Errors
    /// Refuses a kernel that cannot be represented by the maintained native model.
    pub fn matches_kernel(&self, kernel: &crate::KernelDocument) -> Result<bool, AiPolicyError> {
        serde_json::to_value(kernel)
            .map(|value| value == self.kernel_snapshot)
            .map_err(|error| AiPolicyError::new("", error.to_string()))
    }

    /// Read the selected complete fallback chain without concatenation.
    fn fallback_chain<'a>(&'a self, agent: &'a AgentDeclaration) -> Option<&'a [FallbackStep]> {
        agent
            .fallback_chain
            .as_deref()
            .or(self.oversight.fallback_chain.as_deref())
    }
    /// Require a harmless supplied-output exercise, distinct from invocation readiness.
    ///
    /// # Errors
    /// Refuses effects or operations this limited exercise cannot establish.
    pub fn check_supplied_output_exercise(&self, agent_id: &str) -> Result<(), AiPolicyError> {
        let agent = self
            .agent(agent_id)
            .ok_or_else(|| AiPolicyError::new("/agents", "unknown admitted agent"))?;
        for field in [
            "governance",
            "advanced",
            "signature",
            "custody",
            "assurance",
            "intake",
        ] {
            if self.source.get(field).is_some_and(|value| !value.is_null()) {
                return Err(AiPolicyError::new(
                    format!("/{field}"),
                    "unsupported composed governance operation in isolated supplied-output policy exercise",
                ));
            }
        }
        if self
            .source
            .get("bindings")
            .is_some_and(|value| value.as_array().is_none_or(|bindings| !bindings.is_empty()))
        {
            return Err(AiPolicyError::new(
                "/bindings",
                "unsupported governed output operation in supplied-output policy exercise",
            ));
        }
        if !self.action_pointers.is_empty() {
            return Err(AiPolicyError::new(
                "/lifecycle",
                "unsupported supplied-output exercise with lifecycle actions: no output-to-case write or side-effect route is admitted",
            ));
        }
        if !agent.action_overrides.is_empty() {
            return Err(AiPolicyError::new(
                "/agents",
                "unsupported action policy: admitted action context is unavailable",
            ));
        }
        if agent.model_version_policy != ModelVersionPolicy::Pinned {
            return Err(AiPolicyError::new(
                "/agents",
                "unsupported model-selection resolver",
            ));
        }
        if agent.calibration.as_ref().is_some_and(|v| v.required)
            || agent.confidence_decay.as_ref().is_some_and(|v| v.enabled)
        {
            return Err(AiPolicyError::new(
                "/agents",
                "unsupported required calibration/decay history operation",
            ));
        }
        if agent
            .autonomy_policy
            .as_ref()
            .is_some_and(|v| !v.escalation.is_empty() || !v.demotion.is_empty())
            || agent.drift_monitoring.is_some()
        {
            return Err(AiPolicyError::new(
                "/agents",
                "unsupported operational autonomy/monitoring operation",
            ));
        }
        if self.oversight.volume_constraints.is_some() {
            return Err(AiPolicyError::new(
                "/aiOversight/volumeConstraints",
                "unsupported maintained counter/window/review operations",
            ));
        }
        if self.oversight.disclosure.as_ref().is_some_and(|v| {
            v.disclose_that_agent_assisted || v.disclose_model_identity || v.disclose_confidence
        }) {
            return Err(AiPolicyError::new(
                "/aiOversight/disclosure",
                "unsupported supplied-output disclosure operation",
            ));
        }
        Ok(())
    }
}
fn parse<T: DeserializeOwned>(value: &Value, pointer: &str) -> Result<T, AiPolicyError> {
    serde_json::from_value(value.clone()).map_err(|e| AiPolicyError::new(pointer, e.to_string()))
}
fn nonempty(value: &str, pointer: &str) -> Result<(), AiPolicyError> {
    if value.trim().is_empty() {
        Err(AiPolicyError::new(pointer, "nonempty value required"))
    } else {
        Ok(())
    }
}
fn positive_duration(value: &str, pointer: &str) -> Result<u64, AiPolicyError> {
    let ms = crate::eval::parse_iso_duration_to_ms(value)
        .map_err(|e| AiPolicyError::new(pointer, e.to_string()))?;
    if ms == 0 {
        return Err(AiPolicyError::new(pointer, "positive duration required"));
    }
    Ok(ms)
}
fn unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    pointer: &str,
) -> Result<(), AiPolicyError> {
    let mut seen = BTreeSet::new();
    for value in values {
        nonempty(value, pointer)?;
        if !seen.insert(value) {
            return Err(AiPolicyError::new(
                pointer,
                format!("duplicate identity {value}"),
            ));
        }
    }
    Ok(())
}
/// Decode strict RFC6901 segments without treating string prefixes as coverage.
///
/// # Errors
/// Rejects malformed pointers and invalid tilde escapes.
pub fn pointer_segments(pointer: &str) -> Result<Vec<String>, String> {
    if pointer.is_empty() {
        return Ok(Vec::new());
    }
    if !pointer.starts_with('/') {
        return Err("JSON Pointer must be empty or start with /".into());
    }
    pointer[1..]
        .split('/')
        .map(|part| {
            let mut out = String::new();
            let mut chars = part.chars();
            while let Some(c) = chars.next() {
                if c == '~' {
                    match chars.next() {
                        Some('0') => out.push('~'),
                        Some('1') => out.push('/'),
                        _ => return Err("invalid JSON Pointer escape".into()),
                    }
                } else {
                    out.push(c)
                }
            }
            Ok(out)
        })
        .collect()
}
fn escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
fn collect_actions(state: &Value, path: &str, out: &mut BTreeSet<String>) {
    for field in ["onEntry", "onExit"] {
        if let Some(actions) = state[field].as_array() {
            for (index, _) in actions.iter().enumerate() {
                out.insert(format!("{path}/{field}/{index}"));
            }
        }
    }
    if let Some(transitions) = state["transitions"].as_array() {
        for (index, transition) in transitions.iter().enumerate() {
            if let Some(actions) = transition["actions"].as_array() {
                for (action_index, _) in actions.iter().enumerate() {
                    out.insert(format!("{path}/transitions/{index}/actions/{action_index}"));
                }
            }
        }
    }
    if let Some(states) = state["states"].as_object() {
        for (name, state) in states {
            collect_actions(state, &format!("{path}/states/{}", escape(name)), out);
        }
    }
    if let Some(regions) = state["regions"].as_object() {
        for (name, region) in regions {
            if let Some(states) = region["states"].as_object() {
                for (state_name, state) in states {
                    collect_actions(
                        state,
                        &format!(
                            "{path}/regions/{}/states/{}",
                            escape(name),
                            escape(state_name)
                        ),
                        out,
                    );
                }
            }
        }
    }
    if let Some(body) = state.get("body") {
        collect_actions(body, &format!("{path}/body"), out);
    }
}
fn validate_rules(rules: &[DeonticConstraint], pointer: &str) -> Result<(), AiPolicyError> {
    unique(rules.iter().map(DeonticConstraint::id), pointer)?;
    for (index, rule) in rules.iter().enumerate() {
        let p = format!("{pointer}/{index}");
        let citations = match rule {
            DeonticConstraint::Permission(v) => {
                if v.allowed_fields.is_none() && v.bounds.is_none() {
                    return Err(AiPolicyError::new(
                        &p,
                        "permission requires allowedFields or bounds",
                    ));
                }
                if let Some(fields) = &v.allowed_fields {
                    for field in fields {
                        pointer_segments(field).map_err(|e| AiPolicyError::new(&p, e))?;
                    }
                }
                if let Some(bounds) = &v.bounds {
                    nonempty(bounds, &p)?;
                }
                &v.citation_refs
            }
            DeonticConstraint::Prohibition(v) => {
                nonempty(&v.condition, &p)?;
                &v.citation_refs
            }
            DeonticConstraint::Obligation(v) => {
                nonempty(&v.requirement, &p)?;
                &v.citation_refs
            }
            DeonticConstraint::Right(v) => {
                let segments =
                    pointer_segments(&v.entitlement).map_err(|e| AiPolicyError::new(&p, e))?;
                if !matches!(
                    segments.first().map(String::as_str),
                    Some("input" | "caseFile")
                ) {
                    return Err(AiPolicyError::new(
                        &p,
                        "right entitlement must be rooted at /input or /caseFile",
                    ));
                }
                &v.citation_refs
            }
        };
        for citation in citations {
            nonempty(citation, &p)?;
        }
    }
    Ok(())
}
fn validate_floor(floor: Option<&ConfidenceFloor>, pointer: &str) -> Result<(), AiPolicyError> {
    if floor.is_some_and(|f| !valid_score(f.threshold)) {
        return Err(AiPolicyError::new(
            pointer,
            "confidence threshold must be finite and in [0,1]",
        ));
    }
    Ok(())
}

/// Load the single native AI policy, preserving the original candidate.
///
/// # Errors
/// Refuses malformed native fields, identity joins, references, rules or settings.
/// Successful admission is draft loading, never execution readiness.
pub fn load_native_ai_policy(source: &Value) -> Result<Option<NativeAiPolicy>, AiPolicyError> {
    let agents: Vec<AgentDeclaration> =
        parse(source.get("agents").unwrap_or(&json!([])), "/agents")?;
    let actor_values = source["actors"]
        .as_array()
        .ok_or_else(|| AiPolicyError::new("/actors", "native actor array required"))?;
    let mut actors = BTreeMap::new();
    for actor in actor_values {
        let id = actor["id"]
            .as_str()
            .ok_or_else(|| AiPolicyError::new("/actors", "actor id required"))?;
        nonempty(id, "/actors")?;
        if actors
            .insert(id, actor["type"].as_str().unwrap_or(""))
            .is_some()
        {
            return Err(AiPolicyError::new("/actors", "duplicate actor id"));
        }
    }
    unique(agents.iter().map(|a| a.id.as_str()), "/agents")?;
    for agent in &agents {
        if actors.get(agent.id.as_str()) != Some(&"agent") {
            return Err(AiPolicyError::new(
                "/agents",
                "native declaration must join one agent actor",
            ));
        }
    }
    for (id, kind) in &actors {
        if *kind == "agent" && !agents.iter().any(|a| a.id == *id) {
            return Err(AiPolicyError::new(
                "/actors",
                "agent actor requires native declaration",
            ));
        }
    }
    if agents.is_empty() && source.get("aiOversight").is_none() {
        return Ok(None);
    }
    let oversight: AiOversight = parse(
        source.get("aiOversight").unwrap_or(&json!({})),
        "/aiOversight",
    )?;
    let mut action_pointers = BTreeSet::new();
    if let Some(states) = source
        .pointer("/lifecycle/states")
        .and_then(Value::as_object)
    {
        for (name, state) in states {
            collect_actions(
                state,
                &format!("/lifecycle/states/{}", escape(name)),
                &mut action_pointers,
            );
        }
    }
    let workflow_url = source["url"]
        .as_str()
        .ok_or_else(|| AiPolicyError::new("/url", "workflow URL required"))?
        .to_owned();
    nonempty(&workflow_url, "/url")?;
    let workflow_version = source["version"]
        .as_str()
        .ok_or_else(|| AiPolicyError::new("/version", "workflow version required"))?
        .to_owned();
    nonempty(&workflow_version, "/version")?;
    let impact_level: ImpactLevel = parse(&source["impactLevel"], "/impactLevel")?;
    let native_kernel: crate::KernelDocument = parse(source, "")?;
    let kernel_snapshot = serde_json::to_value(&native_kernel)
        .map_err(|error| AiPolicyError::new("", error.to_string()))?;
    let policy = NativeAiPolicy {
        workflow_url,
        workflow_version,
        impact_level,
        agents,
        oversight,
        source: source.clone(),
        kernel_snapshot,
        action_pointers,
    };
    for (pointer, raw) in native_rule_scopes(source) {
        let rules: Vec<DeonticConstraint> = parse(raw, &pointer)?;
        validate_rules(&rules, &pointer)?;
    }
    validate_floor(
        policy.oversight.confidence_floor.as_ref(),
        "/aiOversight/confidenceFloor",
    )?;
    if policy.impact_level == ImpactLevel::RightsImpacting
        && policy
            .oversight
            .disclosure
            .as_ref()
            .is_some_and(|d| !d.disclose_that_agent_assisted)
    {
        return Err(AiPolicyError::new(
            "/aiOversight/disclosure",
            "rights-impacting disclosure cannot be false",
        ));
    }
    for (index, agent) in policy.agents.iter().enumerate() {
        let p = format!("/agents/{index}");
        nonempty(&agent.model_identifier, &p)?;
        nonempty(&agent.model_version, &p)?;
        unique(
            agent.capabilities.iter().map(|v| v.id.as_str()),
            &format!("{p}/capabilities"),
        )?;
        unique(agent.cascading_invocations.iter().map(String::as_str), &p)?;
        for id in &agent.cascading_invocations {
            if policy.agent(id).is_none() {
                return Err(AiPolicyError::new(&p, "unresolved cascading agent"));
            }
        }
        validate_floor(
            agent.confidence_floor.as_ref(),
            &format!("{p}/confidenceFloor"),
        )?;
        for capability in &agent.capabilities {
            for expression in &capability.preconditions {
                nonempty(expression, &p)?;
            }
        }
        if let Some(crate::agent::InvokerSpec::Stub { responses, .. }) = &agent.invoker {
            for response in responses {
                response
                    .confidence_report
                    .validate()
                    .map_err(|e| AiPolicyError::new(format!("{p}/invoker/responses"), e))?;
            }
        }
        if let Some(calibration) = &agent.calibration {
            if calibration.minimum_samples == Some(0) {
                return Err(AiPolicyError::new(&p, "minimumSamples must be positive"));
            }
            if calibration.required {
                positive_duration(
                    calibration.frequency.as_deref().ok_or_else(|| {
                        AiPolicyError::new(&p, "required calibration needs frequency")
                    })?,
                    &p,
                )?;
            } else if let Some(frequency) = &calibration.frequency {
                positive_duration(frequency, &p)?;
            }
        }
        if let Some(decay) = &agent.confidence_decay {
            if let Some(half_life) = &decay.half_life {
                positive_duration(half_life, &p)?;
            }
            if decay.enabled && decay.half_life.is_none() && decay.triggers.is_empty() {
                return Err(AiPolicyError::new(
                    &p,
                    "enabled decay needs a halfLife or trigger",
                ));
            }
            for trigger in &decay.triggers {
                nonempty(&trigger.event, &p)?;
                if !valid_score(trigger.decay_factor) {
                    return Err(AiPolicyError::new(&p, "invalid decayFactor"));
                }
            }
        }
        if let Some(autonomy) = &agent.autonomy_policy {
            unique(autonomy.demotion.iter().map(|v| v.id.as_str()), &p)?;
            unique(autonomy.escalation.iter().map(|v| v.id.as_str()), &p)?;
            for rule in &autonomy.demotion {
                nonempty(&rule.condition, &p)?;
                if rule.target_level == AutonomyLevel::Autonomous {
                    return Err(AiPolicyError::new(
                        &p,
                        "demotion target cannot be autonomous",
                    ));
                }
            }
            for rule in &autonomy.escalation {
                nonempty(&rule.condition, &p)?;
                if rule.target_level == AutonomyLevel::Manual
                    || actors.get(rule.required_approver_ref.as_str()) != Some(&"human")
                {
                    return Err(AiPolicyError::new(
                        &p,
                        "escalation needs a nonmanual target and native human approver",
                    ));
                }
                positive_duration(&rule.escalation_expiry, &p)?;
            }
        }
        unique(
            agent.action_overrides.iter().map(|v| v.action_ref.as_str()),
            &p,
        )?;
        for (index, action) in agent.action_overrides.iter().enumerate() {
            let ap = format!("{p}/actionOverrides/{index}");
            policy.action(&action.action_ref)?;
            validate_floor(
                action.confidence_floor.as_ref(),
                &format!("{ap}/confidenceFloor"),
            )?;
            if let Some(window) = &action.review_window {
                positive_duration(window, &ap)?;
            }
            if action.autonomy == Some(AutonomyLevel::Supervisory) && action.review_window.is_none()
            {
                return Err(AiPolicyError::new(
                    &ap,
                    "supervisory action requires reviewWindow",
                ));
            }
        }
        validate_monitoring(agent, &p)?;
    }
    validate_fallbacks(&policy, &actors)?;
    Ok(Some(policy))
}
fn validate_monitoring(agent: &AgentDeclaration, pointer: &str) -> Result<(), AiPolicyError> {
    let Some(monitoring) = &agent.drift_monitoring else {
        return Ok(());
    };
    for monitor in &monitoring.monitors {
        positive_duration(&monitor.evaluation_window, pointer)?;
        for metric in &monitor.metrics {
            nonempty(&metric.name, pointer)?;
            if metric.threshold.is_some_and(|v| !v.is_finite()) {
                return Err(AiPolicyError::new(
                    pointer,
                    "metric threshold must be finite",
                ));
            }
        }
        if let Some(rubber) = &monitor.rubber_stamp {
            if let Some(duration) = &rubber.min_review_time {
                positive_duration(duration, pointer)?;
            }
            if rubber.max_agreement_rate.is_some_and(|v| !valid_score(v)) {
                return Err(AiPolicyError::new(pointer, "invalid agreement fraction"));
            }
            if rubber.enabled
                && rubber.min_review_time.is_none()
                && rubber.max_agreement_rate.is_none()
            {
                return Err(AiPolicyError::new(
                    pointer,
                    "enabled review monitoring requires a threshold",
                ));
            }
        }
        for alert in &monitor.alert_thresholds {
            nonempty(&alert.condition, pointer)?;
            if alert.action.is_some() == alert.policy_ref.is_some() {
                return Err(AiPolicyError::new(
                    pointer,
                    "alert requires exactly one action or policyRef",
                ));
            }
            if let Some(reference) = &alert.policy_ref {
                if !agent
                    .autonomy_policy
                    .as_ref()
                    .is_some_and(|a| a.demotion.iter().any(|r| &r.id == reference))
                {
                    return Err(AiPolicyError::new(
                        pointer,
                        "unresolved same-agent demotion policyRef",
                    ));
                }
            }
        }
    }
    if let Some(sequence) = &monitoring.deployment_sequence {
        for duration in [&sequence.shadow_duration, &sequence.canary_duration]
            .into_iter()
            .flatten()
        {
            positive_duration(duration, pointer)?;
        }
        if sequence
            .canary_percentage
            .is_some_and(|v| !v.is_finite() || v <= 0.0 || v >= 1.0)
        {
            return Err(AiPolicyError::new(
                pointer,
                "canaryPercentage must be between zero and one",
            ));
        }
        if sequence.enabled
            && (sequence.shadow_duration.is_none()
                || sequence.canary_duration.is_none()
                || sequence.canary_percentage.is_none())
        {
            return Err(AiPolicyError::new(
                pointer,
                "enabled deployment sequence requires all bounds",
            ));
        }
    }
    Ok(())
}
fn validate_fallbacks(
    policy: &NativeAiPolicy,
    actors: &BTreeMap<&str, &str>,
) -> Result<(), AiPolicyError> {
    let mut graph: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for agent in &policy.agents {
        let Some(chain) = policy.fallback_chain(agent) else {
            continue;
        };
        if chain.is_empty() {
            return Err(AiPolicyError::new(
                "/agents",
                "explicit fallback chain cannot be empty",
            ));
        }
        for (index, step) in chain.iter().enumerate() {
            let terminal = matches!(
                step,
                FallbackStep::Fail { .. } | FallbackStep::EscalateToHuman { .. }
            );
            if terminal != (index == chain.len() - 1) {
                return Err(AiPolicyError::new(
                    "/agents",
                    "fallback terminal must be last and only last",
                ));
            }
            match step {
                FallbackStep::Retry {
                    max_retries,
                    backoff,
                    initial_interval,
                    ..
                } => {
                    if *max_retries == 0 {
                        return Err(AiPolicyError::new("/agents", "maxRetries must be positive"));
                    }
                    let interval = positive_duration(initial_interval, "/agents")?;
                    let multiplier = match backoff {
                        BackoffStrategy::Fixed => 1,
                        BackoffStrategy::Linear => u64::from(*max_retries),
                        BackoffStrategy::Exponential => {
                            1_u64.checked_shl(*max_retries - 1).ok_or_else(|| {
                                AiPolicyError::new("/agents", "unrepresentable retry schedule")
                            })?
                        }
                    };
                    interval.checked_mul(multiplier).ok_or_else(|| {
                        AiPolicyError::new("/agents", "unrepresentable retry delay")
                    })?;
                }
                FallbackStep::AlternateAgent {
                    alternate_agent_ref,
                    ..
                } => {
                    let target = policy.agent(alternate_agent_ref).ok_or_else(|| {
                        AiPolicyError::new("/agents", "unresolved alternate agent")
                    })?;
                    for capability in &agent.capabilities {
                        if !target.capabilities.iter().any(|v| v.id == capability.id) {
                            return Err(AiPolicyError::new(
                                "/agents",
                                "alternate agent lacks selected capability",
                            ));
                        }
                    }
                    graph.entry(&agent.id).or_default().push(&target.id);
                }
                FallbackStep::EscalateToHuman {
                    task_action_ref, ..
                } => {
                    let action = policy.action(task_action_ref)?;
                    if action["action"] != "createTask" {
                        return Err(AiPolicyError::new(
                            task_action_ref,
                            "fallback action must be createTask",
                        ));
                    }
                    let human = action["assignTo"].as_str().ok_or_else(|| {
                        AiPolicyError::new(
                            task_action_ref,
                            "fallback requires explicit native human assignment",
                        )
                    })?;
                    if actors.get(human) != Some(&"human") {
                        return Err(AiPolicyError::new(
                            task_action_ref,
                            "fallback assignment must resolve to a native human actor",
                        ));
                    }
                }
                FallbackStep::Fail { .. } => {}
            }
        }
    }
    fn visit<'a>(
        node: &'a str,
        graph: &BTreeMap<&'a str, Vec<&'a str>>,
        stack: &mut BTreeSet<&'a str>,
        done: &mut BTreeSet<&'a str>,
    ) -> bool {
        if done.contains(node) {
            return true;
        }
        if !stack.insert(node) {
            return false;
        }
        for target in graph.get(node).into_iter().flatten() {
            if !visit(target, graph, stack, done) {
                return false;
            }
        }
        stack.remove(node);
        done.insert(node);
        true
    }
    let mut done = BTreeSet::new();
    for agent in &policy.agents {
        if !visit(&agent.id, &graph, &mut BTreeSet::new(), &mut done) {
            return Err(AiPolicyError::new(
                "/agents",
                "alternate-agent fallback cycle",
            ));
        }
    }
    Ok(())
}

/// Phase exercised explicitly by a trusted caller, never inferred from output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyPhase {
    InputRights,
    Output,
}
/// Supplied policy inputs from the processor or explicit test harness.
#[derive(Debug)]
pub struct PolicyExerciseContext<'a> {
    /// Admitted actor id.
    pub agent_id: &'a str,
    /// Selected actual lifecycle action, if established by the caller.
    pub action_ref: Option<&'a str>,
    /// Resolved input, distinct from the untrusted output.
    pub input: Option<&'a Value>,
    /// Actual native case snapshot.
    pub case_file: &'a Value,
    /// Untrusted proposed output; cannot supply authority or context.
    pub output: &'a Value,
    /// Untrusted explicit canonical report.
    pub confidence_report: Option<&'a Value>,
    /// A waiver request; no authenticated waiver operation exists here.
    pub waiver: Option<&'a Value>,
    /// Explicit observed phase.
    pub phase: PolicyPhase,
}
/// Observed rule result with its exact occurrence, not only its readable id.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleObservation {
    /// Scope and rule occurrence in the admitted source.
    pub pointer: String,
    /// Readable rule id, unique only within its scope.
    pub rule_id: String,
    /// Observed predicate/right result or exact failure.
    pub outcome: String,
    /// Violation effect if any.
    pub action: Option<ViolationAction>,
    /// Processor attribution for input rights, agent for output rules.
    pub attribution: String,
    /// Kind evaluated at this occurrence.
    pub kind: String,
}
/// Shared observed policy-stage result; never execution readiness or commit authority.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyEvaluation {
    /// Individual observed rules and floors, in native scope order.
    pub observations: Vec<RuleObservation>,
    /// Combined enforcement effect.
    pub effective_action: Option<ViolationAction>,
    /// Required evaluation failures, retained verbatim.
    pub errors: Vec<String>,
    /// Unsupported required context or operation.
    pub refusals: Vec<String>,
    /// Whether a processor obligation failed.
    pub processor_error: bool,
    /// Whether an unsupported waiver was requested.
    pub waiver_refused: bool,
    /// Original valid reported values, never computed replacements.
    pub confidence_report: Option<ConfidenceReport>,
}
impl PolicyEvaluation {
    /// Read the observed stage disposition, independently from readiness.
    pub fn disposition(&self) -> &'static str {
        if self.waiver_refused {
            return "waiverRefused";
        }
        if self.processor_error {
            return "processorError";
        }
        if !self.errors.is_empty() {
            return "evaluationError";
        }
        if !self.refusals.is_empty() {
            return "unsupported";
        }
        match self.effective_action {
            None => "pass",
            Some(ViolationAction::Flag) => "flag",
            Some(ViolationAction::SwitchToAssistive) => "switchToAssistive",
            Some(ViolationAction::EscalateToHuman) => "escalateToHuman",
            Some(ViolationAction::Reject) => "reject",
        }
    }
    /// Restrict the result without weakening earlier policy effects.
    pub fn restrict(&mut self, action: ViolationAction) {
        self.effective_action = Some(self.effective_action.map_or(action, |old| old.max(action)));
    }
}

fn native_rule_scopes(source: &Value) -> Vec<(String, &Value)> {
    let mut scopes = Vec::new();
    if let Some(rules) = source.pointer("/aiOversight/deonticConstraints") {
        scopes.push(("/aiOversight/deonticConstraints".into(), rules));
    }
    if let Some(agents) = source["agents"].as_array() {
        for (index, agent) in agents.iter().enumerate() {
            if let Some(rules) = agent.get("deonticConstraints") {
                scopes.push((format!("/agents/{index}/deonticConstraints"), rules));
            }
            if let Some(actions) = agent["actionOverrides"].as_array() {
                for (action_index, action) in actions.iter().enumerate() {
                    if let Some(rules) = action.get("deonticConstraints") {
                        scopes.push((
                            format!(
                                "/agents/{index}/actionOverrides/{action_index}/deonticConstraints"
                            ),
                            rules,
                        ));
                    }
                }
            }
        }
    }
    scopes
}
/// One original rule value and its exact native source locator.
#[derive(Debug)]
pub struct NativeRuleSource<'a> {
    /// JSON Pointer into the unchanged candidate.
    pub pointer: String,
    /// Original value; it may be an incomplete or invalid draft.
    pub value: &'a Value,
}
/// Locate raw native rule array entries without admission or interpretation.
///
/// Source navigation works on drafts. It never parses a variant, grants
/// readiness, follows metadata, or reads retired extension locations.
pub fn native_rule_sources(source: &Value) -> Vec<NativeRuleSource<'_>> {
    native_rule_scopes(source)
        .into_iter()
        .flat_map(|(pointer, rules)| {
            rules
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
                .map(move |(index, value)| NativeRuleSource {
                    pointer: format!("{pointer}/{index}"),
                    value,
                })
        })
        .collect()
}
