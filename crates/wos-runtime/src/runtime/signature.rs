// Rust guideline compliant 2026-02-21

//! Signature Profile runtime semantics.
//!
//! This module owns the WOS-side signing workflow behavior from the Signature
//! Profile. Ceremony providers remain adapters; the runtime consumes their
//! evidence through task responses and emits `SignatureAffirmation`
//! provenance when the profile requirements are satisfied.

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;

use fel_core::{evaluate, has_error_diagnostics, parse, types::FelValue};
use serde::{Deserialize, Serialize};
use wos_core::context::EvalContext;
use wos_core::instance::{ActiveTask, CaseInstance, PendingEvent};
use wos_core::provenance::{ProvenanceKind, ProvenanceRecord, SignatureAffirmationInput};

use crate::store::RuntimeRecord;

use super::{RuntimeError, TaskSubmissionResult, WosRuntime};

/// Extension key carrying a package-local Signature Profile key.
pub const SIGNATURE_PROFILE_KEY_EXTENSION: &str = "x-wos-signature-profile-key";

/// Extension key carrying a cross-artifact Signature Profile URI.
pub const SIGNATURE_PROFILE_REF_EXTENSION: &str = "x-wos-signature-profile-ref";

/// Extension key carrying the Signature Profile signing-step id.
pub const SIGNATURE_STEP_ID_EXTENSION: &str = "x-wos-signature-step-id";

const SIGNATURE_COMPLETIONS_EXTENSION: &str = "x-wos-signature-completions";
const SIGNATURE_ASSIGNMENTS_EXTENSION: &str = "x-wos-signature-assignments";

/// Vendor-extension token prefix. Mirrors the schema-side
/// `^x-[a-z][a-z0-9-]*$` pattern; lowercase-only by design — case-mismatched
/// tokens (e.g. `X-Foo`) are not vendor extensions and intentionally fall
/// through to the unknown-token branch in the comparators below.
pub const VENDOR_TOKEN_PREFIX: &str = "x-";

/// True when `token` carries the vendor-extension prefix that the schema
/// permits via `^x-[a-z]...`. Case-sensitive on purpose (see
/// [`VENDOR_TOKEN_PREFIX`]).
#[inline]
pub fn is_vendor_token(token: &str) -> bool {
    token.starts_with(VENDOR_TOKEN_PREFIX)
}

fn is_identity_method_vendor_token(token: &str) -> bool {
    // Match the schema's `^x-[a-z][a-z0-9-]*$` vendor-token shape.
    let Some(rest) = token.strip_prefix(VENDOR_TOKEN_PREFIX) else {
        return false;
    };
    let mut chars = rest.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

/// Validated signature authentication-method token.
///
/// Identity methods are the canonical WOS signature methods or an `x-*`
/// vendor token.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IdentityMethod(String);

impl IdentityMethod {
    fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        Self::validate(&value)?;
        Ok(Self(value))
    }

    fn validate(value: &str) -> Result<(), String> {
        if matches!(
            value,
            "none"
                | "email-otp"
                | "sms-otp"
                | "knowledge-based"
                | "oidc"
                | "webauthn"
                | "credential"
                | "in-person"
                | "notary"
        ) || is_identity_method_vendor_token(value)
        {
            Ok(())
        } else {
            Err(format!(
                "invalid signature identity method '{value}'; expected a canonical WOS method or an x-* vendor token"
            ))
        }
    }
}

impl AsRef<str> for IdentityMethod {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<IdentityMethod> for String {
    fn from(value: IdentityMethod) -> Self {
        value.0
    }
}

impl TryFrom<String> for IdentityMethod {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for IdentityMethod {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Signature content — the embedded `signature` block of a `$wosWorkflow`
/// document (ADR 0076 D-1). Represents the interior shape of the `signature`
/// block: signers, documents, signing flow, evidence, and policies. Type name
/// retained for consumer compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureProfileDocument {
    /// Optional schema URI.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,
    /// Target workflow declaration.
    #[serde(default)]
    pub target_workflow: SignatureTargetWorkflow,
    /// Optional profile version.
    #[serde(default)]
    pub version: Option<String>,
    /// Optional profile title.
    #[serde(default)]
    pub title: Option<String>,
    /// Optional profile description.
    #[serde(default)]
    pub description: Option<String>,
    /// Signature roles.
    pub roles: Vec<SignatureRole>,
    /// Signable documents.
    pub documents: Vec<SignatureDocument>,
    /// Signing flow declaration.
    pub signing_flow: SigningFlow,
    /// Evidence requirements.
    pub evidence: SignatureEvidence,
    /// Authentication policies.
    #[serde(default)]
    pub authentication_policies: Vec<AuthenticationPolicy>,
    /// Reminder policy.
    #[serde(default)]
    pub reminders: Option<ReminderPolicy>,
    /// Expiry policy.
    #[serde(default)]
    pub expiry_policy: Option<ExpiryPolicy>,
    /// Decline policy.
    #[serde(default)]
    pub decline_policy: Option<DeclinePolicy>,
    /// Void policy.
    #[serde(default)]
    pub void_policy: Option<VoidPolicy>,
    /// Reassignment policy.
    #[serde(default)]
    pub reassignment_policy: Option<ReassignmentPolicy>,
    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Target workflow declaration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureTargetWorkflow {
    /// Kernel document URL.
    #[serde(default)]
    pub url: String,
    /// Compatible kernel versions.
    #[serde(default)]
    pub compatible_versions: Option<String>,
}

/// Signature role declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureRole {
    /// Role id.
    pub id: String,
    /// Role literal.
    pub role: String,
    /// Bound kernel actor id.
    pub actor_id: String,
    /// Whether the role is required.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Authentication policy key.
    #[serde(default)]
    pub authentication_policy_key: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Signable document declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureDocument {
    /// Document id.
    pub id: String,
    /// Document URI.
    pub document_ref: String,
    /// Document digest.
    pub document_hash: String,
    /// Digest algorithm.
    pub document_hash_algorithm: String,
    /// Optional rendering URI.
    #[serde(default)]
    pub rendering_ref: Option<String>,
    /// Optional Formspec response URI.
    #[serde(default)]
    pub formspec_response_ref: Option<String>,
}

/// Signing flow declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningFlow {
    /// Flow type.
    #[serde(rename = "type")]
    pub flow_type: SigningFlowType,
    /// Ordered signing steps.
    pub steps: Vec<SigningStep>,
    /// Completion requirement.
    #[serde(default)]
    pub completion: Option<CompletionRequirement>,
}

/// Signing flow type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SigningFlowType {
    Sequential,
    Parallel,
    Routed,
    FreeForAll,
}

/// Signing step declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningStep {
    /// Step id.
    pub id: String,
    /// Role id.
    pub role_id: String,
    /// Document id.
    pub document_id: String,
    /// Routed-step guard.
    #[serde(default)]
    pub guard: Option<String>,
    /// Dependency step ids.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Whether this step is required.
    #[serde(default = "default_true")]
    pub required: bool,
}

/// Closed enum mirroring `signature.signingFlow.completion.type` in
/// `wos-workflow.schema.json` (lines 2358-2363). Vendor extensions live at
/// the sibling `signingFlow.x-*` extension surface, not as new completion
/// kinds — adding a kind requires a normative spec change. Unknown values
/// fail serde deserialization by design (no `#[serde(other)]`); upstream
/// schema validation catches this earlier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompletionRequirementKind {
    /// Every selected required step must complete.
    AllRequired,
    /// At least one selected required step must complete.
    AnyRequired,
    /// At least `count` selected required steps must complete.
    Count,
    /// Every role id in `role_ids` must have a completed step.
    RoleSet,
}

impl Default for CompletionRequirementKind {
    fn default() -> Self {
        Self::AllRequired
    }
}

/// Completion requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionRequirement {
    /// Requirement type.
    #[serde(rename = "type", default)]
    pub requirement_type: CompletionRequirementKind,
    /// Count for `count` requirements.
    #[serde(default)]
    pub count: Option<usize>,
    /// Role ids for `role-set` requirements.
    #[serde(default)]
    pub role_ids: Vec<String>,
}

/// Signature evidence requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureEvidence {
    /// Record kind, fixed to `signatureAffirmation`.
    pub record_kind: String,
    /// Required evidence paths.
    pub required_fields: Vec<String>,
    /// Consent-reference shape.
    pub consent_reference: ConsentReference,
    /// Identity-binding requirement.
    pub identity_binding: IdentityBindingRequirement,
    /// Whether emitted records are custody eligible.
    pub custody_hook_eligible: bool,
}

/// Consent-reference shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentReference {
    /// Consent text URI.
    pub consent_text_ref: String,
    /// Consent version.
    pub consent_version: String,
    /// Evidence path for acceptance timestamp.
    pub accepted_at_path: String,
    /// Evidence path for explicit affirmation.
    pub affirmation_path: String,
}

/// Identity-binding requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityBindingRequirement {
    /// Canonical authentication method or x-* vendor token.
    pub method: IdentityMethod,
    /// Assurance level.
    pub assurance_level: String,
    /// Provider URI.
    #[serde(default)]
    pub provider_ref: Option<String>,
    /// External attestation URI.
    #[serde(default)]
    pub external_attestation_ref: Option<String>,
}

/// Authentication policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationPolicy {
    /// Policy key.
    pub key: String,
    /// Canonical authentication method or x-* vendor token.
    pub method: IdentityMethod,
    /// Assurance level.
    pub assurance_level: String,
    /// Provider URI.
    #[serde(default)]
    pub provider_ref: Option<String>,
    /// Whether in-person evidence is required.
    #[serde(default)]
    pub requires_in_person: bool,
    /// Whether credential evidence is required.
    #[serde(default)]
    pub requires_credential_evidence: bool,
}

/// Reminder policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderPolicy {
    /// Event emitted by reminders.
    pub event_name: String,
    /// Reminder schedule.
    pub schedule: Vec<String>,
    /// Optional template key.
    #[serde(default)]
    pub template_key: Option<String>,
}

/// Expiry policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpiryPolicy {
    /// Event emitted on expiry.
    pub event_name: String,
    /// Duration before expiry.
    pub after: String,
    /// Optional transition id.
    #[serde(default)]
    pub transition_id: Option<String>,
}

/// Decline policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclinePolicy {
    /// Transition id or event name to follow.
    pub transition_id: String,
    /// Whether a reason is required.
    #[serde(default = "default_true")]
    pub reason_required: bool,
}

/// Void policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidPolicy {
    /// Actor ids authorized to void the flow.
    pub authorized_actor_ids: Vec<String>,
    /// Whether a reason is required.
    #[serde(default = "default_true")]
    pub reason_required: bool,
}

/// Reassignment policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReassignmentPolicy {
    /// Actor ids authorized to reassign.
    pub authorized_actor_ids: Vec<String>,
    /// Accountability mode.
    pub accountability: String,
    /// Whether a reason is required.
    #[serde(default = "default_true")]
    pub reason_required: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct SignatureProfileRegistration {
    pub(crate) key: Option<String>,
    pub(crate) profile_ref: Option<String>,
    pub(crate) profile: SignatureProfileDocument,
}

impl WosRuntime {
    /// Attach a package-local Signature Profile.
    #[must_use]
    pub fn with_signature_profile(
        mut self,
        key: impl Into<String>,
        profile: SignatureProfileDocument,
    ) -> Self {
        self.signature_profiles.push(SignatureProfileRegistration {
            key: Some(key.into()),
            profile_ref: None,
            profile,
        });
        self
    }

    /// Attach a cross-artifact Signature Profile.
    #[must_use]
    pub fn with_signature_profile_ref(
        mut self,
        profile_ref: impl Into<String>,
        profile: SignatureProfileDocument,
    ) -> Self {
        self.signature_profiles.push(SignatureProfileRegistration {
            key: None,
            profile_ref: Some(profile_ref.into()),
            profile,
        });
        self
    }

    pub(super) fn is_signature_task(task: &ActiveTask) -> bool {
        task.extensions
            .contains_key(SIGNATURE_PROFILE_KEY_EXTENSION)
            || task
                .extensions
                .contains_key(SIGNATURE_PROFILE_REF_EXTENSION)
    }

    pub(super) fn signature_affirmation_for_submission(
        &self,
        record: &RuntimeRecord,
        task: &ActiveTask,
        response: &serde_json::Value,
        actor_id: &str,
        signed_at_default: &str,
    ) -> Result<Option<ProvenanceRecord>, RuntimeError> {
        if !Self::is_signature_task(task) {
            return Ok(None);
        }

        let (profile_selector, profile) = self.signature_profile_for_task(task)?;
        let step_id = task_extension_str(task, SIGNATURE_STEP_ID_EXTENSION)
            .ok_or_else(|| RuntimeError::Signature("signature task missing step id".to_string()))?;
        let step = profile
            .signing_flow
            .steps
            .iter()
            .find(|candidate| candidate.id == step_id)
            .ok_or_else(|| {
                RuntimeError::Signature(format!("signature step '{step_id}' is not declared"))
            })?;
        let role = profile
            .roles
            .iter()
            .find(|candidate| candidate.id == step.role_id)
            .ok_or_else(|| {
                RuntimeError::Signature(format!(
                    "signature role '{}' is not declared",
                    step.role_id
                ))
            })?;
        let document = profile
            .documents
            .iter()
            .find(|candidate| candidate.id == step.document_id)
            .ok_or_else(|| {
                RuntimeError::Signature(format!(
                    "signature document '{}' is not declared",
                    step.document_id
                ))
            })?;

        self.ensure_step_can_complete(record, profile, step)?;
        self.ensure_required_evidence_present(record, response, &profile.evidence)?;
        self.ensure_consent_present(record, response, &profile.evidence.consent_reference)?;
        let identity_binding = self.identity_binding_for_submission(response, &profile.evidence)?;
        self.ensure_identity_satisfies_role(profile, role, &identity_binding)?;
        self.ensure_document_hash_matches(record, response, &profile.evidence, document)?;

        let signed_at = resolve_path(
            record,
            response,
            &profile.evidence.consent_reference.accepted_at_path,
        )
        .and_then(serde_json::Value::as_str)
        .unwrap_or(signed_at_default);
        let signer_id = response_string(response, "signerId").unwrap_or(actor_id);
        let signature_provider =
            response_string(response, "signatureProvider").unwrap_or("wos-runtime");
        let ceremony_id = response_string(response, "ceremonyId").unwrap_or(&task.task_id);
        let formspec_response_ref = response_string(response, "formspecResponseRef")
            .or(document.formspec_response_ref.as_deref())
            .ok_or_else(|| {
                RuntimeError::Signature(
                    "signature document or response must provide formspecResponseRef".to_string(),
                )
            })?;

        let (profile_ref, profile_key) = match &profile_selector {
            SignatureProfileSelector::Key(key) => (None, Some(key.as_str())),
            SignatureProfileSelector::Ref(profile_ref) => (Some(profile_ref.as_str()), None),
        };

        let record = ProvenanceRecord::signature_affirmation(SignatureAffirmationInput {
            signer_id,
            role_id: &role.id,
            role: &role.role,
            document_id: &document.id,
            document_hash: &document.document_hash,
            document_hash_algorithm: &document.document_hash_algorithm,
            signed_at,
            identity_binding,
            consent_reference: serde_json::to_value(&profile.evidence.consent_reference)
                .map_err(|error| RuntimeError::Signature(error.to_string()))?,
            signature_provider,
            ceremony_id,
            profile_ref,
            profile_key,
            formspec_response_ref,
            custody_hook_eligible: profile.evidence.custody_hook_eligible,
        });
        Ok(Some(record))
    }

    pub(super) fn record_signature_completion(
        &self,
        instance: &mut CaseInstance,
        task: &ActiveTask,
        signer_id: &str,
        signed_at: &str,
    ) -> Result<(), RuntimeError> {
        if !Self::is_signature_task(task) {
            return Ok(());
        }
        let (selector, profile) = self.signature_profile_for_task(task)?;
        let step_id = task_extension_str(task, SIGNATURE_STEP_ID_EXTENSION)
            .ok_or_else(|| RuntimeError::Signature("signature task missing step id".to_string()))?;
        let step = profile
            .signing_flow
            .steps
            .iter()
            .find(|candidate| candidate.id == step_id)
            .ok_or_else(|| RuntimeError::Signature(format!("unknown signature step {step_id}")))?;
        let completion = serde_json::json!({
            "profile": selector.as_string(),
            "stepId": step.id,
            "roleId": step.role_id,
            "documentId": step.document_id,
            "signerId": signer_id,
            "signedAt": signed_at,
        });
        let completions = instance
            .extensions
            .entry(SIGNATURE_COMPLETIONS_EXTENSION.to_string())
            .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        let serde_json::Value::Array(items) = completions else {
            return Err(RuntimeError::Signature(
                "signature completions extension must be an array".to_string(),
            ));
        };
        if items.iter().any(|item| {
            item.get("profile") == Some(&serde_json::Value::String(selector.as_string()))
                && item.get("stepId") == Some(&serde_json::Value::String(step.id.clone()))
        }) {
            return Err(RuntimeError::Signature(format!(
                "signature step '{}' already completed",
                step.id
            )));
        }
        items.push(completion);
        Ok(())
    }

    pub(super) fn signature_flow_complete_after(
        &self,
        instance: &CaseInstance,
        task: &ActiveTask,
    ) -> Result<bool, RuntimeError> {
        if !Self::is_signature_task(task) {
            return Ok(true);
        }
        let (_selector, profile) = self.signature_profile_for_task(task)?;
        let current_step_id = task_extension_str(task, SIGNATURE_STEP_ID_EXTENSION)
            .ok_or_else(|| RuntimeError::Signature("signature task missing step id".to_string()))?;
        let mut completed = completed_signature_steps(instance);
        completed.insert(current_step_id.to_string());

        let mut selected_required = Vec::new();
        for step in &profile.signing_flow.steps {
            if step.required && step_selected(profile, step, &instance.case_state)? {
                selected_required.push(step);
            }
        }

        let kind = profile
            .signing_flow
            .completion
            .as_ref()
            .map(|completion| completion.requirement_type)
            .unwrap_or_default();

        let complete = match kind {
            CompletionRequirementKind::AnyRequired => selected_required
                .iter()
                .any(|step| completed.contains(&step.id)),
            CompletionRequirementKind::Count => {
                let required_count = profile
                    .signing_flow
                    .completion
                    .as_ref()
                    .and_then(|completion| completion.count)
                    .unwrap_or(selected_required.len());
                selected_required
                    .iter()
                    .filter(|step| completed.contains(&step.id))
                    .count()
                    >= required_count
            }
            CompletionRequirementKind::RoleSet => {
                let role_ids = profile
                    .signing_flow
                    .completion
                    .as_ref()
                    .map(|completion| completion.role_ids.as_slice())
                    .unwrap_or(&[]);
                role_ids.iter().all(|role_id| {
                    profile
                        .signing_flow
                        .steps
                        .iter()
                        .any(|step| &step.role_id == role_id && completed.contains(&step.id))
                })
            }
            CompletionRequirementKind::AllRequired => selected_required
                .iter()
                .all(|step| completed.contains(&step.id)),
        };
        Ok(complete)
    }

    pub(super) fn handle_signature_non_completion(
        &mut self,
        record: &mut RuntimeRecord,
        task_index: usize,
        response: &serde_json::Value,
        actor_id: &str,
        now_iso: &str,
        status: &str,
    ) -> Result<Option<TaskSubmissionResult>, RuntimeError> {
        if !Self::is_signature_task(&record.instance.active_tasks[task_index]) {
            return Ok(None);
        }
        match status {
            "declined" => {
                let reason = response_string(response, "reason");
                self.require_signature_reason(
                    &record.instance.active_tasks[task_index],
                    reason,
                    SignatureReasonPolicy::Decline,
                )?;
                let task = record.instance.active_tasks.remove(task_index);
                let (_selector, profile) = self.signature_profile_for_task(&task)?;
                let signer_id = task.assigned_actor.clone();
                let document_id = signature_document_id_for_task(profile, &task);
                let emitted_event = profile.decline_policy.as_ref().map(|policy| {
                    let event = policy.transition_id.clone();
                    record.instance.pending_events.push(PendingEvent {
                        event: event.clone(),
                        actor_id: Some(actor_id.to_string()),
                        data: Some(serde_json::json!({
                            "taskId": task.task_id,
                            "reason": reason,
                        })),
                        timestamp: now_iso.to_string(),
                        idempotency_token: None,
                    });
                    event
                });
                self.push_signature_lifecycle_record(
                    record,
                    ProvenanceKind::TaskFailed,
                    &task,
                    actor_id,
                    now_iso,
                    serde_json::json!({
                        "signatureStatus": "declined",
                        "signerId": signer_id,
                        "documentId": document_id,
                        "reason": reason,
                    }),
                )?;
                return Ok(Some(TaskSubmissionResult::Failed {
                    code: "signatureDeclined".to_string(),
                    emitted_event,
                }));
            }
            "voided" => {
                let reason = response_string(response, "reason");
                let task = record.instance.active_tasks[task_index].clone();
                self.require_signature_reason(&task, reason, SignatureReasonPolicy::Void)?;
                self.ensure_signature_actor_authorized(&task, actor_id, SignaturePolicyKind::Void)?;
                let (selector, _profile) = self.signature_profile_for_task(&task)?;
                let task_count_before_void = record.instance.active_tasks.len();
                record
                    .instance
                    .active_tasks
                    .retain(|candidate| !selector.matches_task(candidate));
                let cancelled_task_count =
                    task_count_before_void.saturating_sub(record.instance.active_tasks.len());
                self.push_signature_lifecycle_record(
                    record,
                    ProvenanceKind::TaskDismissed,
                    &task,
                    actor_id,
                    now_iso,
                    serde_json::json!({
                        "signatureStatus": "voided",
                        "reason": reason,
                        "cancelledTaskCount": cancelled_task_count,
                    }),
                )?;
                return Ok(Some(TaskSubmissionResult::Failed {
                    code: "signatureVoided".to_string(),
                    emitted_event: None,
                }));
            }
            "reassigned" => {
                let reason = response_string(response, "reason");
                let new_actor = response_string(response, "newActorId").ok_or_else(|| {
                    RuntimeError::Signature(
                        "signature reassignment requires newActorId".to_string(),
                    )
                })?;
                let task = record.instance.active_tasks[task_index].clone();
                self.require_signature_reason(&task, reason, SignatureReasonPolicy::Reassignment)?;
                self.ensure_signature_actor_authorized(
                    &task,
                    actor_id,
                    SignaturePolicyKind::Reassignment,
                )?;
                record.instance.active_tasks[task_index].assigned_actor =
                    Some(new_actor.to_string());
                record.instance.active_tasks[task_index].updated_at = now_iso.to_string();
                let assignments = record
                    .instance
                    .extensions
                    .entry(SIGNATURE_ASSIGNMENTS_EXTENSION.to_string())
                    .or_insert_with(|| serde_json::Value::Array(Vec::new()));
                let serde_json::Value::Array(items) = assignments else {
                    return Err(RuntimeError::Signature(
                        "signature assignments extension must be an array".to_string(),
                    ));
                };
                items.push(serde_json::json!({
                    "taskId": task.task_id,
                    "originalSigner": task.assigned_actor,
                    "newSigner": new_actor,
                    "authorizedBy": actor_id,
                    "reason": reason,
                    "reassignedAt": now_iso,
                }));
                self.push_signature_lifecycle_record(
                    record,
                    ProvenanceKind::TaskCompleted,
                    &task,
                    actor_id,
                    now_iso,
                    serde_json::json!({
                        "signatureStatus": "reassigned",
                        "originalSigner": task.assigned_actor,
                        "newSigner": new_actor,
                        "authorizedBy": actor_id,
                        "reason": reason,
                    }),
                )?;
                return Ok(Some(TaskSubmissionResult::Completed {
                    artifact_id: format!("{}:reassignment", task.task_id),
                    case_mutated: false,
                    emitted_event: None,
                }));
            }
            "expired" => {
                let task = record.instance.active_tasks.remove(task_index);
                let (_selector, profile) = self.signature_profile_for_task(&task)?;
                let emitted_event = profile.expiry_policy.as_ref().map(|policy| {
                    let event = policy.event_name.clone();
                    record.instance.pending_events.push(PendingEvent {
                        event: event.clone(),
                        actor_id: Some(actor_id.to_string()),
                        data: Some(serde_json::json!({ "taskId": task.task_id })),
                        timestamp: now_iso.to_string(),
                        idempotency_token: None,
                    });
                    event
                });
                self.push_signature_lifecycle_record(
                    record,
                    ProvenanceKind::TaskFailed,
                    &task,
                    actor_id,
                    now_iso,
                    serde_json::json!({
                        "signatureStatus": "expired",
                    }),
                )?;
                return Ok(Some(TaskSubmissionResult::Failed {
                    code: "signatureExpired".to_string(),
                    emitted_event,
                }));
            }
            _ => {}
        }
        Ok(None)
    }

    pub(super) fn signature_expiry_records_for_event(
        &self,
        record: &mut RuntimeRecord,
        event_name: &str,
        actor_id: Option<&str>,
        now_iso: &str,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        let mut records = Vec::new();
        let actor_id = actor_id.unwrap_or("system");

        for registration in &self.signature_profiles {
            let Some(expiry_policy) = &registration.profile.expiry_policy else {
                continue;
            };
            if expiry_policy.event_name != event_name {
                continue;
            }

            let selector = if let Some(key) = &registration.key {
                SignatureProfileSelector::Key(key.clone())
            } else if let Some(profile_ref) = &registration.profile_ref {
                SignatureProfileSelector::Ref(profile_ref.clone())
            } else {
                continue;
            };

            let expiring_tasks: Vec<ActiveTask> = record
                .instance
                .active_tasks
                .iter()
                .filter(|task| selector.matches_task(task))
                .cloned()
                .collect();
            let Some(first_task) = expiring_tasks.first() else {
                continue;
            };

            let pending_signer_ids: Vec<String> = expiring_tasks
                .iter()
                .filter_map(|task| task.assigned_actor.clone())
                .collect();
            let expired_task_ids: Vec<String> = expiring_tasks
                .iter()
                .map(|task| task.task_id.clone())
                .collect();
            let mut expired_document_ids: Vec<String> = expiring_tasks
                .iter()
                .filter_map(|task| {
                    let step_id = task_extension_str(task, SIGNATURE_STEP_ID_EXTENSION)?;
                    registration
                        .profile
                        .signing_flow
                        .steps
                        .iter()
                        .find(|step| step.id == step_id)
                        .map(|step| step.document_id.clone())
                })
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            expired_document_ids.sort();

            record
                .instance
                .active_tasks
                .retain(|task| !selector.matches_task(task));

            let mut provenance = ProvenanceRecord::task_lifecycle(
                ProvenanceKind::TaskFailed,
                &first_task.task_id,
                Some(actor_id),
                Some(serde_json::json!({
                    "signatureStatus": "expired",
                    "profile": selector.as_string(),
                    "expiredTaskIds": expired_task_ids,
                    "pendingSignerIds": pending_signer_ids,
                    "expiredDocumentIds": expired_document_ids,
                })),
            );
            provenance.timestamp = now_iso.to_string();
            records.push(provenance);
        }

        Ok(records)
    }

    fn signature_profile_for_task(
        &self,
        task: &ActiveTask,
    ) -> Result<(SignatureProfileSelector, &SignatureProfileDocument), RuntimeError> {
        if let Some(key) = task_extension_str(task, SIGNATURE_PROFILE_KEY_EXTENSION) {
            let profile = self
                .signature_profiles
                .iter()
                .find(|registration| registration.key.as_deref() == Some(key))
                .map(|registration| &registration.profile)
                .ok_or_else(|| {
                    RuntimeError::Signature(format!("signature profile key '{key}' is not loaded"))
                })?;
            return Ok((SignatureProfileSelector::Key(key.to_string()), profile));
        }
        if let Some(profile_ref) = task_extension_str(task, SIGNATURE_PROFILE_REF_EXTENSION) {
            let profile = self
                .signature_profiles
                .iter()
                .find(|registration| registration.profile_ref.as_deref() == Some(profile_ref))
                .map(|registration| &registration.profile)
                .ok_or_else(|| {
                    RuntimeError::Signature(format!(
                        "signature profile ref '{profile_ref}' is not loaded"
                    ))
                })?;
            return Ok((
                SignatureProfileSelector::Ref(profile_ref.to_string()),
                profile,
            ));
        }
        Err(RuntimeError::Signature(
            "signature task missing profile key or ref".to_string(),
        ))
    }

    fn ensure_step_can_complete(
        &self,
        record: &RuntimeRecord,
        profile: &SignatureProfileDocument,
        step: &SigningStep,
    ) -> Result<(), RuntimeError> {
        if !step_selected(profile, step, &record.instance.case_state)? {
            return Err(RuntimeError::Signature(format!(
                "signature step '{}' is not selected by its guard",
                step.id
            )));
        }

        let completed = completed_signature_steps(&record.instance);
        for dependency in &step.depends_on {
            if !completed.contains(dependency) {
                return Err(RuntimeError::Signature(format!(
                    "signature step '{}' depends on incomplete step '{}'",
                    step.id, dependency
                )));
            }
        }

        if profile.signing_flow.flow_type == SigningFlowType::Sequential {
            for prior in &profile.signing_flow.steps {
                if prior.id == step.id {
                    break;
                }
                if prior.required
                    && step_selected(profile, prior, &record.instance.case_state)?
                    && !completed.contains(&prior.id)
                {
                    return Err(RuntimeError::Signature(format!(
                        "signature step '{}' is blocked by prior step '{}'",
                        step.id, prior.id
                    )));
                }
            }
        }
        Ok(())
    }

    fn ensure_required_evidence_present(
        &self,
        record: &RuntimeRecord,
        response: &serde_json::Value,
        evidence: &SignatureEvidence,
    ) -> Result<(), RuntimeError> {
        for path in &evidence.required_fields {
            let Some(value) = resolve_path(record, response, path) else {
                return Err(RuntimeError::Signature(format!(
                    "missing signature evidence field '{path}'"
                )));
            };
            if value.is_null() {
                return Err(RuntimeError::Signature(format!(
                    "signature evidence field '{path}' is null"
                )));
            }
        }
        Ok(())
    }

    fn ensure_consent_present(
        &self,
        record: &RuntimeRecord,
        response: &serde_json::Value,
        consent: &ConsentReference,
    ) -> Result<(), RuntimeError> {
        let accepted_at = resolve_path(record, response, &consent.accepted_at_path).and_then(|v| {
            v.as_str()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
        });
        if accepted_at.is_none() {
            return Err(RuntimeError::Signature(format!(
                "missing consent acceptance timestamp at '{}'",
                consent.accepted_at_path
            )));
        }

        let Some(affirmation) = resolve_path(record, response, &consent.affirmation_path) else {
            return Err(RuntimeError::Signature(format!(
                "missing consent affirmation at '{}'",
                consent.affirmation_path
            )));
        };
        if !truthy(affirmation) {
            return Err(RuntimeError::Signature(format!(
                "consent affirmation at '{}' is not true",
                consent.affirmation_path
            )));
        }
        Ok(())
    }

    fn identity_binding_for_submission(
        &self,
        response: &serde_json::Value,
        evidence: &SignatureEvidence,
    ) -> Result<serde_json::Value, RuntimeError> {
        if let Some(identity) = response_path(response, "identityBinding") {
            return Ok(identity.clone());
        }
        serde_json::to_value(&evidence.identity_binding)
            .map_err(|error| RuntimeError::Signature(error.to_string()))
    }

    fn ensure_identity_satisfies_role(
        &self,
        profile: &SignatureProfileDocument,
        role: &SignatureRole,
        identity_binding: &serde_json::Value,
    ) -> Result<(), RuntimeError> {
        let method = identity_binding
            .get("method")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| RuntimeError::Signature("identityBinding.method missing".to_string()))?;
        let method = IdentityMethod::new(method).map_err(RuntimeError::Signature)?;
        let assurance = identity_binding
            .get("assuranceLevel")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                RuntimeError::Signature("identityBinding.assuranceLevel missing".to_string())
            })?;

        if matches!(role.role.as_str(), "notary" | "in-person-signer")
            && !matches!(method.as_ref(), "notary" | "in-person")
        {
            return Err(RuntimeError::Signature(format!(
                "role '{}' requires in-person or notary authentication",
                role.id
            )));
        }

        if let Some(policy_key) = &role.authentication_policy_key {
            let policy = profile
                .authentication_policies
                .iter()
                .find(|candidate| &candidate.key == policy_key)
                .ok_or_else(|| {
                    RuntimeError::Signature(format!(
                        "authentication policy '{policy_key}' is not declared"
                    ))
                })?;
            identity_binding_meets_policy(policy, &method, assurance)?;
        }
        Ok(())
    }

    fn ensure_document_hash_matches(
        &self,
        record: &RuntimeRecord,
        response: &serde_json::Value,
        evidence: &SignatureEvidence,
        document: &SignatureDocument,
    ) -> Result<(), RuntimeError> {
        for field in &evidence.required_fields {
            if !field.ends_with("documentHash") {
                continue;
            }
            if let Some(value) = resolve_path(record, response, field)
                && let Some(hash) = value.as_str()
                && hash != document.document_hash
            {
                return Err(RuntimeError::Signature(format!(
                    "signature document hash '{hash}' does not match profile document '{}'",
                    document.id
                )));
            }
        }
        Ok(())
    }

    fn require_signature_reason(
        &self,
        task: &ActiveTask,
        reason: Option<&str>,
        policy: SignatureReasonPolicy,
    ) -> Result<(), RuntimeError> {
        let (_selector, profile) = self.signature_profile_for_task(task)?;
        let required = match policy {
            SignatureReasonPolicy::Decline => profile
                .decline_policy
                .as_ref()
                .is_none_or(|policy| policy.reason_required),
            SignatureReasonPolicy::Void => profile
                .void_policy
                .as_ref()
                .is_none_or(|policy| policy.reason_required),
            SignatureReasonPolicy::Reassignment => profile
                .reassignment_policy
                .as_ref()
                .is_none_or(|policy| policy.reason_required),
        };
        if required && reason.is_none_or(|value| value.trim().is_empty()) {
            return Err(RuntimeError::Signature(format!(
                "signature {} requires a reason",
                policy.as_status()
            )));
        }
        Ok(())
    }

    fn ensure_signature_actor_authorized(
        &self,
        task: &ActiveTask,
        actor_id: &str,
        policy_kind: SignaturePolicyKind,
    ) -> Result<(), RuntimeError> {
        let (_selector, profile) = self.signature_profile_for_task(task)?;
        let authorized = match policy_kind {
            SignaturePolicyKind::Void => profile
                .void_policy
                .as_ref()
                .map(|policy| policy.authorized_actor_ids.as_slice())
                .unwrap_or(&[]),
            SignaturePolicyKind::Reassignment => profile
                .reassignment_policy
                .as_ref()
                .map(|policy| policy.authorized_actor_ids.as_slice())
                .unwrap_or(&[]),
        };
        if !authorized.iter().any(|candidate| candidate == actor_id) {
            return Err(RuntimeError::Unauthorized(actor_id.to_string()));
        }
        Ok(())
    }

    fn push_signature_lifecycle_record(
        &self,
        record: &mut RuntimeRecord,
        kind: ProvenanceKind,
        task: &ActiveTask,
        actor_id: &str,
        now_iso: &str,
        details: serde_json::Value,
    ) -> Result<(), RuntimeError> {
        let mut provenance =
            ProvenanceRecord::task_lifecycle(kind, &task.task_id, Some(actor_id), Some(details));
        provenance.timestamp = now_iso.to_string();
        let kernel = self.resolver.resolve_kernel(
            &record.instance.definition_url,
            &record.instance.definition_version,
        )?;
        super::populate_provenance_record_fields(
            std::slice::from_mut(&mut provenance),
            &kernel,
            &record.instance.definition_version,
        );
        record.instance.provenance_position += 1;
        record.provenance_log.push(provenance);
        record.instance.updated_at = now_iso.to_string();
        Ok(())
    }
}

#[derive(Debug, Clone)]
enum SignatureProfileSelector {
    Key(String),
    Ref(String),
}

impl SignatureProfileSelector {
    fn as_string(&self) -> String {
        match self {
            Self::Key(key) | Self::Ref(key) => key.clone(),
        }
    }

    fn matches_task(&self, task: &ActiveTask) -> bool {
        match self {
            Self::Key(key) => {
                task_extension_str(task, SIGNATURE_PROFILE_KEY_EXTENSION) == Some(key.as_str())
            }
            Self::Ref(profile_ref) => {
                task_extension_str(task, SIGNATURE_PROFILE_REF_EXTENSION)
                    == Some(profile_ref.as_str())
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SignatureReasonPolicy {
    Decline,
    Void,
    Reassignment,
}

impl SignatureReasonPolicy {
    fn as_status(self) -> &'static str {
        match self {
            Self::Decline => "decline",
            Self::Void => "void",
            Self::Reassignment => "reassignment",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SignaturePolicyKind {
    Void,
    Reassignment,
}

fn default_true() -> bool {
    true
}

fn task_extension_str<'a>(task: &'a ActiveTask, key: &str) -> Option<&'a str> {
    task.extensions.get(key).and_then(serde_json::Value::as_str)
}

fn signature_document_id_for_task(
    profile: &SignatureProfileDocument,
    task: &ActiveTask,
) -> Option<String> {
    let step_id = task_extension_str(task, SIGNATURE_STEP_ID_EXTENSION)?;
    profile
        .signing_flow
        .steps
        .iter()
        .find(|step| step.id == step_id)
        .map(|step| step.document_id.clone())
}

fn response_string<'a>(response: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    response_path(response, key).and_then(serde_json::Value::as_str)
}

fn response_path<'a>(response: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    if let Some(data) = response.get("data")
        && let Some(value) = value_at_path(data, path)
    {
        return Some(value);
    }
    value_at_path(response, path)
}

fn resolve_path<'a>(
    record: &'a RuntimeRecord,
    response: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let path = path.trim();
    if let Some(rest) = path.strip_prefix("response.") {
        return response_path(response, rest);
    }
    if let Some(rest) = path.strip_prefix("caseFile.") {
        return value_at_path(&record.instance.case_state, rest);
    }
    response_path(response, path).or_else(|| value_at_path(&record.instance.case_state, path))
}

fn value_at_path<'a>(root: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = root;
    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }
        current = current.get(segment)?;
    }
    Some(current)
}

fn truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(value) => *value,
        serde_json::Value::String(value) => !value.trim().is_empty() && value != "false",
        serde_json::Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        serde_json::Value::Array(values) => !values.is_empty(),
        serde_json::Value::Object(values) => !values.is_empty(),
        serde_json::Value::Null => false,
    }
}

fn completed_signature_steps(instance: &CaseInstance) -> HashSet<String> {
    instance
        .extensions
        .get(SIGNATURE_COMPLETIONS_EXTENSION)
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("stepId").and_then(serde_json::Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn step_selected(
    profile: &SignatureProfileDocument,
    step: &SigningStep,
    case_state: &serde_json::Value,
) -> Result<bool, RuntimeError> {
    if profile.signing_flow.flow_type != SigningFlowType::Routed {
        return Ok(true);
    }
    let Some(guard) = &step.guard else {
        return Ok(true);
    };
    let case_map = case_state
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let ctx = EvalContext::from_case_state(&case_map, None);
    let parsed = parse(guard).map_err(|error| {
        RuntimeError::Signature(format!(
            "signature guard '{guard}' failed to parse: {error}"
        ))
    })?;
    let result = evaluate(&parsed, &ctx.to_fel_environment());
    if has_error_diagnostics(&result.diagnostics) {
        return Err(RuntimeError::Signature(format!(
            "signature guard '{guard}' produced evaluation errors"
        )));
    }
    Ok(matches!(result.value, FelValue::Boolean(true)))
}

fn assurance_rank(value: &str) -> u8 {
    match value {
        "none" => 0,
        "low" | "ial1" | "aal1" => 1,
        "standard" | "ial2" | "aal2" => 2,
        "high" | "ial3" | "aal3" => 3,
        "very-high" => 4,
        s if is_vendor_token(s) => 2,
        _ => 0,
    }
}

/// Enforce `method`, optional assurance floor, and in-person requirements against
/// a declared [`AuthenticationPolicy`].
///
/// **Deferred-strict-mode for vendor `x-*` assurance (fail-open posture).**
/// Signature Profile §2.13 floors are normative when posture × intent-URI is
/// declared; until the posture-floor table and Posture Declaration registry
/// land (parent PLN-0384), `x-*` assurance tokens have no portable ordering
/// against IAL/AAL or ranked tiers, so when either side is `x-*` the runtime
/// skips ordinal comparison and lets the binding through. **Product /
/// compliance impact:** affirmations can pass with vendor assurance that would
/// not ordinally meet a named tier; treat as explicit posture debt, not a
/// silent guarantee. **Close-out:** MUST flip to fail-closed once §2.13 admits
/// a vendor-floor declaration — tracked in workspace-root `T4-TODO.md`
/// (“Vendor `x-*` assurance floor enforcement”). Callers of this function
/// participate in signature affirmation gates; coordinate changes with
/// conformance SIG-013 and `identity_binding_meets_policy` unit tests below.
fn identity_binding_meets_policy(
    policy: &AuthenticationPolicy,
    method: &IdentityMethod,
    assurance: &str,
) -> Result<(), RuntimeError> {
    if method != &policy.method {
        return Err(RuntimeError::Signature(format!(
            "identity method '{}' does not satisfy policy '{}'",
            method.as_ref(),
            policy.key
        )));
    }
    let vendor_assurance = is_vendor_token(assurance) || is_vendor_token(&policy.assurance_level);
    if !vendor_assurance && assurance_rank(assurance) < assurance_rank(&policy.assurance_level) {
        return Err(RuntimeError::Signature(format!(
            "identity assurance '{assurance}' is below policy '{}'",
            policy.key
        )));
    }
    if policy.requires_in_person && !matches!(method.as_ref(), "notary" | "in-person") {
        return Err(RuntimeError::Signature(format!(
            "authentication policy '{}' requires in-person evidence",
            policy.key
        )));
    }
    Ok(())
}

pub(super) fn signed_at_for_response(
    record: &RuntimeRecord,
    task: &ActiveTask,
    response: &serde_json::Value,
    now_iso: &str,
    runtime: &WosRuntime,
) -> Result<String, RuntimeError> {
    if !WosRuntime::is_signature_task(task) {
        return Ok(now_iso.to_string());
    }
    let (_selector, profile) = runtime.signature_profile_for_task(task)?;
    Ok(resolve_path(
        record,
        response,
        &profile.evidence.consent_reference.accepted_at_path,
    )
    .and_then(serde_json::Value::as_str)
    .unwrap_or(now_iso)
    .to_string())
}

pub(super) fn signer_id_for_response<'a>(
    task: &'a ActiveTask,
    response: &'a serde_json::Value,
    actor_id: &'a str,
) -> &'a str {
    if WosRuntime::is_signature_task(task) {
        response_string(response, "signerId").unwrap_or(actor_id)
    } else {
        actor_id
    }
}

pub(super) fn append_signature_task_extensions(
    task: &mut ActiveTask,
    action_extensions: &HashMap<String, serde_json::Value>,
) {
    for (key, value) in action_extensions {
        task.extensions.insert(key.clone(), value.clone());
        if let Some(context) = task.context.as_mut() {
            context.extensions.insert(key.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod assurance_binding_tests {
    use super::*;

    fn identity_method(value: &str) -> IdentityMethod {
        IdentityMethod::new(value).unwrap()
    }

    fn policy(key: &str, method: &str, assurance_level: &str) -> AuthenticationPolicy {
        AuthenticationPolicy {
            key: key.to_string(),
            method: identity_method(method),
            assurance_level: assurance_level.to_string(),
            provider_ref: None,
            requires_in_person: false,
            requires_credential_evidence: false,
        }
    }

    #[test]
    fn identity_method_round_trips_canonical_and_vendor_tokens() {
        for value in ["email-otp", "x-acme-identity-method"] {
            let method = identity_method(value);
            let serialized = serde_json::to_value(&method).unwrap();
            assert_eq!(serialized, serde_json::json!(value));
            let roundtrip: IdentityMethod = serde_json::from_value(serialized).unwrap();
            assert_eq!(roundtrip, method);
        }
    }

    #[test]
    fn identity_binding_deserializes_canonical_and_vendor_methods() {
        let json = serde_json::json!({
            "method": "credential",
            "assuranceLevel": "ial2"
        });
        let binding: IdentityBindingRequirement = serde_json::from_value(json).unwrap();
        assert_eq!(binding.method.as_ref(), "credential");
        assert_eq!(binding.assurance_level, "ial2");
        let json2 = serde_json::json!({
            "method": "x-acme-credential",
            "assuranceLevel": "high"
        });
        let binding2: IdentityBindingRequirement = serde_json::from_value(json2).unwrap();
        assert_eq!(binding2.method.as_ref(), "x-acme-credential");
        assert_eq!(binding2.assurance_level, "high");
        let roundtrip: IdentityBindingRequirement =
            serde_json::from_value(serde_json::to_value(&binding).unwrap()).unwrap();
        assert_eq!(roundtrip.method, binding.method);
    }

    #[test]
    fn assurance_rank_orders_ial_against_ranked_policy() {
        assert!(super::assurance_rank("ial2") >= super::assurance_rank("standard"));
        assert!(super::assurance_rank("high") > super::assurance_rank("ial2"));
    }

    #[test]
    fn assurance_rank_known_tiers_and_unknown() {
        assert_eq!(super::assurance_rank("none"), 0);
        assert_eq!(super::assurance_rank("low"), 1);
        assert_eq!(super::assurance_rank("aal1"), 1);
        assert_eq!(super::assurance_rank("standard"), 2);
        assert_eq!(super::assurance_rank("ial2"), 2);
        assert_eq!(super::assurance_rank("x-vendor-tier"), 2);
        assert_eq!(super::assurance_rank("very-high"), 4);
        assert_eq!(super::assurance_rank("typo-tier"), 0);
    }

    #[test]
    fn identity_binding_meets_policy_method_mismatch_fails() {
        let p = policy("p1", "email-otp", "ial2");
        let err = super::identity_binding_meets_policy(&p, &identity_method("credential"), "ial2")
            .unwrap_err();
        assert!(err.to_string().contains("identity method"), "{err:?}");
    }

    #[test]
    fn identity_binding_meets_policy_assurance_below_floor_fails() {
        let p = policy("p1", "email-otp", "high");
        let method = identity_method("email-otp");
        assert!(super::identity_binding_meets_policy(&p, &method, "ial1").is_err());
        assert!(super::identity_binding_meets_policy(&p, &method, "low").is_err());
    }

    #[test]
    fn identity_binding_meets_policy_assurance_at_or_above_floor_ok() {
        let p = policy("p1", "email-otp", "standard");
        let method = identity_method("email-otp");
        assert!(super::identity_binding_meets_policy(&p, &method, "ial2").is_ok());
        assert!(super::identity_binding_meets_policy(&p, &method, "high").is_ok());
    }

    #[test]
    fn identity_binding_meets_policy_vendor_binding_skips_ordinal_vs_ranked_policy() {
        let p = policy("p1", "email-otp", "very-high");
        let method = identity_method("email-otp");
        assert!(super::identity_binding_meets_policy(&p, &method, "x-vendor-a").is_ok());
    }

    #[test]
    fn identity_binding_meets_policy_vendor_policy_skips_ordinal_vs_ranked_binding() {
        let p = policy("p1", "email-otp", "x-vendor-floor");
        let method = identity_method("email-otp");
        assert!(super::identity_binding_meets_policy(&p, &method, "ial1").is_ok());
    }

    #[test]
    fn identity_binding_meets_policy_requires_in_person_enforced() {
        let mut p = policy("p1", "email-otp", "ial1");
        p.requires_in_person = true;
        let method = identity_method("email-otp");
        assert!(super::identity_binding_meets_policy(&p, &method, "ial2").is_err());
        let mut notary_policy = policy("p2", "notary", "ial1");
        notary_policy.requires_in_person = true;
        let notary_method = identity_method("notary");
        assert!(
            super::identity_binding_meets_policy(&notary_policy, &notary_method, "ial1").is_ok()
        );
        let mut in_person_policy = policy("p3", "in-person", "ial1");
        in_person_policy.requires_in_person = true;
        let in_person_method = identity_method("in-person");
        assert!(
            super::identity_binding_meets_policy(&in_person_policy, &in_person_method, "ial1")
                .is_ok()
        );
    }
}
