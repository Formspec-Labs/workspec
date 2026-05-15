// Rust guideline compliant 2026-02-21

//! Signature Profile runtime semantics.
//!
//! This module owns the WOS-side signing workflow behavior from the Signature
//! Profile. Ceremony providers remain adapters; the runtime consumes their
//! evidence through task responses and emits `SignatureAffirmation`
//! provenance when the profile requirements are satisfied.

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::time::Duration;

use fel_core::{evaluate, has_error_diagnostics, parse, types::Value};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wos_core::context::EvalContext;
use wos_core::instance::{ActiveTask, PendingEvent, WorkflowProcess};
use wos_core::{ProvenanceKind, ProvenanceRecord, SignatureAffirmationInput};

use crate::binding::{
    SignatureAdmissionFailureReason as BindingAdmissionFailureReason,
    SignatureEvidence as VerifiedSignatureEvidence, SignaturePrimitiveStatus,
};
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
const POSTURE_DECLARATION_MAX_BYTES: u64 = 64 * 1024;
const POSTURE_DECLARATION_TIMEOUT_SECS: u64 = 5;

/// Resolved Posture Declaration bytes.
///
/// `source_uri` records the deployment-owned lookup target. `body` is the
/// exact JSON string whose bytes are hashed for runtime cache identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPostureDeclaration {
    /// URI supplied to the resolver.
    pub source_uri: String,
    /// Raw JSON body returned by the resolver.
    pub body: String,
}

impl ResolvedPostureDeclaration {
    /// Creates a resolved Posture Declaration body.
    #[must_use]
    pub fn new(source_uri: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            source_uri: source_uri.into(),
            body: body.into(),
        }
    }
}

/// Resolves Posture Declaration bytes for signature admission.
///
/// Hosts implement this trait to own posture-document lookup, freshness, and
/// network policy outside the admission hot path.
pub trait PostureResolver {
    /// Resolve a Posture Declaration URI into raw JSON bytes.
    ///
    /// # Errors
    /// Returns [`RuntimeError`] when the declaration cannot be resolved under
    /// the host's posture-resolution policy.
    fn resolve_posture_declaration(
        &self,
        posture_uri: &str,
    ) -> Result<ResolvedPostureDeclaration, RuntimeError>;
}

/// HTTP-backed Posture Declaration resolver.
///
/// This preserves the legacy allowlisted HTTP behavior behind an injectable
/// boundary. Production hosts should inject a deployment-owned resolver that
/// serves pinned bundle bytes or an equivalent bounded cache.
#[derive(Debug, Clone, Copy, Default)]
pub struct HttpPostureResolver;

impl PostureResolver for HttpPostureResolver {
    fn resolve_posture_declaration(
        &self,
        posture_uri: &str,
    ) -> Result<ResolvedPostureDeclaration, RuntimeError> {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(POSTURE_DECLARATION_TIMEOUT_SECS)))
            .build()
            .new_agent();
        let mut response = agent.get(posture_uri).call().map_err(|error| {
            RuntimeError::Signature(format!(
                "failed to fetch posture declaration from '{posture_uri}': {error}"
            ))
        })?;
        let body = response
            .body_mut()
            .with_config()
            .limit(POSTURE_DECLARATION_MAX_BYTES)
            .read_to_string()
            .map_err(|error| {
                RuntimeError::Signature(format!(
                    "failed to read posture declaration body from '{posture_uri}': {error}"
                ))
            })?;
        Ok(ResolvedPostureDeclaration::new(posture_uri, body))
    }
}

/// In-memory Posture Declaration resolver.
///
/// Tests and embedded deployments use this resolver when posture bytes are
/// already part of the trusted runtime bundle.
#[derive(Debug, Clone, Default)]
pub struct StaticPostureResolver {
    bodies_by_uri: HashMap<String, String>,
}

impl StaticPostureResolver {
    /// Creates an empty static posture resolver.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds raw Posture Declaration JSON for `posture_uri`.
    #[must_use]
    pub fn with_body(mut self, posture_uri: impl Into<String>, body: impl Into<String>) -> Self {
        self.bodies_by_uri.insert(posture_uri.into(), body.into());
        self
    }

    /// Creates a resolver from a parsed Posture Declaration.
    ///
    /// # Panics
    /// Panics only if serializing [`PostureDeclaration`] to JSON fails.
    #[must_use]
    pub fn from_declaration(declaration: PostureDeclaration) -> Self {
        let posture_uri = declaration.url.clone();
        let body = serde_json::to_string(&declaration)
            .expect("PostureDeclaration serialization is infallible");
        Self::new().with_body(posture_uri, body)
    }
}

impl PostureResolver for StaticPostureResolver {
    fn resolve_posture_declaration(
        &self,
        posture_uri: &str,
    ) -> Result<ResolvedPostureDeclaration, RuntimeError> {
        let body = self.bodies_by_uri.get(posture_uri).ok_or_else(|| {
            RuntimeError::Signature(format!(
                "posture declaration URI '{posture_uri}' is not loaded"
            ))
        })?;
        Ok(ResolvedPostureDeclaration::new(posture_uri, body.clone()))
    }
}

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
    /// Allowlist of deployment-local signing-intent URIs admitted by this
    /// workflow in addition to the §2.13.1 registered WOS set. Schema lint
    /// (and a runtime parse-time guard) rejects URIs in the reserved
    /// `urn:wos:signing-intent:*` namespace. Bridge until the WOS Posture
    /// Declaration object lands (PLN-0384).
    #[serde(default)]
    pub deployment_local_signing_intents: Vec<String>,
    /// Reference to the deployment's Posture Declaration (ADR-0090).
    /// Supersedes `deployment_local_signing_intents` when present.
    #[serde(default)]
    pub posture_policy: Option<PosturePolicyRef>,
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
    /// Optional source response URI.
    #[serde(default, alias = "formspecResponseRef")]
    pub source_response_ref: Option<String>,
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
    /// Expected WOS signing-intent URI for this step.
    #[serde(default)]
    pub signing_intent: Option<String>,
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
    /// Historical record kind, fixed to `signatureAffirmation`.
    #[serde(
        default = "default_signature_evidence_record_kind",
        skip_serializing_if = "is_signature_evidence_record_kind_default"
    )]
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

fn default_signature_evidence_record_kind() -> String {
    "signatureAffirmation".to_string()
}

fn is_signature_evidence_record_kind_default(value: &str) -> bool {
    value == "signatureAffirmation"
}

/// Outcome of signature admission for a task submission.
///
/// Carries the provenance record produced by admission alongside the verified
/// `signed_at` and `signer_id` derived from the binding-supplied evidence. The
/// completion path consumes the same `signed_at` that flowed into the
/// `SignatureAffirmation` record so the case-ledger entry and the
/// `x-wos-signature-completions` completion-state entry can never disagree for
/// a given signature event (review F4).
#[derive(Debug, Clone)]
pub struct SignatureAffirmationOutcome {
    /// Provenance record for `SignatureAffirmation`.
    pub record: ProvenanceRecord,
    /// Verified evidence timestamp, authoritative for the completion entry.
    pub signed_at: String,
    /// Verified signer id, authoritative for the completion entry.
    pub signer_id: String,
}

/// Outcome of a failed signature admission.
#[derive(Debug, Clone)]
pub struct SignatureAdmissionFailedOutcome {
    /// Machine-readable failure reason.
    pub reason: SignatureAdmissionFailedReason,
    /// Evidence identities that tie the failed admission to the source response,
    /// signed payload, signature, and signing intent.
    pub evidence_bindings: EvidenceBindings,
    /// Signer id from verified evidence (K2 carry-forward), when available.
    pub signer_id: Option<String>,
    /// Signer authority claim, when available.
    pub signer_authority: Option<serde_json::Value>,
    /// RFC 3339 timestamp when the admission was evaluated.
    pub emitted_at: String,
    /// Reason-specific context supplied by the failing gate.
    pub failure_context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Closed-enum reason for a signature admission failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureAdmissionFailedReason {
    /// The cryptographic signature primitive ran and rejected the signature.
    PrimitiveVerificationFailed,
    /// The identity method is not supported by any registered adapter.
    MethodUnsupported,
    /// The identity method is not registered in the WOS method registry.
    MethodUnregistered,
    /// Evidence fields in the response diverge from the verified binding evidence.
    EvidenceDivergence,
    /// The posture floor for this signing intent × method is not met.
    PostureFloorUnmet,
    /// The signing method is not recognized in the method registry.
    RegistryUnrecognizedMethod,
    /// The adapter required to verify this method is unavailable.
    AdapterUnavailable,
}

impl From<&BindingAdmissionFailureReason> for SignatureAdmissionFailedReason {
    fn from(reason: &BindingAdmissionFailureReason) -> Self {
        match reason {
            BindingAdmissionFailureReason::PrimitiveVerificationFailed => {
                Self::PrimitiveVerificationFailed
            }
            BindingAdmissionFailureReason::MethodUnsupported => Self::MethodUnsupported,
            BindingAdmissionFailureReason::MethodUnregistered => Self::MethodUnregistered,
            BindingAdmissionFailureReason::EvidenceDivergence => Self::EvidenceDivergence,
            BindingAdmissionFailureReason::PostureFloorUnmet => Self::PostureFloorUnmet,
            BindingAdmissionFailureReason::RegistryUnrecognizedMethod => {
                Self::RegistryUnrecognizedMethod
            }
            BindingAdmissionFailureReason::AdapterUnavailable => Self::AdapterUnavailable,
        }
    }
}

/// Evidence identities binding a failed admission to its source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBindings {
    /// The task response id that triggered admission evaluation.
    pub response_id: String,
    /// Digest of the signed payload verified by the binding.
    pub signed_payload_digest: String,
    /// Source-system signature identifier.
    pub signature_id: String,
    /// WOS signing-intent URI carried by the source evidence.
    pub signing_intent: String,
}

/// Top-level admission outcome — either admitted or rejected.
#[derive(Debug, Clone)]
pub enum AdmissionOutcome {
    /// Admission succeeded; the signature step is complete.
    Affirmation(SignatureAffirmationOutcome),
    /// Admission was rejected; the signature step is not complete.
    AdmissionFailed(SignatureAdmissionFailedOutcome),
}

fn admission_failed_outcome(
    task_id: &str,
    signature_evidence: &VerifiedSignatureEvidence,
    reason: SignatureAdmissionFailedReason,
    signer_id: Option<String>,
    emitted_at: &str,
    failure_context: Option<serde_json::Map<String, serde_json::Value>>,
) -> AdmissionOutcome {
    AdmissionOutcome::AdmissionFailed(SignatureAdmissionFailedOutcome {
        reason,
        evidence_bindings: EvidenceBindings {
            response_id: task_id.to_string(),
            signed_payload_digest: signature_evidence.signed_payload_digest.clone(),
            signature_id: signature_evidence.source_signature_id.clone(),
            signing_intent: signature_evidence.signing_intent.clone(),
        },
        signer_id,
        signer_authority: signature_evidence.signer_authority.clone(),
        emitted_at: emitted_at.to_string(),
        failure_context,
    })
}

fn admission_failure_context(
    field: &str,
    expected: &str,
    actual: &str,
) -> serde_json::Map<String, serde_json::Value> {
    serde_json::Map::from_iter([
        (
            "field".to_string(),
            serde_json::Value::String(field.to_string()),
        ),
        (
            "expected".to_string(),
            serde_json::Value::String(expected.to_string()),
        ),
        (
            "actual".to_string(),
            serde_json::Value::String(actual.to_string()),
        ),
    ])
}

fn signed_at_divergence_context(
    record: &RuntimeRecord,
    response: &serde_json::Value,
    consent_accepted_at_path: &str,
    evidence_signed_at: &str,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let consent_signed_at = resolve_path(record, response, consent_accepted_at_path)
        .and_then(serde_json::Value::as_str)?;
    if consent_signed_at.is_empty() || consent_signed_at == evidence_signed_at {
        return None;
    }
    Some(admission_failure_context(
        consent_accepted_at_path,
        evidence_signed_at,
        consent_signed_at,
    ))
}

/// Reference to a deployment's Posture Declaration document (ADR-0090).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PosturePolicyRef {
    pub url: String,
    #[serde(default)]
    pub version: Option<String>,
}

/// A loaded Posture Declaration controlling signature admission policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostureDeclaration {
    #[serde(rename = "$postureDeclaration")]
    pub version_marker: String,
    pub url: String,
    pub version: String,
    pub signature_policy: SignaturePolicy,
    #[serde(default)]
    pub jurisdictional_posture: Option<serde_json::Value>,
    #[serde(default)]
    pub custody_posture: Option<serde_json::Value>,
}

/// Signature admission policy from a Posture Declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignaturePolicy {
    pub allowed_methods: Vec<String>,
    pub minimum_primitive_verification: String,
    pub receipt_signing_required: bool,
    #[serde(default)]
    pub allowed_signing_intents: Vec<String>,
    #[serde(default)]
    pub revocation_policy: Option<serde_json::Value>,
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

    /// Attach a cached Posture Declaration.
    #[must_use]
    pub fn with_posture_declaration(self, declaration: PostureDeclaration) -> Self {
        self.with_posture_resolver(StaticPostureResolver::from_declaration(declaration))
    }

    /// Replace the Posture Declaration resolver.
    #[must_use]
    pub fn with_posture_resolver<R>(mut self, resolver: R) -> Self
    where
        R: PostureResolver + Send + Sync + 'static,
    {
        self.posture_resolver = Box::new(resolver);
        self.posture_declarations.borrow_mut().clear();
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
        signature_evidence: Option<&[VerifiedSignatureEvidence]>,
        actor_id: &str,
        signed_at_default: &str,
    ) -> Result<Option<AdmissionOutcome>, RuntimeError> {
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
        let signature_evidence = self.signature_evidence_for_submission(
            signature_evidence,
            actor_id,
            step,
            document,
            profile,
        )?;
        if let Some(failure) = signature_evidence.admission_failure.as_ref() {
            return Ok(Some(admission_failed_outcome(
                &task.task_id,
                &signature_evidence,
                SignatureAdmissionFailedReason::from(&failure.reason),
                signature_evidence.signer_id.clone(),
                signed_at_default,
                failure.failure_context.clone(),
            )));
        }
        if let Some(expected_intent) = &step.signing_intent
            && signature_evidence.signing_intent != *expected_intent
        {
            return Ok(Some(admission_failed_outcome(
                &task.task_id,
                &signature_evidence,
                SignatureAdmissionFailedReason::EvidenceDivergence,
                signature_evidence.signer_id.clone(),
                signed_at_default,
                Some(admission_failure_context(
                    "signingIntent",
                    expected_intent,
                    &signature_evidence.signing_intent,
                )),
            )));
        }
        if signature_evidence.document_hash != document.document_hash {
            return Ok(Some(admission_failed_outcome(
                &task.task_id,
                &signature_evidence,
                SignatureAdmissionFailedReason::EvidenceDivergence,
                signature_evidence.signer_id.clone(),
                signed_at_default,
                Some(admission_failure_context(
                    "documentHash",
                    &document.document_hash,
                    &signature_evidence.document_hash,
                )),
            )));
        }
        if signature_evidence.document_hash_algorithm != document.document_hash_algorithm {
            return Ok(Some(admission_failed_outcome(
                &task.task_id,
                &signature_evidence,
                SignatureAdmissionFailedReason::EvidenceDivergence,
                signature_evidence.signer_id.clone(),
                signed_at_default,
                Some(admission_failure_context(
                    "documentHashAlgorithm",
                    &document.document_hash_algorithm,
                    &signature_evidence.document_hash_algorithm,
                )),
            )));
        }
        if let Err(_reason) = ensure_signing_intent_admitted(
            &signature_evidence.signing_intent,
            &profile.deployment_local_signing_intents,
        ) {
            return Ok(Some(admission_failed_outcome(
                &task.task_id,
                &signature_evidence,
                SignatureAdmissionFailedReason::MethodUnsupported,
                signature_evidence.signer_id.clone(),
                signed_at_default,
                None,
            )));
        }
        let identity_binding = signature_evidence
            .identity_binding
            .clone()
            .map(Ok)
            .unwrap_or_else(|| self.identity_binding_for_submission(response, &profile.evidence))?;
        let identity_method = identity_binding
            .get("method")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("none")
            .to_string();
        let signature_method = signature_evidence
            .signature_method
            .as_deref()
            .unwrap_or(identity_method.as_str());
        self.ensure_identity_satisfies_role(profile, role, &identity_binding)?;
        self.ensure_identity_satisfies_signing_intent(
            &signature_evidence.signing_intent,
            &identity_binding,
        )?;
        self.ensure_document_hash_matches(record, response, &profile.evidence, document)?;

        let signed_at = if signature_evidence.signed_at.is_empty() {
            resolve_path(
                record,
                response,
                &profile.evidence.consent_reference.accepted_at_path,
            )
            .and_then(serde_json::Value::as_str)
            .unwrap_or(signed_at_default)
        } else {
            signature_evidence.signed_at.as_str()
        };
        // Source of truth = verified evidence record. If the response carries
        // a separate consent-path `signedAt` and it diverges from the
        // evidence's `signed_at`, WOS records an admission failure instead of
        // creating a conflicting completion entry.
        if let Some(failure_context) = signed_at_divergence_context(
            record,
            response,
            &profile.evidence.consent_reference.accepted_at_path,
            signed_at,
        ) {
            return Ok(Some(admission_failed_outcome(
                &task.task_id,
                &signature_evidence,
                SignatureAdmissionFailedReason::EvidenceDivergence,
                signature_evidence.signer_id.clone(),
                signed_at_default,
                Some(failure_context),
            )));
        }
        let signer_id = signature_evidence.signer_id.as_deref().unwrap_or(actor_id);
        if let Err(detail) = validate_signer_authority_detail(
            &signature_evidence.signing_intent,
            signature_evidence.signer_authority.as_ref(),
            Some(signed_at),
            Some(signer_id),
        ) {
            return Ok(Some(admission_failed_outcome(
                &task.task_id,
                &signature_evidence,
                SignatureAdmissionFailedReason::PostureFloorUnmet,
                Some(signer_id.to_string()),
                signed_at_default,
                Some(detail.into_failure_context()),
            )));
        }
        let signature_provider = signature_evidence
            .signature_provider
            .as_deref()
            .unwrap_or(&signature_evidence.source_system);
        let ceremony_id = signature_evidence
            .ceremony_id
            .as_deref()
            .unwrap_or(&task.task_id);
        let source_response_ref = signature_evidence
            .source_response_ref
            .as_deref()
            .or_else(|| response_string(response, "sourceResponseRef"))
            .or_else(|| response_string(response, "formspecResponseRef"))
            .or(document.source_response_ref.as_deref())
            .ok_or_else(|| {
                RuntimeError::Signature(
                    "signature document, response, or evidence must provide sourceResponseRef"
                        .to_string(),
                )
            })?;

        let (profile_ref, profile_key) = match &profile_selector {
            SignatureProfileSelector::Key(key) => (None, Some(key.as_str())),
            SignatureProfileSelector::Ref(profile_ref) => (Some(profile_ref.as_str()), None),
        };

        let (signing_act_id, presentation_hash) =
            signature_affirmation_k2_fields(&signature_evidence);
        let provenance_record =
            ProvenanceRecord::signature_affirmation(SignatureAffirmationInput {
                signer_id,
                role_id: &role.id,
                role: &role.role,
                document_id: &document.id,
                signing_act_id,
                document_ref: serde_json::json!({
                    "documentId": document.id.as_str(),
                    "locale": "und",
                }),
                document_hash: &document.document_hash,
                presentation_hash,
                document_hash_algorithm: &document.document_hash_algorithm,
                source_signature_system: &signature_evidence.source_system,
                source_signature_id: &signature_evidence.source_signature_id,
                signed_payload_digest: &signature_evidence.signed_payload_digest,
                signed_payload_digest_algorithm: &signature_evidence
                    .signed_payload_digest_algorithm,
                signing_intent: &signature_evidence.signing_intent,
                signed_at,
                identity_binding,
                consent_reference: serde_json::to_value(&profile.evidence.consent_reference)
                    .map_err(|error| RuntimeError::Signature(error.to_string()))?,
                signature_provider,
                ceremony_id,
                profile_ref,
                profile_key,
                source_response_ref,
                signer_authority: signature_evidence.signer_authority.clone(),
                custody_hook_eligible: profile.evidence.custody_hook_eligible,
                primitive_verification: serde_json::to_value(
                    &signature_evidence.primitive_verification,
                )
                .map_err(|error| RuntimeError::Signature(error.to_string()))?,
                verification_receipt: signature_evidence.verification_receipt.as_deref(),
                witnessed_signature_ref: None,
            });
        let signed_at_owned = signed_at.to_string();
        let signer_id_owned = signer_id.to_string();

        // Posture Declaration checks (ADR-0090).
        if let Some(posture_ref) = &profile.posture_policy {
            let posture = self.load_posture_declaration(posture_ref)?;
            if !posture
                .signature_policy
                .allowed_methods
                .iter()
                .any(|m| m == signature_method)
            {
                return Ok(Some(admission_failed_outcome(
                    &task.task_id,
                    &signature_evidence,
                    SignatureAdmissionFailedReason::MethodUnsupported,
                    Some(signer_id_owned),
                    signed_at_default,
                    None,
                )));
            }
            if !posture.signature_policy.allowed_signing_intents.is_empty()
                && !posture
                    .signature_policy
                    .allowed_signing_intents
                    .iter()
                    .any(|intent| intent == &signature_evidence.signing_intent)
            {
                return Ok(Some(admission_failed_outcome(
                    &task.task_id,
                    &signature_evidence,
                    SignatureAdmissionFailedReason::MethodUnsupported,
                    Some(signer_id_owned),
                    signed_at_default,
                    None,
                )));
            }
            match &signature_evidence.primitive_verification {
                SignaturePrimitiveStatus::Verified => {}
                SignaturePrimitiveStatus::DeferredPendingHelper { .. } => {
                    if posture.signature_policy.minimum_primitive_verification == "verified" {
                        return Ok(Some(admission_failed_outcome(
                            &task.task_id,
                            &signature_evidence,
                            SignatureAdmissionFailedReason::PostureFloorUnmet,
                            Some(signer_id_owned),
                            signed_at_default,
                            None,
                        )));
                    }
                }
                SignaturePrimitiveStatus::Failed { .. } => {}
            }
            if posture.signature_policy.receipt_signing_required
                && signature_evidence
                    .verification_receipt
                    .as_deref()
                    .is_none_or(str::is_empty)
            {
                return Ok(Some(admission_failed_outcome(
                    &task.task_id,
                    &signature_evidence,
                    SignatureAdmissionFailedReason::PostureFloorUnmet,
                    Some(signer_id_owned),
                    signed_at_default,
                    Some(admission_failure_context(
                        "verificationReceipt",
                        "signed receipt bytes",
                        "missing",
                    )),
                )));
            }
            // TODO(Phase 3.3): enforce revocation_policy when present
        }

        match &signature_evidence.primitive_verification {
            SignaturePrimitiveStatus::Verified => Ok(Some(AdmissionOutcome::Affirmation(
                SignatureAffirmationOutcome {
                    record: provenance_record,
                    signed_at: signed_at_owned,
                    signer_id: signer_id_owned,
                },
            ))),
            SignaturePrimitiveStatus::DeferredPendingHelper { .. } => Ok(Some(
                AdmissionOutcome::Affirmation(SignatureAffirmationOutcome {
                    record: provenance_record,
                    signed_at: signed_at_owned,
                    signer_id: signer_id_owned,
                }),
            )),
            SignaturePrimitiveStatus::Failed { .. } => Ok(Some(admission_failed_outcome(
                &task.task_id,
                &signature_evidence,
                SignatureAdmissionFailedReason::PrimitiveVerificationFailed,
                Some(signer_id_owned),
                signed_at_default,
                None,
            ))),
        }
    }

    pub(super) fn record_signature_completion(
        &self,
        instance: &mut WorkflowProcess,
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
        instance: &WorkflowProcess,
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
        if !Self::is_signature_task(&record.process.active_tasks[task_index]) {
            return Ok(None);
        }
        match status {
            "declined" => {
                let reason = response_string(response, "reason");
                self.require_signature_reason(
                    &record.process.active_tasks[task_index],
                    reason,
                    SignatureReasonPolicy::Decline,
                )?;
                let task = record.process.active_tasks.remove(task_index);
                let (_selector, profile) = self.signature_profile_for_task(&task)?;
                let signer_id = task.assigned_actor.clone();
                let document_id = signature_document_id_for_task(profile, &task);
                let emitted_event = profile.decline_policy.as_ref().map(|policy| {
                    let event = policy.transition_id.clone();
                    record.process.pending_events.push(PendingEvent {
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
                let task = record.process.active_tasks[task_index].clone();
                self.require_signature_reason(&task, reason, SignatureReasonPolicy::Void)?;
                self.ensure_signature_actor_authorized(&task, actor_id, SignaturePolicyKind::Void)?;
                let (selector, _profile) = self.signature_profile_for_task(&task)?;
                let task_count_before_void = record.process.active_tasks.len();
                record
                    .process
                    .active_tasks
                    .retain(|candidate| !selector.matches_task(candidate));
                let cancelled_task_count =
                    task_count_before_void.saturating_sub(record.process.active_tasks.len());
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
                let task = record.process.active_tasks[task_index].clone();
                self.require_signature_reason(&task, reason, SignatureReasonPolicy::Reassignment)?;
                self.ensure_signature_actor_authorized(
                    &task,
                    actor_id,
                    SignaturePolicyKind::Reassignment,
                )?;
                record.process.active_tasks[task_index].assigned_actor =
                    Some(new_actor.to_string());
                record.process.active_tasks[task_index].updated_at = now_iso.to_string();
                let assignments = record
                    .process
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
                let task = record.process.active_tasks.remove(task_index);
                let (_selector, profile) = self.signature_profile_for_task(&task)?;
                let emitted_event = profile.expiry_policy.as_ref().map(|policy| {
                    let event = policy.event_name.clone();
                    record.process.pending_events.push(PendingEvent {
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
                .process
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
                .process
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
        if !step_selected(profile, step, &record.process.case_state)? {
            return Err(RuntimeError::Signature(format!(
                "signature step '{}' is not selected by its guard",
                step.id
            )));
        }

        let completed = completed_signature_steps(&record.process);
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
                    && step_selected(profile, prior, &record.process.case_state)?
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

    fn signature_evidence_for_submission(
        &self,
        signature_evidence: Option<&[VerifiedSignatureEvidence]>,
        actor_id: &str,
        _step: &SigningStep,
        document: &SignatureDocument,
        _profile: &SignatureProfileDocument,
    ) -> Result<VerifiedSignatureEvidence, RuntimeError> {
        let evidence = signature_evidence.ok_or_else(|| {
            RuntimeError::Signature(
                "signature binding did not provide verified signature evidence".to_string(),
            )
        })?;
        let mut matches = evidence.iter().filter(|candidate| {
            candidate.document_id == document.id
                && candidate
                    .signer_id
                    .as_deref()
                    .is_none_or(|signer_id| signer_id == actor_id)
        });

        let signature = matches.next().ok_or_else(|| {
            RuntimeError::Signature(format!(
                "no verified signature evidence matched document '{}' and signer '{}'",
                document.id, actor_id
            ))
        })?;
        if matches.next().is_some() {
            return Err(RuntimeError::Signature(format!(
                "multiple verified signature evidence records matched document '{}' and signer '{}'",
                document.id, actor_id
            )));
        }

        Ok(signature.clone())
    }

    fn ensure_identity_satisfies_signing_intent(
        &self,
        signing_intent: &str,
        identity_binding: &serde_json::Value,
    ) -> Result<(), RuntimeError> {
        let method = identity_binding
            .get("method")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| RuntimeError::Signature("identityBinding.method missing".to_string()))?;
        match signing_intent {
            "urn:wos:signing-intent:notarial-attestation" => {
                if !matches!(method, "notary" | "in-person") {
                    return Err(RuntimeError::Signature(
                        "notarial signing intent requires notary or in-person authentication"
                            .to_string(),
                    ));
                }
            }
            "urn:wos:signing-intent:agent-as-attorney-in-fact"
            | "urn:wos:signing-intent:agent-as-officer" => {
                if !matches!(method, "oidc" | "webauthn" | "credential") {
                    return Err(RuntimeError::Signature(format!(
                        "signing intent '{signing_intent}' requires oidc, webauthn, or credential authentication"
                    )));
                }
            }
            _ => {
                if method == "none" {
                    return Err(RuntimeError::Signature(format!(
                        "signing intent '{signing_intent}' requires an authentication method"
                    )));
                }
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
            &record.process.definition_url,
            &record.process.definition_version,
        )?;
        super::populate_provenance_record_fields(
            std::slice::from_mut(&mut provenance),
            &kernel,
            &record.process.definition_version,
        );
        record.process.provenance_position += 1;
        record.provenance_log.push(provenance);
        record.process.updated_at = now_iso.to_string();
        Ok(())
    }

    /// Load a Posture Declaration from the configured resolver.
    pub(crate) fn load_posture_declaration(
        &self,
        posture_ref: &PosturePolicyRef,
    ) -> Result<PostureDeclaration, RuntimeError> {
        let posture_uri = posture_ref.url.as_str();
        if !posture_uri_allowed(posture_uri) {
            return Err(RuntimeError::Signature(format!(
                "posture declaration URI '{posture_uri}' is not allowed; expected https URL or loopback http URL"
            )));
        }
        let resolved = self
            .posture_resolver
            .resolve_posture_declaration(posture_uri)?;
        let cache_key = posture_declaration_cache_key(&resolved.body);
        {
            let cache = self.posture_declarations.borrow();
            if let Some(cached) = cache.get(&cache_key) {
                validate_posture_declaration(posture_ref, cached)?;
                return Ok(cached.clone());
            }
        }
        let declaration: PostureDeclaration =
            serde_json::from_str(&resolved.body).map_err(|error| {
                RuntimeError::Signature(format!(
                    "failed to parse posture declaration from '{posture_uri}': {error}"
                ))
            })?;
        validate_posture_declaration(posture_ref, &declaration)?;
        self.posture_declarations
            .borrow_mut()
            .insert(cache_key, declaration.clone());
        Ok(declaration)
    }
}

fn posture_declaration_cache_key(body: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(body.as_bytes()))
}

fn validate_posture_declaration(
    posture_ref: &PosturePolicyRef,
    declaration: &PostureDeclaration,
) -> Result<(), RuntimeError> {
    if declaration.url != posture_ref.url {
        return Err(RuntimeError::Signature(format!(
            "posture declaration URL mismatch: expected '{}', got '{}'",
            posture_ref.url, declaration.url
        )));
    }
    if let Some(expected_version) = posture_ref.version.as_deref()
        && declaration.version != expected_version
    {
        return Err(RuntimeError::Signature(format!(
            "posture declaration version mismatch for '{}': expected '{}', got '{}'",
            posture_ref.url, expected_version, declaration.version
        )));
    }
    Ok(())
}

fn posture_uri_allowed(uri: &str) -> bool {
    if let Some(rest) = uri.strip_prefix("https://") {
        return uri_host(rest).is_some_and(|host| !host.is_empty());
    }
    if let Some(rest) = uri.strip_prefix("http://") {
        return uri_host(rest)
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]"));
    }
    false
}

fn uri_host(uri_without_scheme: &str) -> Option<&str> {
    let authority = uri_without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    if authority.starts_with('[') {
        let end = authority.find(']')?;
        return Some(&authority[..=end]);
    }
    Some(authority.split(':').next().unwrap_or_default())
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

/// K-2 signing-act and presentation digests for `SignatureAffirmation` minting.
fn signature_affirmation_k2_fields(evidence: &VerifiedSignatureEvidence) -> (&str, &str) {
    (
        evidence.effective_signing_act_id(),
        evidence.effective_presentation_hash(),
    )
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

/// Admit a signingIntent if it is in the §2.11.1 registered WOS set OR in
/// this workflow's `deploymentLocalSigningIntents` allowlist (§2.11.2 bridge
/// until the WOS Posture Declaration object lands, PLN-0384). Reject
/// otherwise — fail-closed.
fn ensure_signing_intent_admitted(
    signing_intent: &str,
    deployment_local_signing_intents: &[String],
) -> Result<(), RuntimeError> {
    if is_registered_signing_intent(signing_intent) {
        return Ok(());
    }
    if deployment_local_signing_intents
        .iter()
        .any(|allowed| allowed == signing_intent)
    {
        return Ok(());
    }
    Err(RuntimeError::Signature(format!(
        "signingIntent '{signing_intent}' is neither registered (urn:wos:signing-intent:*) nor in this workflow's deploymentLocalSigningIntents allowlist"
    )))
}

fn is_registered_signing_intent(signing_intent: &str) -> bool {
    matches!(
        signing_intent,
        "urn:wos:signing-intent:applicant-signature"
            | "urn:wos:signing-intent:counter-signature"
            | "urn:wos:signing-intent:witness-attestation"
            | "urn:wos:signing-intent:notarial-attestation"
            | "urn:wos:signing-intent:consent"
            | "urn:wos:signing-intent:attestation-of-fact"
            | "urn:wos:signing-intent:agent-as-attorney-in-fact"
            | "urn:wos:signing-intent:agent-as-officer"
            | "urn:wos:signing-intent:approval"
            | "urn:wos:signing-intent:certified-receipt"
    )
}

fn required_authority_classes(signing_intent: &str) -> Option<&'static [&'static str]> {
    match signing_intent {
        "urn:wos:signing-intent:witness-attestation" => Some(&["witness"]),
        "urn:wos:signing-intent:notarial-attestation" => Some(&["notary-commissioned"]),
        "urn:wos:signing-intent:agent-as-attorney-in-fact" => Some(&["as-attorney-in-fact"]),
        "urn:wos:signing-intent:agent-as-officer" => Some(&["as-officer-of"]),
        _ => None,
    }
}

/// Classes that, per Signature Profile §2.12.2, are non-`self` and therefore
/// require backing evidence (commission, power-of-attorney, board resolution).
/// `self` is the only floor for which the WOS center admits a claim with no
/// supporting `evidenceBinding`/`authoritySource` — every other class
/// represents capacity-to-bind that must trace to a verifiable instrument.
fn is_non_self_class(class: &str) -> bool {
    matches!(
        class,
        "witness" | "notary-commissioned" | "as-attorney-in-fact" | "as-officer-of"
    ) || class.starts_with("x-")
}

/// Classes whose §2.12.2 row marks `principal` as REQUIRED (delegating
/// classes — the signer is binding someone else's interest). Self-delegation
/// is rejected: `principal` MUST NOT equal `signerId`.
fn class_requires_principal(class: &str) -> bool {
    matches!(class, "as-officer-of" | "as-attorney-in-fact")
}

/// Classes whose §2.12.2 row marks `authoritySource` as REQUIRED — the URI
/// of the instrument that grants the capacity (commission certificate,
/// executed power-of-attorney, board resolution).
fn class_requires_authority_source(class: &str) -> bool {
    matches!(
        class,
        "as-officer-of" | "as-attorney-in-fact" | "notary-commissioned"
    )
}

/// Permitted `evidenceBinding.evidenceHashAlgorithm` per §2.7. `sha-256`
/// is REQUIRED for Core conformance; future profile revisions or `x-*`
/// extension policies MAY widen this set.
fn evidence_hash_algorithm_permitted(algorithm: &str) -> bool {
    algorithm == "sha-256"
}

/// Full §2.12.4 signer-authority validation. Replaces the prior class-only
/// check that admitted any `notary-commissioned` claim regardless of
/// authoritySource / principal / evidenceBinding presence.
///
/// Steps mirror §2.12.4:
///
/// 1. If the intent's authority floor is `self`, an absent `signerAuthority`
///    is admissible; a present claim is still validated for shape.
/// 2. If the floor is non-`self`, an absent `signerAuthority` is rejected.
/// 3. `class` must satisfy the floor (existing behavior preserved).
/// 4. For non-`self` classes: `evidenceBinding.evidenceHash` is REQUIRED and
///    its algorithm must be permitted by §2.7.
/// 5. For classes that mandate `authoritySource` (§2.12.2): authoritySource
///    must be present, non-empty.
/// 6. For delegating classes (`as-officer-of`, `as-attorney-in-fact`):
///    `principal` is REQUIRED and MUST NOT equal `signerId`.
/// 7. `validFrom` / `validUntil` (when present) bracket `signedAt`.
#[cfg(test)]
fn validate_signer_authority(
    signing_intent: &str,
    signer_authority: Option<&serde_json::Value>,
    signed_at: Option<&str>,
    signer_id: Option<&str>,
) -> Result<(), RuntimeError> {
    validate_signer_authority_detail(signing_intent, signer_authority, signed_at, signer_id)
        .map_err(SignatureAdmissionFailureDetail::into_runtime_error)
}

#[derive(Debug, Clone)]
struct SignatureAdmissionFailureDetail {
    field: &'static str,
    message: String,
}

impl SignatureAdmissionFailureDetail {
    fn new(field: &'static str, message: String) -> Self {
        Self { field, message }
    }

    #[cfg(test)]
    fn into_runtime_error(self) -> RuntimeError {
        RuntimeError::Signature(self.message)
    }

    fn into_failure_context(self) -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::from_iter([
            (
                "field".to_string(),
                serde_json::Value::String(self.field.to_string()),
            ),
            (
                "message".to_string(),
                serde_json::Value::String(self.message),
            ),
        ])
    }
}

fn validate_signer_authority_detail(
    signing_intent: &str,
    signer_authority: Option<&serde_json::Value>,
    signed_at: Option<&str>,
    signer_id: Option<&str>,
) -> Result<(), SignatureAdmissionFailureDetail> {
    let allowed_classes = required_authority_classes(signing_intent);

    let Some(authority) = signer_authority else {
        if allowed_classes.is_some() {
            return Err(SignatureAdmissionFailureDetail::new(
                "signerAuthority",
                format!("signing intent '{signing_intent}' requires signerAuthority"),
            ));
        }
        return Ok(());
    };

    let class = authority
        .get("class")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            SignatureAdmissionFailureDetail::new(
                "signerAuthority.class",
                "signerAuthority.class missing".to_string(),
            )
        })?;

    if let Some(allowed) = allowed_classes
        && !allowed.contains(&class)
    {
        return Err(SignatureAdmissionFailureDetail::new(
            "signerAuthority.class",
            format!(
                "signerAuthority.class '{class}' does not satisfy signing intent '{signing_intent}'"
            ),
        ));
    }

    if is_non_self_class(class) {
        validate_evidence_binding(authority)?;
        if class_requires_authority_source(class) {
            validate_authority_source(authority)?;
        }
        if class_requires_principal(class) {
            validate_principal(authority, signer_id)?;
        }
    }

    validate_validity_window(authority, signed_at)?;

    Ok(())
}

fn validate_authority_source(
    authority: &serde_json::Value,
) -> Result<(), SignatureAdmissionFailureDetail> {
    let value = authority
        .get("authoritySource")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if value.is_none() {
        return Err(SignatureAdmissionFailureDetail::new(
            "signerAuthority.authoritySource",
            "signerAuthority.authoritySource is required for this class".to_string(),
        ));
    }
    Ok(())
}

fn validate_principal(
    authority: &serde_json::Value,
    signer_id: Option<&str>,
) -> Result<(), SignatureAdmissionFailureDetail> {
    let principal = authority
        .get("principal")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SignatureAdmissionFailureDetail::new(
                "signerAuthority.principal",
                "signerAuthority.principal is required for this class".to_string(),
            )
        })?;
    if let Some(signer) = signer_id
        && signer == principal
    {
        return Err(SignatureAdmissionFailureDetail::new(
            "signerAuthority.principal",
            format!(
                "signerAuthority.principal '{principal}' must not equal signerId for a delegating class"
            ),
        ));
    }
    Ok(())
}

fn validate_evidence_binding(
    authority: &serde_json::Value,
) -> Result<(), SignatureAdmissionFailureDetail> {
    let binding = authority.get("evidenceBinding").ok_or_else(|| {
        SignatureAdmissionFailureDetail::new(
            "signerAuthority.evidenceBinding",
            "signerAuthority.evidenceBinding is required for non-self classes".to_string(),
        )
    })?;
    let hash = binding
        .get("evidenceHash")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SignatureAdmissionFailureDetail::new(
                "signerAuthority.evidenceBinding.evidenceHash",
                "signerAuthority.evidenceBinding.evidenceHash is required".to_string(),
            )
        })?;
    let hex_ok = !hash.is_empty()
        && hash.len() >= 64
        && hash.len() <= 128
        && hash.chars().all(|character| character.is_ascii_hexdigit());
    if !hex_ok {
        return Err(SignatureAdmissionFailureDetail::new(
            "signerAuthority.evidenceBinding.evidenceHash",
            format!(
                "signerAuthority.evidenceBinding.evidenceHash '{hash}' is not a valid hex digest"
            ),
        ));
    }
    let algorithm = binding
        .get("evidenceHashAlgorithm")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SignatureAdmissionFailureDetail::new(
                "signerAuthority.evidenceBinding.evidenceHashAlgorithm",
                "signerAuthority.evidenceBinding.evidenceHashAlgorithm is required".to_string(),
            )
        })?;
    if !evidence_hash_algorithm_permitted(algorithm) {
        return Err(SignatureAdmissionFailureDetail::new(
            "signerAuthority.evidenceBinding.evidenceHashAlgorithm",
            format!(
                "signerAuthority.evidenceBinding.evidenceHashAlgorithm '{algorithm}' is not permitted by §2.7"
            ),
        ));
    }
    Ok(())
}

fn validate_validity_window(
    authority: &serde_json::Value,
    signed_at: Option<&str>,
) -> Result<(), SignatureAdmissionFailureDetail> {
    let valid_from = authority
        .get("validFrom")
        .and_then(serde_json::Value::as_str);
    let valid_until = authority
        .get("validUntil")
        .and_then(serde_json::Value::as_str);
    if valid_from.is_none() && valid_until.is_none() {
        return Ok(());
    }
    let signed_at = signed_at.ok_or_else(|| {
        SignatureAdmissionFailureDetail::new(
            "signedAt",
            "signerAuthority validity window present but signedAt is unavailable".to_string(),
        )
    })?;
    let signed = parse_rfc3339_for_admission(signed_at, "signedAt")?;
    if let Some(from_str) = valid_from {
        let from = parse_rfc3339_for_admission(from_str, "signerAuthority.validFrom")?;
        if signed < from {
            return Err(SignatureAdmissionFailureDetail::new(
                "signerAuthority.validFrom",
                format!("signedAt '{signed_at}' precedes signerAuthority.validFrom '{from_str}'"),
            ));
        }
    }
    if let Some(until_str) = valid_until {
        let until = parse_rfc3339_for_admission(until_str, "signerAuthority.validUntil")?;
        if signed > until {
            return Err(SignatureAdmissionFailureDetail::new(
                "signerAuthority.validUntil",
                format!("signedAt '{signed_at}' follows signerAuthority.validUntil '{until_str}'"),
            ));
        }
    }
    Ok(())
}

fn parse_rfc3339_for_admission(
    value: &str,
    field: &'static str,
) -> Result<chrono::DateTime<chrono::FixedOffset>, SignatureAdmissionFailureDetail> {
    chrono::DateTime::parse_from_rfc3339(value).map_err(|error| {
        SignatureAdmissionFailureDetail::new(
            field,
            format!("{field} '{value}' is not RFC 3339: {error}"),
        )
    })
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
        return value_at_path(&record.process.case_state, rest);
    }
    response_path(response, path).or_else(|| value_at_path(&record.process.case_state, path))
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

fn completed_signature_steps(instance: &WorkflowProcess) -> HashSet<String> {
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
    Ok(matches!(result.value, Value::Boolean(true)))
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

/// Pure signed-at consistency check for unit testing without a `RuntimeRecord`.
///
/// Equivalent semantics: empty / absent consent value = admission proceeds;
/// non-empty divergent value = fail closed.
#[cfg(test)]
fn ensure_signed_at_consistency_pure(
    evidence_signed_at: &str,
    consent_signed_at: Option<&str>,
    consent_accepted_at_path: &str,
) -> Result<(), RuntimeError> {
    let Some(consent_signed_at) = consent_signed_at else {
        return Ok(());
    };
    if consent_signed_at.is_empty() {
        return Ok(());
    }
    if consent_signed_at != evidence_signed_at {
        return Err(RuntimeError::Signature(format!(
            "evidence signed_at '{evidence_signed_at}' diverges from response consent signedAt '{consent_signed_at}' at '{consent_accepted_at_path}'"
        )));
    }
    Ok(())
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

#[cfg(test)]
mod signer_authority_tests {
    use super::*;
    use serde_json::json;

    const VALID_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn signed_at() -> &'static str {
        "2026-04-22T14:30:00Z"
    }

    #[test]
    fn rejects_non_self_class_without_evidence_binding() {
        let authority = json!({
            "class": "notary-commissioned",
            "authoritySource": "urn:agency.gov:notary-commissions:tx:12345",
            "principal": "urn:agency.gov:applicants:0001"
        });
        let result = validate_signer_authority(
            "urn:wos:signing-intent:notarial-attestation",
            Some(&authority),
            Some(signed_at()),
            Some("notary-1"),
        );
        match result {
            Err(RuntimeError::Signature(message)) => {
                assert!(
                    message.contains("evidenceBinding"),
                    "expected evidenceBinding error, got: {message}"
                );
            }
            other => panic!("expected Signature error about evidenceBinding, got {other:?}"),
        }
    }

    #[test]
    fn rejects_signed_at_outside_validity_window() {
        let authority = json!({
            "class": "notary-commissioned",
            "authoritySource": "urn:agency.gov:notary-commissions:tx:12345",
            "evidenceBinding": {
                "evidenceHash": VALID_HASH,
                "evidenceHashAlgorithm": "sha-256"
            },
            "validFrom": "2026-01-01T00:00:00Z",
            "validUntil": "2026-03-01T00:00:00Z"
        });
        let result = validate_signer_authority(
            "urn:wos:signing-intent:notarial-attestation",
            Some(&authority),
            Some("2026-05-08T10:00:00Z"),
            Some("notary-1"),
        );
        match result {
            Err(RuntimeError::Signature(message)) => {
                assert!(
                    message.contains("validUntil"),
                    "expected validUntil error, got: {message}"
                );
            }
            other => panic!("expected Signature validity-window error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_evidence_hash() {
        let authority = json!({
            "class": "notary-commissioned",
            "authoritySource": "urn:agency.gov:notary-commissions:tx:12345",
            "evidenceBinding": {
                "evidenceHash": "not-a-real-hash",
                "evidenceHashAlgorithm": "sha-256"
            }
        });
        let result = validate_signer_authority(
            "urn:wos:signing-intent:notarial-attestation",
            Some(&authority),
            Some(signed_at()),
            Some("notary-1"),
        );
        match result {
            Err(RuntimeError::Signature(message)) => {
                assert!(
                    message.contains("evidenceHash"),
                    "expected evidenceHash error, got: {message}"
                );
            }
            other => panic!("expected Signature evidence-hash error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_unpermitted_evidence_hash_algorithm() {
        let authority = json!({
            "class": "notary-commissioned",
            "authoritySource": "urn:agency.gov:notary-commissions:tx:12345",
            "evidenceBinding": {
                "evidenceHash": VALID_HASH,
                "evidenceHashAlgorithm": "md5"
            }
        });
        let result = validate_signer_authority(
            "urn:wos:signing-intent:notarial-attestation",
            Some(&authority),
            Some(signed_at()),
            Some("notary-1"),
        );
        match result {
            Err(RuntimeError::Signature(message)) => {
                assert!(
                    message.contains("evidenceHashAlgorithm"),
                    "expected algorithm error, got: {message}"
                );
            }
            other => panic!("expected Signature algorithm error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_delegating_class_with_self_principal() {
        let authority = json!({
            "class": "as-attorney-in-fact",
            "principal": "agent-007",
            "authoritySource": "urn:agency.gov:poa:42",
            "evidenceBinding": {
                "evidenceHash": VALID_HASH,
                "evidenceHashAlgorithm": "sha-256"
            }
        });
        let result = validate_signer_authority(
            "urn:wos:signing-intent:agent-as-attorney-in-fact",
            Some(&authority),
            Some(signed_at()),
            Some("agent-007"),
        );
        match result {
            Err(RuntimeError::Signature(message)) => {
                assert!(
                    message.contains("must not equal signerId"),
                    "expected self-delegation error, got: {message}"
                );
            }
            other => panic!("expected Signature self-delegation error, got {other:?}"),
        }
    }

    #[test]
    fn admits_self_class_without_authority_source() {
        let authority = json!({
            "class": "self"
        });
        let result = validate_signer_authority(
            "urn:wos:signing-intent:applicant-signature",
            Some(&authority),
            Some(signed_at()),
            Some("applicant-1"),
        );
        assert!(
            result.is_ok(),
            "self-class without authoritySource must admit, got {result:?}"
        );
    }

    #[test]
    fn admits_full_notary_claim() {
        let authority = json!({
            "class": "notary-commissioned",
            "authoritySource": "urn:agency.gov:notary-commissions:tx:12345",
            "principal": "urn:agency.gov:applicants:0001",
            "evidenceBinding": {
                "evidenceHash": VALID_HASH,
                "evidenceHashAlgorithm": "sha-256",
                "evidenceLocation": "urn:agency.gov:notary-commissions:tx:12345:document"
            },
            "validFrom": "2026-01-01T00:00:00Z",
            "validUntil": "2027-01-01T00:00:00Z"
        });
        let result = validate_signer_authority(
            "urn:wos:signing-intent:notarial-attestation",
            Some(&authority),
            Some(signed_at()),
            Some("notary-1"),
        );
        assert!(
            result.is_ok(),
            "complete notarial claim must admit, got {result:?}"
        );
    }

    #[test]
    fn rejects_missing_signer_authority_when_intent_requires_one() {
        let result = validate_signer_authority(
            "urn:wos:signing-intent:notarial-attestation",
            None,
            Some(signed_at()),
            Some("notary-1"),
        );
        match result {
            Err(RuntimeError::Signature(message)) => {
                assert!(
                    message.contains("requires signerAuthority"),
                    "expected requires-signerAuthority error, got: {message}"
                );
            }
            other => panic!("expected Signature missing-authority error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_class_that_does_not_satisfy_floor() {
        let authority = json!({
            "class": "self"
        });
        let result = validate_signer_authority(
            "urn:wos:signing-intent:notarial-attestation",
            Some(&authority),
            Some(signed_at()),
            Some("applicant-1"),
        );
        match result {
            Err(RuntimeError::Signature(message)) => {
                assert!(
                    message.contains("does not satisfy"),
                    "expected floor-mismatch error, got: {message}"
                );
            }
            other => panic!("expected Signature floor-mismatch error, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod deployment_local_intent_tests {
    //! Phase K1: deployment-local signing-intent admission. Bridge until the
    //! WOS Posture Declaration object lands (PLN-0384). Schema enforces that
    //! `deploymentLocalSigningIntents` items do NOT shadow the reserved
    //! `urn:wos:signing-intent:*` namespace; runtime admits an intent that is
    //! either registered or in the workflow's allowlist, fails closed otherwise.

    use super::*;
    use jsonschema::Validator;

    fn allowlist(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    fn signature_schema() -> Validator {
        let raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../schemas/wos-workflow.schema.json"
        ));
        let root: serde_json::Value = serde_json::from_str(raw).expect("workflow schema parses");
        // Extract the Signature definition as a standalone schema (drop $defs
        // refs are not used inside Signature for the field under test).
        let sig = root
            .pointer("/$defs/Signature")
            .cloned()
            .expect("Signature definition exists in workflow schema");
        Validator::new(&sig).expect("Signature schema compiles")
    }

    #[test]
    fn unregistered_intent_without_allowlist_is_rejected() {
        let intents = allowlist(&[]);
        let result = ensure_signing_intent_admitted("urn:acme:custom:intent", &intents);
        match result {
            Err(RuntimeError::Signature(msg)) => {
                assert!(
                    msg.contains("deploymentLocalSigningIntents"),
                    "expected fail-closed message about allowlist, got: {msg}"
                );
            }
            other => panic!("expected Signature rejection, got {other:?}"),
        }
    }

    #[test]
    fn unregistered_intent_with_allowlist_is_admitted() {
        let intents = allowlist(&["urn:acme:custom:intent"]);
        let result = ensure_signing_intent_admitted("urn:acme:custom:intent", &intents);
        assert!(
            result.is_ok(),
            "allowlisted deployment-local intent must admit, got {result:?}"
        );
    }

    #[test]
    fn registered_intent_admitted_regardless_of_allowlist() {
        let intents = allowlist(&[]);
        let result =
            ensure_signing_intent_admitted("urn:wos:signing-intent:applicant-signature", &intents);
        assert!(
            result.is_ok(),
            "registered WOS intent must admit even with empty allowlist, got {result:?}"
        );
    }

    #[test]
    fn allowlist_cannot_shadow_wos_namespace() {
        let validator = signature_schema();
        // Minimal Signature instance: only the field under test, plus the
        // required structural skeleton needed for jsonschema to even reach
        // the property. We intentionally violate `required` (roles/documents/
        // signingFlow/evidence) — the schema MUST reject regardless because
        // the array item pattern check fires before the required check, but
        // even if it didn't, the test asserts that the *combined* result is
        // not-valid, which is what schema-level rejection means.
        let instance = serde_json::json!({
            "deploymentLocalSigningIntents": ["urn:wos:signing-intent:fake"]
        });
        let report = validator.validate(&instance);
        assert!(
            report.is_err(),
            "schema must reject urn:wos:signing-intent:* in deploymentLocalSigningIntents"
        );
        // Sanity: a deployment-scoped URI passes the per-item pattern guard
        // (the missing required keys still fail the overall instance, but the
        // failure for THIS item shape is gone).
        let scoped_instance = serde_json::json!({
            "deploymentLocalSigningIntents": ["urn:acme:signing-intent:supervisor-approval"]
        });
        // Both instances fail overall (required keys missing); we assert the
        // first instance has at least one error mentioning the urn:wos:
        // pattern, distinguishing schema-level shadow rejection from the
        // structural required-fields failure that affects both.
        let collect_errors = |inst: &serde_json::Value| -> Vec<String> {
            validator.iter_errors(inst).map(|e| e.to_string()).collect()
        };
        let bad_errors = collect_errors(&instance);
        let scoped_errors = collect_errors(&scoped_instance);
        let mentions_pattern = |errs: &[String]| {
            errs.iter()
                .any(|e| e.contains("urn:wos:signing-intent:") || e.contains("\"not\""))
        };
        assert!(
            mentions_pattern(&bad_errors),
            "expected a urn:wos: shadow rejection error, got: {bad_errors:?}"
        );
        assert!(
            !mentions_pattern(&scoped_errors),
            "deployment-scoped URI should not trigger the urn:wos: shadow guard, got: {scoped_errors:?}"
        );
    }
}

#[cfg(test)]
mod signed_at_divergence_tests {
    use super::*;

    const EVIDENCE_SIGNED_AT: &str = "2026-05-08T11:00:00Z";
    const PATH: &str = "response.signature.acceptedAt";

    #[test]
    fn admission_fails_when_evidence_signed_at_diverges_from_response_consent() {
        let result = ensure_signed_at_consistency_pure(
            EVIDENCE_SIGNED_AT,
            Some("2026-05-08T10:00:00Z"),
            PATH,
        );
        match result {
            Err(RuntimeError::Signature(msg)) => {
                assert!(
                    msg.contains("signed_at") || msg.contains("signedAt"),
                    "expected divergence message, got: {msg}"
                );
                assert!(msg.contains(EVIDENCE_SIGNED_AT), "got: {msg}");
                assert!(msg.contains("2026-05-08T10:00:00Z"), "got: {msg}");
            }
            other => panic!("expected divergence rejection, got {other:?}"),
        }
    }

    #[test]
    fn admission_succeeds_when_evidence_and_response_signed_at_agree() {
        let result =
            ensure_signed_at_consistency_pure(EVIDENCE_SIGNED_AT, Some(EVIDENCE_SIGNED_AT), PATH);
        assert!(
            result.is_ok(),
            "agreeing timestamps must admit, got {result:?}"
        );
    }

    #[test]
    fn admission_succeeds_when_response_lacks_consent_signed_at() {
        let result = ensure_signed_at_consistency_pure(EVIDENCE_SIGNED_AT, None, PATH);
        assert!(
            result.is_ok(),
            "absent consent-path signedAt must admit, got {result:?}"
        );
    }

    #[test]
    fn admission_succeeds_when_response_consent_signed_at_is_empty_string() {
        let result = ensure_signed_at_consistency_pure(EVIDENCE_SIGNED_AT, Some(""), PATH);
        assert!(
            result.is_ok(),
            "empty consent-path signedAt is treated as absent, got {result:?}"
        );
    }
}

#[cfg(test)]
mod posture_declaration_tests {
    use super::*;

    fn minimal_posture_json() -> &'static str {
        r#"{
            "$postureDeclaration": "1.0",
            "url": "https://example.gov/posture/signature-v1.json",
            "version": "1.0.0",
            "signaturePolicy": {
                "allowedMethods": ["urn:formspec:sig-method:ed25519-cose-sign1@1"],
                "minimumPrimitiveVerification": "verified",
                "receiptSigningRequired": false
            }
        }"#
    }

    #[test]
    fn parse_minimal_posture_declaration() {
        let decl: PostureDeclaration =
            serde_json::from_str(minimal_posture_json()).expect("must parse");
        assert_eq!(decl.version_marker, "1.0");
        assert_eq!(decl.version, "1.0.0");
        assert_eq!(
            decl.signature_policy.allowed_methods,
            vec!["urn:formspec:sig-method:ed25519-cose-sign1@1".to_string()]
        );
        assert_eq!(
            decl.signature_policy.minimum_primitive_verification,
            "verified"
        );
        assert!(!decl.signature_policy.receipt_signing_required);
    }

    #[test]
    fn parse_posture_declaration_with_optional_fields() {
        let json = r#"{
            "$postureDeclaration": "1.0",
            "url": "https://example.gov/posture/signature-v1.json",
            "version": "1.0.0",
            "signaturePolicy": {
                "allowedMethods": [
                    "urn:formspec:sig-method:ed25519-cose-sign1@1",
                    "urn:formspec:sig-method:ecdsa-p256-cose-sign1@1"
                ],
                "minimumPrimitiveVerification": "deferredPendingHelper",
                "receiptSigningRequired": true,
                "allowedSigningIntents": ["urn:wos:signing-intent:formal-attestation@1"]
            },
            "jurisdictionalPosture": { "framework": "esign" },
            "custodyPosture": { "requiresTrellisExport": true }
        }"#;
        let decl: PostureDeclaration = serde_json::from_str(json).expect("must parse");
        assert_eq!(decl.signature_policy.allowed_methods.len(), 2);
        assert_eq!(
            decl.signature_policy.minimum_primitive_verification,
            "deferredPendingHelper"
        );
        assert!(decl.signature_policy.receipt_signing_required);
        assert_eq!(decl.signature_policy.allowed_signing_intents.len(), 1);
        assert!(decl.jurisdictional_posture.is_some());
        assert!(decl.custody_posture.is_some());
    }

    #[test]
    fn reject_posture_declaration_missing_version_marker() {
        let json = r#"{
            "url": "https://example.gov/posture/signature-v1.json",
            "version": "1.0.0",
            "signaturePolicy": {
                "allowedMethods": ["urn:formspec:sig-method:ed25519-cose-sign1@1"],
                "minimumPrimitiveVerification": "verified",
                "receiptSigningRequired": false
            }
        }"#;
        assert!(
            serde_json::from_str::<PostureDeclaration>(json).is_err(),
            "must reject missing $postureDeclaration"
        );
    }

    #[test]
    fn reject_posture_declaration_missing_signature_policy() {
        let json = r#"{
            "$postureDeclaration": "1.0",
            "url": "https://example.gov/posture/signature-v1.json",
            "version": "1.0.0"
        }"#;
        assert!(
            serde_json::from_str::<PostureDeclaration>(json).is_err(),
            "must reject missing signaturePolicy"
        );
    }

    #[test]
    fn posture_declaration_roundtrip() {
        let decl: PostureDeclaration =
            serde_json::from_str(minimal_posture_json()).expect("must parse");
        let serialized = serde_json::to_string(&decl).expect("must serialize");
        let deserialized: PostureDeclaration =
            serde_json::from_str(&serialized).expect("must deserialize");
        assert_eq!(decl.url, deserialized.url);
        assert_eq!(decl.version, deserialized.version);
        assert_eq!(
            decl.signature_policy.allowed_methods,
            deserialized.signature_policy.allowed_methods
        );
    }

    #[test]
    fn posture_uri_policy_allows_https_and_loopback_http() {
        assert!(posture_uri_allowed(
            "https://example.gov/posture/signature-v1.json"
        ));
        assert!(posture_uri_allowed(
            "http://127.0.0.1:8080/posture/signature-v1.json"
        ));
        assert!(posture_uri_allowed(
            "http://localhost/posture/signature-v1.json"
        ));
        assert!(posture_uri_allowed(
            "http://[::1]:8080/posture/signature-v1.json"
        ));
    }

    #[test]
    fn posture_uri_policy_rejects_non_https_non_loopback() {
        assert!(!posture_uri_allowed(
            "http://example.gov/posture/signature-v1.json"
        ));
        assert!(!posture_uri_allowed(
            "file:///tmp/posture/signature-v1.json"
        ));
        assert!(!posture_uri_allowed(
            "https://user@example.gov/posture/signature-v1.json"
        ));
    }
}

#[cfg(test)]
mod k2_field_binding_tests {
    use super::*;
    use crate::binding::SignatureEvidence;
    use crate::binding::SignaturePrimitiveStatus;
    use wos_core::ProvenanceRecord;

    const DOC_HASH: &str =
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const PRESENTATION_HASH: &str =
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    /// Exercises the same K-2 field wiring as `signature_affirmation_for_submission`.
    #[test]
    fn given_distinct_k2_evidence_when_runtime_k2_helper_used_then_effective_fields_apply() {
        let evidence = SignatureEvidence {
            source_system: "test-binding".to_string(),
            source_signature_id: "source-sig-001".to_string(),
            signing_act_id: Some("signing-act-777".to_string()),
            presentation_hash: Some(PRESENTATION_HASH.to_string()),
            source_response_ref: None,
            document_id: "application".to_string(),
            signer_id: None,
            signing_intent: "urn:wos:signing-intent:applicant-signature".to_string(),
            signature_method: None,
            signed_payload_digest: DOC_HASH.to_string(),
            signed_payload_digest_algorithm: "sha-256".to_string(),
            signed_at: "2026-05-15T12:00:00Z".to_string(),
            document_hash: DOC_HASH.to_string(),
            document_hash_algorithm: "sha-256".to_string(),
            signature_provider: None,
            ceremony_id: None,
            identity_binding: None,
            signer_authority: None,
            primitive_verification: SignaturePrimitiveStatus::Verified,
            verification_receipt: None,
            admission_failure: None,
        };
        let (signing_act_id, presentation_hash) =
            super::signature_affirmation_k2_fields(&evidence);
        assert_eq!(signing_act_id, "signing-act-777");
        assert_eq!(presentation_hash, PRESENTATION_HASH);
    }

    /// Given binding evidence with distinct signing-act and presentation digests,
    /// when a signature affirmation record is minted, then all four K-2 fields
    /// remain distinct in the emitted provenance payload.
    #[test]
    fn given_distinct_k2_evidence_when_affirmation_minted_then_fields_stay_distinct() {
        let evidence = SignatureEvidence {
            source_system: "test-binding".to_string(),
            source_signature_id: "source-sig-001".to_string(),
            signing_act_id: Some("signing-act-777".to_string()),
            presentation_hash: Some(PRESENTATION_HASH.to_string()),
            source_response_ref: Some("urn:test:response:1".to_string()),
            document_id: "application".to_string(),
            signer_id: Some("signer-1".to_string()),
            signing_intent: "urn:wos:signing-intent:applicant-signature".to_string(),
            signature_method: None,
            signed_payload_digest: DOC_HASH.to_string(),
            signed_payload_digest_algorithm: "sha-256".to_string(),
            signed_at: "2026-05-15T12:00:00Z".to_string(),
            document_hash: DOC_HASH.to_string(),
            document_hash_algorithm: "sha-256".to_string(),
            signature_provider: None,
            ceremony_id: None,
            identity_binding: None,
            signer_authority: None,
            primitive_verification: SignaturePrimitiveStatus::Verified,
            verification_receipt: None,
            admission_failure: None,
        };

        let (signing_act_id, presentation_hash) =
            super::signature_affirmation_k2_fields(&evidence);
        let record = ProvenanceRecord::signature_affirmation(SignatureAffirmationInput {
            signer_id: "signer-1",
            role_id: "role-1",
            role: "applicant",
            document_id: "application",
            signing_act_id,
            document_ref: serde_json::json!({ "documentId": "application", "locale": "und" }),
            document_hash: &evidence.document_hash,
            presentation_hash,
            document_hash_algorithm: &evidence.document_hash_algorithm,
            source_signature_system: &evidence.source_system,
            source_signature_id: &evidence.source_signature_id,
            signed_payload_digest: &evidence.signed_payload_digest,
            signed_payload_digest_algorithm: &evidence.signed_payload_digest_algorithm,
            signing_intent: &evidence.signing_intent,
            signed_at: &evidence.signed_at,
            identity_binding: serde_json::json!({}),
            consent_reference: serde_json::json!({}),
            signature_provider: "test",
            ceremony_id: "ceremony-1",
            profile_ref: None,
            profile_key: None,
            source_response_ref: "urn:test:response:1",
            signer_authority: None,
            custody_hook_eligible: true,
            primitive_verification: serde_json::json!({ "status": "verified" }),
            verification_receipt: None,
            witnessed_signature_ref: None,
        });

        let data = record
            .data
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .expect("affirmation data map");
        assert_eq!(
            data.get("signingActId").and_then(|v| v.as_str()),
            Some("signing-act-777")
        );
        assert_eq!(
            data.get("sourceSignatureId").and_then(|v| v.as_str()),
            Some("source-sig-001")
        );
        assert_eq!(
            data.get("documentHash").and_then(|v| v.as_str()),
            Some(DOC_HASH)
        );
        assert_eq!(
            data.get("presentationHash").and_then(|v| v.as_str()),
            Some(PRESENTATION_HASH)
        );
    }
}
