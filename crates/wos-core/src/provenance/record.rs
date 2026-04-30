// Rust guideline compliant 2026-02-21

use serde::{Deserialize, Serialize};

use crate::typeid;

use super::kind::ProvenanceKind;
use super::snapshot::CaseFileSnapshot;

/// Configuration-warning provenance input (cross-cutting; covers AI
/// `drift-monitor.policyRef`, governance `continuationPolicyRef`, and
/// notification-template key/render failures).
///
/// Carrier for the four spec MUSTs at `drift-monitor.md:77`,
/// `workflow-governance.md:154`, and `notification-template.md:199,222`.
/// `subject` is the discriminator literal naming the failure site; the
/// reserved set is `drift-monitor.policyRef`,
/// `governance.continuationPolicyRef`, `notification-template.key`,
/// `notification-template.render`. Vendor extensions use an `x-` prefix.
pub struct ConfigurationWarningInput<'a> {
    /// Failure-site discriminator (see type docstring for reserved set).
    pub subject: &'a str,
    /// The configuration reference that failed to resolve, when the
    /// failure mode is "ref unresolvable" (drift-monitor, governance,
    /// notification-template key). Omit for render-failure subjects
    /// where the failing identity is the template key carried in
    /// `context.templateKey`.
    pub unresolved_ref: Option<&'a str>,
    /// Additional context payload merged into `data` — failure reason
    /// string, the workflow URI, the case-file fields consulted at
    /// fallback time, etc. Keys in `context` that collide with the
    /// constructor's required fields (`subject`, `unresolvedRef`) are
    /// silently dropped: the typed input is the source of truth, and
    /// `context` (which may originate from caller-supplied scratch) MUST
    /// NOT overwrite the schema-shaping discriminators.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Capability-invocation provenance input (AI Integration §3.3.1).
///
/// Holds the precondition-evaluation outcome for an agent capability before
/// it is serialized into a `CapabilityInvocation` provenance record. The
/// constructor enforces the Kernel §8.2.2 invariant that a blocked
/// invocation carries the reserved outcome literal
/// `"preconditionNotSatisfied"`.
pub struct CapabilityInvocationInput<'a> {
    /// Capability identifier from the agent declaration (AI §3.3).
    pub capability_id: &'a str,
    /// Stable identifier for the agent actor that owns the capability.
    pub agent_id: &'a str,
    /// `true` when a precondition evaluated to non-`true` (false or
    /// non-boolean) and the processor skipped invocation; `false` when all
    /// preconditions passed and the capability proceeds.
    pub invocation_blocked: bool,
    /// Optional context payload merged into `data` — failed expression
    /// source, evaluation snapshot, fallback-chain reference, resolved
    /// precondition value, etc. Keys in `context` that collide with
    /// `capabilityId` / `invocationBlocked` are silently dropped: the
    /// agent declaration is the source of truth for capability identity,
    /// and `context` (which may originate from FEL-evaluator output or
    /// other untrusted scratch) MUST NOT be able to overwrite the
    /// schema-required discriminators that drive the
    /// `CapabilityInvocationRecord` if/then guard.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Resolution payload for [`ProvenanceRecord::clock_resolved`] (ADR 0067 §3).
///
/// `Paused` carries the pause-event hash at the type level so a paused
/// resolution cannot be constructed without it (Q11 maximalist; matches JSON
/// Schema `if/then` on `ClockResolvedRecord`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClockResolvedResolution<'a> {
    /// Terminal satisfied outcome; optional hash when a concrete resolving event exists.
    Satisfied {
        resolving_event_hash: Option<&'a str>,
    },
    /// Deadline elapsed without a resolving event (synthetic elapsed).
    Elapsed {
        resolving_event_hash: Option<&'a str>,
    },
    /// Cancelled (e.g. on supersession); optional hash of the cancelling event.
    Cancelled {
        resolving_event_hash: Option<&'a str>,
    },
    /// Paused — the pause event itself is the resolving event (hash required).
    Paused {
        resolving_event_hash: &'a str,
    },
}

/// Closed failure-kind discriminant for
/// [`ProvenanceRecord::commit_attempt_failure`] (ADR 0070 §2). Typed at
/// the Rust seam so invalid failure kinds cannot reach the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommitFailureKind {
    NetworkTimeout,
    SubstrateDown,
    HashConflict,
    Other,
}

impl CommitFailureKind {
    fn as_camel_str(self) -> &'static str {
        match self {
            Self::NetworkTimeout => "networkTimeout",
            Self::SubstrateDown => "substrateDown",
            Self::HashConflict => "hashConflict",
            Self::Other => "other",
        }
    }
}

/// Amendment-authorization provenance input (ADR 0066 §2).
///
/// Authorizes a substantive change to a prior determination. Pairs with
/// [`DeterminationAmendedInput`] which carries the new value.
pub struct AmendmentAuthorizedInput<'a> {
    /// Hash of the event being amended (the amendment target).
    pub amendment_target_event_hash: &'a str,
    /// Hash of the prior determination value being superseded.
    pub prior_determination_hash: &'a str,
    /// Free-text rationale captured from the authorizing actor.
    pub reason: &'a str,
    /// Stable identifier for the actor authorizing the amendment.
    pub authorizing_actor_id: &'a str,
    /// Discriminated union: `{"kind": "uri", "value": "..."}` or
    /// `{"kind": "actorPolicyRef", "value": "..."}`.
    pub authority_basis: serde_json::Value,
    /// Optional context payload merged into `data`. Constructor's
    /// required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Authorization-attestation provenance input (ADR 0066 §5).
pub struct AuthorizationAttestationInput<'a> {
    /// Stable identifier for the attesting (authorizing) actor.
    pub authorizing_actor_id: &'a str,
    /// Discriminated-union policy basis (see
    /// [`AmendmentAuthorizedInput::authority_basis`]).
    pub authority_basis: serde_json::Value,
    /// Closed-namespace policy predicate. Reserved literals include
    /// `"amendment-authority"`, `"rescission-authority"`,
    /// `"reinstatement-authority"`.
    pub policy_predicate: &'a str,
    /// Optional assurance level (e.g. `"high"`, `"standard"`).
    pub assurance_level: Option<&'a str>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Authorization-rejected provenance input (ADR 0070 §4).
pub struct AuthorizationRejectedInput<'a> {
    /// Identifier of the actor whose attempt was rejected.
    pub attempted_actor_id: &'a str,
    /// Action verb that was attempted, e.g. `"transition:approve"`,
    /// `"submit:taskResponse"`.
    pub attempted_action: &'a str,
    /// Identifier of the resource the actor tried to act upon.
    pub target_resource_id: &'a str,
    /// Free-text rationale (typically copied from the policy decision).
    pub rejection_reason: &'a str,
    /// Optional reference to the upstream policy decision record.
    pub policy_decision_ref: Option<&'a str>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Clock-resolved provenance input (ADR 0067 §3).
pub struct ClockResolvedInput<'a> {
    /// Identifier of the clock that resolved.
    pub clock_id: &'a str,
    /// Hash of the originating `ClockStarted` event.
    pub origin_clock_hash: &'a str,
    /// Resolution outcome; see [`ClockResolvedResolution`].
    pub resolution: ClockResolvedResolution<'a>,
    /// RFC 3339 timestamp at which resolution occurred.
    pub resolved_at: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Clock-skew-observed provenance input (ADR 0069 §3).
pub struct ClockSkewObservedInput<'a> {
    /// Processor-side authoring timestamp (RFC 3339).
    pub processor_authored_at: &'a str,
    /// Substrate-side creation timestamp (RFC 3339).
    pub substrate_created_at: &'a str,
    /// Signed skew (positive = processor ahead). Stored as i64 because
    /// negative values are valid observations.
    pub skew_milliseconds: i64,
    /// Configured threshold above which skew triggers a record.
    pub threshold_milliseconds: u64,
    /// Hash of the event whose timestamps revealed the skew.
    pub event_hash: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Clock-started provenance input (ADR 0067 §2).
pub struct ClockStartedInput<'a> {
    /// Identifier of the new clock.
    pub clock_id: &'a str,
    /// Open-enum kind label (`"AppealClock"`, `"ProcessingSLA"`,
    /// `"GrantExpiry"`, `"StatuteClock"`, `x-*`).
    pub clock_kind: &'a str,
    /// Hash of the event that started the clock.
    pub origin_event_hash: &'a str,
    /// ISO 8601 duration string.
    pub duration: &'a str,
    /// Computed deadline (RFC 3339).
    pub computed_deadline: &'a str,
    /// Optional reference to a business calendar definition.
    pub calendar_ref: Option<&'a str>,
    /// Optional URI naming the governing statute.
    pub statute_reference: Option<&'a str>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Commit-attempt-failure provenance input (ADR 0070 §2).
pub struct CommitAttemptFailureInput<'a> {
    /// Hash of the event whose commit attempt failed.
    pub target_event_hash: &'a str,
    /// Closed failure-kind discriminant.
    pub failure_kind: CommitFailureKind,
    /// Number of attempts that have occurred so far (1-based).
    pub attempt_count: u32,
    /// Remaining retry budget in milliseconds.
    pub retry_budget_remaining_ms: u64,
    /// Optional adapter-specific error payload.
    pub error_payload: Option<serde_json::Value>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Correction-authorized provenance input (ADR 0066 §1).
///
/// Mode 1 of the closed five-mode supersession taxonomy. Records the
/// authorizing act for a non-determination correction (e.g. typo fix).
pub struct CorrectionAuthorizedInput<'a> {
    /// Hash of the event being corrected.
    pub correction_target_event_hash: &'a str,
    /// JSON-pointer strings naming the corrected fields.
    pub corrected_field_set: Vec<&'a str>,
    /// Free-text rationale.
    pub reason: &'a str,
    /// Identifier of the authorizing actor.
    pub authorizing_actor_id: &'a str,
    /// Discriminated-union policy basis.
    pub authority_basis: serde_json::Value,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Determination-amended provenance input (ADR 0066 §2).
pub struct DeterminationAmendedInput<'a> {
    /// Hash of the prior determination value being amended.
    pub prior_determination_hash: &'a str,
    /// New determination value (any JSON shape per binding).
    pub new_determination_value: serde_json::Value,
    /// Hash of the authorizing `AmendmentAuthorized` record.
    pub amendment_authorization_event_hash: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Determination-rescinded provenance input (ADR 0066 §3).
pub struct DeterminationRescindedInput<'a> {
    /// Hash of the prior determination value being rescinded.
    pub prior_determination_hash: &'a str,
    /// Hash of the authorizing `RescissionAuthorized` record.
    pub rescission_authorization_event_hash: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Identity-attestation provenance input (ADR 0068 §4, Q15).
pub struct IdentityAttestationInput<'a> {
    /// Cross-tenant subject identifier.
    pub subject_global_id: &'a str,
    /// Open-enum assurance level (`"low"` | `"standard"` | `"high"` |
    /// `"very-high"` | vendor extension).
    pub assurance_level: &'a str,
    /// Identifier of the attestation provider (issuer).
    pub attestation_provider: &'a str,
    /// Provider-issued identifier for this attestation event.
    pub provider_attestation_id: &'a str,
    /// RFC 3339 timestamp at which the provider attested.
    pub attested_at: &'a str,
    /// Optional RFC 3339 expiry of the attestation.
    pub valid_until: Option<&'a str>,
    /// Open-list of attested predicates (e.g.
    /// `["legal-name-verified", "age-of-majority"]`).
    pub attested_predicates: Vec<&'a str>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Migration-pin-changed provenance input (ADR 0071 §3).
pub struct MigrationPinChangedInput<'a> {
    /// Prior 4-field pin tree (per maximalist Q33: `formspec.definitionVersion`,
    /// `wos.$wosWorkflowVersion`, `trellis.envelopeVersion`,
    /// `trellis.conformanceClass`).
    pub prior_pin_set: serde_json::Value,
    /// New 4-field pin tree (same shape as `prior_pin_set`).
    pub new_pin_set: serde_json::Value,
    /// Identifier of the actor authorizing the pin change.
    pub authorizing_actor_id: &'a str,
    /// Discriminated-union policy basis.
    pub authority_basis: serde_json::Value,
    /// Free-text rationale.
    pub migration_rationale: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Reinstated provenance input (ADR 0066 §4 — Mode 5, owner directive Q26).
///
/// Re-activates a determination from a non-operative (post-rescission)
/// state. Distinct from amendment: the substantive value is unchanged;
/// only the operative status flips back.
pub struct ReinstatedInput<'a> {
    /// Hash of the prior `DeterminationRescinded` (or
    /// `RescissionAuthorized`) event being undone.
    pub prior_rescission_event_hash: &'a str,
    /// Hash of the authorizing `AuthorizationAttestation` record
    /// (predicate `"reinstatement-authority"`).
    pub reactivation_authorization_event_hash: &'a str,
    /// Free-text rationale.
    pub reason: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Rescission-authorized provenance input (ADR 0066 §3).
///
/// Mode 4 of the closed five-mode supersession taxonomy. The optional
/// `migration_pin_change` carries the maximalist Q32 cross-chain hash
/// linkage for supersession that also changes a version pin.
pub struct RescissionAuthorizedInput<'a> {
    /// Hash of the event whose authorization is being rescinded.
    pub rescission_target_event_hash: &'a str,
    /// Hash of the prior determination value being rescinded.
    pub prior_determination_hash: &'a str,
    /// Free-text rationale.
    pub reason: &'a str,
    /// Identifier of the authorizing actor.
    pub authorizing_actor_id: &'a str,
    /// Discriminated-union policy basis.
    pub authority_basis: serde_json::Value,
    /// Optional cross-chain hash linkage when supersession also changes
    /// a version pin (Q32). Carries
    /// `{newChainPinEventHash, priorPinSet, newPinSet}`.
    pub migration_pin_change: Option<serde_json::Map<String, serde_json::Value>>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Signature affirmation provenance input.
///
/// Holds the required WOS Signature Profile evidence fields before they are
/// serialized into a `SignatureAffirmation` provenance record.
pub struct SignatureAffirmationInput<'a> {
    /// Stable signer identifier from the signature ceremony context.
    pub signer_id: &'a str,
    /// Signature Profile role id.
    pub role_id: &'a str,
    /// Signature Profile role literal.
    pub role: &'a str,
    /// Signature Profile document id.
    pub document_id: &'a str,
    /// Digest of the document bytes the signer affirmed.
    pub document_hash: &'a str,
    /// Digest algorithm used for `document_hash`.
    pub document_hash_algorithm: &'a str,
    /// RFC 3339 timestamp for the signing act.
    pub signed_at: &'a str,
    /// Provider-neutral identity-binding evidence.
    pub identity_binding: serde_json::Value,
    /// Consent text/version and affirmation evidence reference.
    pub consent_reference: serde_json::Value,
    /// Signature provider identifier.
    pub signature_provider: &'a str,
    /// Provider or adapter ceremony identifier.
    pub ceremony_id: &'a str,
    /// URI reference to the Signature Profile, when cross-artifact.
    pub profile_ref: Option<&'a str>,
    /// Package-local Signature Profile key, when resolved in-document.
    pub profile_key: Option<&'a str>,
    /// URI reference to the canonical Formspec response.
    pub formspec_response_ref: &'a str,
    /// Whether the record is eligible for `custodyHook` admission.
    pub custody_hook_eligible: bool,
}

/// A single provenance record.
///
/// Records carry an RFC 3339 / ISO 8601 `timestamp` populated by the runtime
/// (or test harness) at the moment the record is appended to the instance log.
/// Constructors leave the field empty; the runtime stamps any empty timestamp
/// with the active clock before persisting the record (see
/// `wos_runtime::stamp_provenance`). Records produced in unit tests that never
/// reach the runtime may carry an empty `timestamp` — exporters and
/// downstream consumers (PROV-O, XES, OCEL) MUST treat an empty value as
/// "unknown" rather than emitting it verbatim.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceRecord {
    /// TypeID-structured identifier minted at authoring time.
    pub id: String,

    /// Record type.
    pub record_kind: ProvenanceKind,

    /// RFC 3339 / ISO 8601 timestamp set by the runtime when the record is
    /// appended to a log. Empty until stamped.
    #[serde(default)]
    pub timestamp: String,

    /// Actor who triggered the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,

    /// Source state (for transitions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_state: Option<String>,

    /// Target state (for transitions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_state: Option<String>,

    /// Triggering event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,

    /// Additional context data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,

    /// Provenance tier: `"facts"`, `"reasoning"`, `"counterfactual"`, or
    /// `"narrative"` (SP §5.4, §6.5). Defaults to `"facts"` at construction;
    /// populated by the runtime tier classifier before persistence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_layer: Option<String>,

    /// Actor type: `"human"`, `"system"`, or `"agent"` (SP §5.3, §5.5, §6.3).
    /// Populated at construction from the kernel `ActorKind` registry lookup
    /// (or from the AI Integration agent registry for `"agent"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<String>,

    /// Canonical lifecycle state at action time, distinct from `from_state`
    /// (which carries the pre-transition label). Maps to `wos:atLifecycleState`
    /// (PROV-O §5.3) and `wos:lifecycleState` (XES §6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_state: Option<String>,

    /// Version of the governing WOS Kernel Document (SP §5.3, §6.3).
    /// Populated from the workflow definition's `version` field at runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_version: Option<String>,

    /// Input entity references used by this activity (SP §5.3 `prov:used`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,

    /// Output entity references generated by this activity (SP §5.3
    /// `prov:wasGeneratedBy` inverse).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,

    /// Tamper-detection digest for the inputs snapshot (SP §5.3, §6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_digest: Option<String>,

    /// Tamper-detection digest for the outputs snapshot (SP §5.3, §6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_digest: Option<String>,

    /// Trellis `canonical_event_hash` stamped after custody admission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_event_hash: Option<String>,

    /// Semantic tags copied from the firing transition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transition_tags: Vec<String>,

    /// Case-file snapshot used by a determination-tagged transition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_file_snapshot: Option<CaseFileSnapshot>,

    /// Open-enum outcome literal recorded by the processor (Kernel §8.2.2).
    ///
    /// Optional; the kernel `$defs/ProvenanceOutcome` schema validates any
    /// populated value against the reserved-literal set
    /// (`preconditionNotSatisfied`, `convergenceCapReached`) plus an
    /// `x-`-prefixed vendor-extension fallback. The `skip_serializing_if`
    /// keeps existing fixtures byte-identical: records that leave the field
    /// `None` still serialize without an `"outcome"` key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
}

impl ProvenanceRecord {
    /// Mints a new provenance-record identifier.
    #[must_use]
    pub fn mint_id() -> String {
        typeid::mint_provenance_id()
    }

    fn blank(record_kind: ProvenanceKind) -> Self {
        Self {
            id: Self::mint_id(),
            record_kind,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: None,
            data: None,
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
            canonical_event_hash: None,
            transition_tags: Vec::new(),
            case_file_snapshot: None,
            outcome: None,
        }
    }

    /// Create a state transition record.
    pub fn state_transition(from: &str, to: &str, event: &str, actor_id: Option<&str>) -> Self {
        let mut record = Self::blank(ProvenanceKind::StateTransition);
        record.actor_id = actor_id.map(String::from);
        record.from_state = Some(from.to_string());
        record.to_state = Some(to.to_string());
        record.event = Some(event.to_string());
        record
    }

    /// Create a state transition record with transition tags and an optional
    /// determination snapshot.
    pub fn tagged_state_transition(
        from: &str,
        to: &str,
        event: &str,
        actor_id: Option<&str>,
        transition_tags: &[String],
        case_file_snapshot: Option<CaseFileSnapshot>,
    ) -> Self {
        let mut record = Self::state_transition(from, to, event, actor_id);
        record.transition_tags = transition_tags.to_vec();
        record.case_file_snapshot = case_file_snapshot;
        record
    }

    /// Create an unmatched event record (Kernel S4.9).
    pub fn unmatched_event(event: &str, actor_id: Option<&str>) -> Self {
        let mut record = Self::blank(ProvenanceKind::UnmatchedEvent);
        record.actor_id = actor_id.map(String::from);
        record.event = Some(event.to_string());
        record
    }

    /// Create a case state mutation record (Kernel S5.4).
    pub fn case_state_mutation(
        path: &str,
        new_value: &serde_json::Value,
        actor_id: Option<&str>,
        lifecycle_state: &str,
    ) -> Self {
        Self::case_state_mutation_with_source(path, new_value, actor_id, lifecycle_state, None, None)
    }

    pub fn case_state_mutation_with_source(
        path: &str,
        new_value: &serde_json::Value,
        actor_id: Option<&str>,
        lifecycle_state: &str,
        mutation_source: Option<&str>,
        verification_level: Option<&str>,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::CaseStateMutation);
        record.actor_id = actor_id.map(String::from);
        let mut data = serde_json::json!({
            "path": path,
            "newValue": new_value,
            "lifecycleState": lifecycle_state,
            "viaExplicitAction": true,
        });
        if let Some(src) = mutation_source {
            data["mutationSource"] = serde_json::Value::String(src.to_string());
        }
        if let Some(vl) = verification_level {
            data["verificationLevel"] = serde_json::Value::String(vl.to_string());
        }
        record.data = Some(data);
        record
    }

    /// Create a timer created record (Lifecycle Detail S6.7).
    pub fn timer_created(timer_id: &str, duration: &str, fires_event: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::TimerCreated);
        record.data = Some(serde_json::json!({
            "timerId": timer_id,
            "duration": duration,
            "firesEvent": fires_event,
        }));
        record
    }

    /// Create a timer fired record (Lifecycle Detail S6.7).
    pub fn timer_fired(timer_id: &str, fires_event: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::TimerFired);
        record.data = Some(serde_json::json!({
            "timerId": timer_id,
            "firesEvent": fires_event,
        }));
        record
    }

    // ── ForEach iteration builders (Kernel §4.3.1; Sub-PR D-2) ──────────────

    /// One iteration of a `ForEach` state is starting.
    pub fn foreach_iteration_started(
        foreach_state: &str,
        index: u32,
        item: &serde_json::Value,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ForEachIterationStarted);
        record.data = Some(serde_json::json!({
            "foreachState": foreach_state,
            "index": index,
            "item": item,
        }));
        record
    }

    /// One iteration of a `ForEach` state has completed. When iteration
    /// terminated early via `breakCondition`, `break_triggered` is `true`.
    pub fn foreach_iteration_completed(
        foreach_state: &str,
        index: u32,
        break_triggered: bool,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ForEachIterationCompleted);
        let mut data = serde_json::json!({
            "foreachState": foreach_state,
            "index": index,
        });
        if break_triggered {
            data["breakTriggered"] = serde_json::Value::Bool(true);
        }
        record.data = Some(data);
        record
    }

    /// All iterations of a `ForEach` state have completed (or the empty-
    /// collection fast path fired). Emitted exactly once per foreach state
    /// entry, immediately before the foreach state's outgoing transition
    /// fires. `iterations` is the number of iterations actually executed
    /// (0 for empty-collection fast path); `broke` indicates whether the
    /// loop terminated early via `breakCondition`.
    pub fn foreach_completed(foreach_state: &str, iterations: u32, broke: bool) -> Self {
        let mut record = Self::blank(ProvenanceKind::ForEachCompleted);
        record.data = Some(serde_json::json!({
            "foreachState": foreach_state,
            "iterations": iterations,
            "broke": broke,
        }));
        record
    }

    /// Create a timer cancelled record (Lifecycle Detail S6.7).
    pub fn timer_cancelled(timer_id: &str, reason: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::TimerCancelled);
        record.data = Some(serde_json::json!({
            "timerId": timer_id,
            "reason": reason,
        }));
        record
    }

    /// Create a state-entry record.
    pub fn state_entered(state: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::OnEntry);
        record.to_state = Some(state.to_string());
        record.data = Some(serde_json::json!({ "state": state }));
        record
    }

    /// Create an onEntry action record.
    pub fn on_entry(state: &str, action_type: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::OnEntry);
        record.to_state = Some(state.to_string());
        record.data = Some(serde_json::json!({ "actionType": action_type }));
        record
    }

    /// Create an onExit action record.
    pub fn on_exit(state: &str, action_type: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::OnExit);
        record.from_state = Some(state.to_string());
        record.data = Some(serde_json::json!({ "actionType": action_type }));
        record
    }

    /// Create a generic action-executed record.
    pub fn action_executed(state: &str, action_type: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::ActionExecuted);
        record.to_state = Some(state.to_string());
        record.data = Some(serde_json::json!({ "actionType": action_type }));
        record
    }

    /// Create a timer tolerance violation record (LCD S6.6, Runtime S7.2).
    pub fn tolerance_violation(
        timer_id: &str,
        duration_iso: &str,
        max_tolerance_iso: &str,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ToleranceViolation);
        record.data = Some(serde_json::json!({
            "timerId": timer_id,
            "duration": duration_iso,
            "maxTolerance": max_tolerance_iso,
        }));
        record
    }

    /// Create a history-cleared record.
    pub fn history_cleared(state: &str, reason: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::HistoryCleared);
        record.data = Some(serde_json::json!({
            "state": state,
            "reason": reason,
        }));
        record
    }

    /// Create an invalid-duration warning record.
    pub fn invalid_duration(raw_duration: &str, timer_id: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::InvalidDuration);
        record.data = Some(serde_json::json!({
            "rawDuration": raw_duration,
            "timerId": timer_id,
            "note": "unrecognized ISO 8601 duration; deadline set to zero (fires immediately)",
        }));
        record
    }

    /// Create a task lifecycle record emitted by the runtime layer.
    pub fn task_lifecycle(
        record_kind: ProvenanceKind,
        task_id: &str,
        actor_id: Option<&str>,
        data: Option<serde_json::Value>,
    ) -> Self {
        let mut record = Self::blank(record_kind);
        record.actor_id = actor_id.map(String::from);
        record.data = Some(match data {
            Some(extra) => {
                let mut object = serde_json::Map::new();
                object.insert(
                    "taskId".to_string(),
                    serde_json::Value::String(task_id.to_string()),
                );
                object.insert("details".to_string(), extra);
                serde_json::Value::Object(object)
            }
            None => serde_json::json!({ "taskId": task_id }),
        });
        record
    }

    /// Create a configuration-warning record for an unresolvable
    /// configuration reference or a configured operation failure
    /// (`drift-monitor.md:77`, `workflow-governance.md:154`,
    /// `notification-template.md:199,222`).
    ///
    /// `subject` is recorded verbatim; callers supply one of the four
    /// reserved literals or an `x-` vendor extension. `unresolvedRef` is
    /// merged into `data` only when the input carries it; render-failure
    /// records typically omit it and convey the failing template key /
    /// reason via `context`.
    #[must_use]
    pub fn configuration_warning(input: ConfigurationWarningInput<'_>) -> Self {
        let mut data = serde_json::Map::new();
        if let Some(context) = input.context {
            for (k, v) in context {
                if k == "subject" || k == "unresolvedRef" {
                    continue;
                }
                data.insert(k, v);
            }
        }
        data.insert(
            "subject".to_string(),
            serde_json::Value::String(input.subject.to_string()),
        );
        if let Some(unresolved_ref) = input.unresolved_ref {
            data.insert(
                "unresolvedRef".to_string(),
                serde_json::Value::String(unresolved_ref.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::ConfigurationWarning);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a capability-invocation record (AI Integration §3.3.1).
    ///
    /// When `invocation_blocked` is `true`, the record's `outcome` is set to
    /// the reserved kernel literal `"preconditionNotSatisfied"` (Kernel §8.2.2)
    /// so audit tooling can distinguish a declarative gate from an agent
    /// failure. When `false`, the outcome is left unset — the invocation
    /// proceeded normally and downstream records carry the agent outcome.
    #[must_use]
    pub fn capability_invocation(input: CapabilityInvocationInput<'_>) -> Self {
        let mut data = serde_json::Map::new();
        if let Some(context) = input.context {
            for (k, v) in context {
                if k == "capabilityId" || k == "invocationBlocked" {
                    continue;
                }
                data.insert(k, v);
            }
        }
        data.insert(
            "capabilityId".to_string(),
            serde_json::Value::String(input.capability_id.to_string()),
        );
        data.insert(
            "invocationBlocked".to_string(),
            serde_json::Value::Bool(input.invocation_blocked),
        );

        let mut record = Self::blank(ProvenanceKind::CapabilityInvocation);
        record.actor_id = Some(input.agent_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        if input.invocation_blocked {
            record.outcome = Some("preconditionNotSatisfied".to_string());
        }
        record
    }

    /// Create a contract validation record emitted by runtime task flows.
    pub fn contract_validation(
        task_id: &str,
        actor_id: Option<&str>,
        data: serde_json::Value,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ContractValidation);
        record.actor_id = actor_id.map(String::from);
        record.data = Some(serde_json::json!({
            "taskId": task_id,
            "details": data,
        }));
        record
    }

    /// Create a Signature Profile affirmation record.
    #[must_use]
    pub fn signature_affirmation(input: SignatureAffirmationInput<'_>) -> Self {
        let mut data = serde_json::Map::from_iter([
            (
                "signerId".to_string(),
                serde_json::Value::String(input.signer_id.to_string()),
            ),
            (
                "roleId".to_string(),
                serde_json::Value::String(input.role_id.to_string()),
            ),
            (
                "role".to_string(),
                serde_json::Value::String(input.role.to_string()),
            ),
            (
                "documentId".to_string(),
                serde_json::Value::String(input.document_id.to_string()),
            ),
            (
                "documentHash".to_string(),
                serde_json::Value::String(input.document_hash.to_string()),
            ),
            (
                "documentHashAlgorithm".to_string(),
                serde_json::Value::String(input.document_hash_algorithm.to_string()),
            ),
            (
                "signedAt".to_string(),
                serde_json::Value::String(input.signed_at.to_string()),
            ),
            ("identityBinding".to_string(), input.identity_binding),
            ("consentReference".to_string(), input.consent_reference),
            (
                "signatureProvider".to_string(),
                serde_json::Value::String(input.signature_provider.to_string()),
            ),
            (
                "ceremonyId".to_string(),
                serde_json::Value::String(input.ceremony_id.to_string()),
            ),
            (
                "formspecResponseRef".to_string(),
                serde_json::Value::String(input.formspec_response_ref.to_string()),
            ),
            (
                "custodyHookEligible".to_string(),
                serde_json::Value::Bool(input.custody_hook_eligible),
            ),
        ]);

        if let Some(profile_ref) = input.profile_ref {
            data.insert(
                "profileRef".to_string(),
                serde_json::Value::String(profile_ref.to_string()),
            );
        }
        if let Some(profile_key) = input.profile_key {
            data.insert(
                "profileKey".to_string(),
                serde_json::Value::String(profile_key.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::SignatureAffirmation);
        record.actor_id = Some(input.signer_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Amendment & supersession (ADR 0066) ─────────────────────────

    /// Create a correction-authorized record (ADR 0066 §1, Mode 1).
    #[must_use]
    pub fn correction_authorized(input: CorrectionAuthorizedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "correctionTargetEventHash",
            "correctedFieldSet",
            "reason",
            "authorizingActorId",
            "authorityBasis",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "correctionTargetEventHash".to_string(),
            serde_json::Value::String(input.correction_target_event_hash.to_string()),
        );
        data.insert(
            "correctedFieldSet".to_string(),
            serde_json::Value::Array(
                input
                    .corrected_field_set
                    .into_iter()
                    .map(|p| serde_json::Value::String(p.to_string()))
                    .collect(),
            ),
        );
        data.insert(
            "reason".to_string(),
            serde_json::Value::String(input.reason.to_string()),
        );
        data.insert(
            "authorizingActorId".to_string(),
            serde_json::Value::String(input.authorizing_actor_id.to_string()),
        );
        data.insert("authorityBasis".to_string(), input.authority_basis);

        let mut record = Self::blank(ProvenanceKind::CorrectionAuthorized);
        record.actor_id = Some(input.authorizing_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create an amendment-authorized record (ADR 0066 §2, Mode 2).
    #[must_use]
    pub fn amendment_authorized(input: AmendmentAuthorizedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "amendmentTargetEventHash",
            "priorDeterminationHash",
            "reason",
            "authorizingActorId",
            "authorityBasis",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "amendmentTargetEventHash".to_string(),
            serde_json::Value::String(input.amendment_target_event_hash.to_string()),
        );
        data.insert(
            "priorDeterminationHash".to_string(),
            serde_json::Value::String(input.prior_determination_hash.to_string()),
        );
        data.insert(
            "reason".to_string(),
            serde_json::Value::String(input.reason.to_string()),
        );
        data.insert(
            "authorizingActorId".to_string(),
            serde_json::Value::String(input.authorizing_actor_id.to_string()),
        );
        data.insert("authorityBasis".to_string(), input.authority_basis);

        let mut record = Self::blank(ProvenanceKind::AmendmentAuthorized);
        record.actor_id = Some(input.authorizing_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a determination-amended record (ADR 0066 §2).
    #[must_use]
    pub fn determination_amended(input: DeterminationAmendedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "priorDeterminationHash",
            "newDeterminationValue",
            "amendmentAuthorizationEventHash",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "priorDeterminationHash".to_string(),
            serde_json::Value::String(input.prior_determination_hash.to_string()),
        );
        data.insert(
            "newDeterminationValue".to_string(),
            input.new_determination_value,
        );
        data.insert(
            "amendmentAuthorizationEventHash".to_string(),
            serde_json::Value::String(input.amendment_authorization_event_hash.to_string()),
        );

        let mut record = Self::blank(ProvenanceKind::DeterminationAmended);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a rescission-authorized record (ADR 0066 §3, Mode 4).
    #[must_use]
    pub fn rescission_authorized(input: RescissionAuthorizedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "rescissionTargetEventHash",
            "priorDeterminationHash",
            "reason",
            "authorizingActorId",
            "authorityBasis",
            "migrationPinChange",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "rescissionTargetEventHash".to_string(),
            serde_json::Value::String(input.rescission_target_event_hash.to_string()),
        );
        data.insert(
            "priorDeterminationHash".to_string(),
            serde_json::Value::String(input.prior_determination_hash.to_string()),
        );
        data.insert(
            "reason".to_string(),
            serde_json::Value::String(input.reason.to_string()),
        );
        data.insert(
            "authorizingActorId".to_string(),
            serde_json::Value::String(input.authorizing_actor_id.to_string()),
        );
        data.insert("authorityBasis".to_string(), input.authority_basis);
        if let Some(migration_pin_change) = input.migration_pin_change {
            data.insert(
                "migrationPinChange".to_string(),
                serde_json::Value::Object(migration_pin_change),
            );
        }

        let mut record = Self::blank(ProvenanceKind::RescissionAuthorized);
        record.actor_id = Some(input.authorizing_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a determination-rescinded record (ADR 0066 §3).
    #[must_use]
    pub fn determination_rescinded(input: DeterminationRescindedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "priorDeterminationHash",
            "rescissionAuthorizationEventHash",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "priorDeterminationHash".to_string(),
            serde_json::Value::String(input.prior_determination_hash.to_string()),
        );
        data.insert(
            "rescissionAuthorizationEventHash".to_string(),
            serde_json::Value::String(input.rescission_authorization_event_hash.to_string()),
        );

        let mut record = Self::blank(ProvenanceKind::DeterminationRescinded);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a reinstated record (ADR 0066 §4, Mode 5 — Q26).
    #[must_use]
    pub fn reinstated(input: ReinstatedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "priorRescissionEventHash",
            "reactivationAuthorizationEventHash",
            "reason",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "priorRescissionEventHash".to_string(),
            serde_json::Value::String(input.prior_rescission_event_hash.to_string()),
        );
        data.insert(
            "reactivationAuthorizationEventHash".to_string(),
            serde_json::Value::String(input.reactivation_authorization_event_hash.to_string()),
        );
        data.insert(
            "reason".to_string(),
            serde_json::Value::String(input.reason.to_string()),
        );

        let mut record = Self::blank(ProvenanceKind::Reinstated);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create an authorization-attestation record (ADR 0066 §5).
    #[must_use]
    pub fn authorization_attestation(input: AuthorizationAttestationInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "authorizingActorId",
            "authorityBasis",
            "policyPredicate",
            "assuranceLevel",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "authorizingActorId".to_string(),
            serde_json::Value::String(input.authorizing_actor_id.to_string()),
        );
        data.insert("authorityBasis".to_string(), input.authority_basis);
        data.insert(
            "policyPredicate".to_string(),
            serde_json::Value::String(input.policy_predicate.to_string()),
        );
        if let Some(assurance_level) = input.assurance_level {
            data.insert(
                "assuranceLevel".to_string(),
                serde_json::Value::String(assurance_level.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::AuthorizationAttestation);
        record.actor_id = Some(input.authorizing_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Statutory clocks (ADR 0067) ──────────────────────────────

    /// Create a clock-started record (ADR 0067 §2).
    #[must_use]
    pub fn clock_started(input: ClockStartedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "clockId",
            "clockKind",
            "originEventHash",
            "duration",
            "computedDeadline",
            "calendarRef",
            "statuteReference",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "clockId".to_string(),
            serde_json::Value::String(input.clock_id.to_string()),
        );
        data.insert(
            "clockKind".to_string(),
            serde_json::Value::String(input.clock_kind.to_string()),
        );
        data.insert(
            "originEventHash".to_string(),
            serde_json::Value::String(input.origin_event_hash.to_string()),
        );
        data.insert(
            "duration".to_string(),
            serde_json::Value::String(input.duration.to_string()),
        );
        data.insert(
            "computedDeadline".to_string(),
            serde_json::Value::String(input.computed_deadline.to_string()),
        );
        if let Some(calendar_ref) = input.calendar_ref {
            data.insert(
                "calendarRef".to_string(),
                serde_json::Value::String(calendar_ref.to_string()),
            );
        }
        if let Some(statute_reference) = input.statute_reference {
            data.insert(
                "statuteReference".to_string(),
                serde_json::Value::String(statute_reference.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::ClockStarted);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a clock-resolved record (ADR 0067 §3).
    #[must_use]
    pub fn clock_resolved(input: ClockResolvedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "clockId",
            "originClockHash",
            "resolution",
            "resolvedAt",
            "resolvingEventHash",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "clockId".to_string(),
            serde_json::Value::String(input.clock_id.to_string()),
        );
        data.insert(
            "originClockHash".to_string(),
            serde_json::Value::String(input.origin_clock_hash.to_string()),
        );
        let (resolution_str, resolving_event_hash): (&str, Option<&str>) =
            match &input.resolution {
                ClockResolvedResolution::Satisfied {
                    resolving_event_hash,
                } => ("satisfied", *resolving_event_hash),
                ClockResolvedResolution::Elapsed {
                    resolving_event_hash,
                } => ("elapsed", *resolving_event_hash),
                ClockResolvedResolution::Cancelled {
                    resolving_event_hash,
                } => ("cancelled", *resolving_event_hash),
                ClockResolvedResolution::Paused {
                    resolving_event_hash,
                } => ("paused", Some(*resolving_event_hash)),
            };
        data.insert(
            "resolution".to_string(),
            serde_json::Value::String(resolution_str.to_string()),
        );
        data.insert(
            "resolvedAt".to_string(),
            serde_json::Value::String(input.resolved_at.to_string()),
        );
        if let Some(resolving_event_hash) = resolving_event_hash {
            data.insert(
                "resolvingEventHash".to_string(),
                serde_json::Value::String(resolving_event_hash.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::ClockResolved);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Identity attestation (ADR 0068) ──────────────────────────

    /// Create an identity-attestation record (ADR 0068 §4, Q15).
    #[must_use]
    pub fn identity_attestation(input: IdentityAttestationInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "subjectGlobalId",
            "assuranceLevel",
            "attestationProvider",
            "providerAttestationId",
            "attestedAt",
            "validUntil",
            "attestedPredicates",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "subjectGlobalId".to_string(),
            serde_json::Value::String(input.subject_global_id.to_string()),
        );
        data.insert(
            "assuranceLevel".to_string(),
            serde_json::Value::String(input.assurance_level.to_string()),
        );
        data.insert(
            "attestationProvider".to_string(),
            serde_json::Value::String(input.attestation_provider.to_string()),
        );
        data.insert(
            "providerAttestationId".to_string(),
            serde_json::Value::String(input.provider_attestation_id.to_string()),
        );
        data.insert(
            "attestedAt".to_string(),
            serde_json::Value::String(input.attested_at.to_string()),
        );
        if let Some(valid_until) = input.valid_until {
            data.insert(
                "validUntil".to_string(),
                serde_json::Value::String(valid_until.to_string()),
            );
        }
        data.insert(
            "attestedPredicates".to_string(),
            serde_json::Value::Array(
                input
                    .attested_predicates
                    .into_iter()
                    .map(|p| serde_json::Value::String(p.to_string()))
                    .collect(),
            ),
        );

        let mut record = Self::blank(ProvenanceKind::IdentityAttestation);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Clock skew (ADR 0069) ────────────────────────────────────

    /// Create a clock-skew-observed record (ADR 0069 §3).
    #[must_use]
    pub fn clock_skew_observed(input: ClockSkewObservedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "processorAuthoredAt",
            "substrateCreatedAt",
            "skewMilliseconds",
            "thresholdMilliseconds",
            "eventHash",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "processorAuthoredAt".to_string(),
            serde_json::Value::String(input.processor_authored_at.to_string()),
        );
        data.insert(
            "substrateCreatedAt".to_string(),
            serde_json::Value::String(input.substrate_created_at.to_string()),
        );
        data.insert(
            "skewMilliseconds".to_string(),
            serde_json::Value::Number(serde_json::Number::from(input.skew_milliseconds)),
        );
        data.insert(
            "thresholdMilliseconds".to_string(),
            serde_json::Value::Number(serde_json::Number::from(input.threshold_milliseconds)),
        );
        data.insert(
            "eventHash".to_string(),
            serde_json::Value::String(input.event_hash.to_string()),
        );

        let mut record = Self::blank(ProvenanceKind::ClockSkewObserved);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Failure & compensation (ADR 0070) ────────────────────────

    /// Create a commit-attempt-failure record (ADR 0070 §2).
    #[must_use]
    pub fn commit_attempt_failure(input: CommitAttemptFailureInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "targetEventHash",
            "failureKind",
            "attemptCount",
            "retryBudgetRemainingMs",
            "errorPayload",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "targetEventHash".to_string(),
            serde_json::Value::String(input.target_event_hash.to_string()),
        );
        data.insert(
            "failureKind".to_string(),
            serde_json::Value::String(input.failure_kind.as_camel_str().to_string()),
        );
        data.insert(
            "attemptCount".to_string(),
            serde_json::Value::Number(serde_json::Number::from(input.attempt_count)),
        );
        data.insert(
            "retryBudgetRemainingMs".to_string(),
            serde_json::Value::Number(serde_json::Number::from(input.retry_budget_remaining_ms)),
        );
        if let Some(error_payload) = input.error_payload {
            data.insert("errorPayload".to_string(), error_payload);
        }

        let mut record = Self::blank(ProvenanceKind::CommitAttemptFailure);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create an authorization-rejected record (ADR 0070 §4).
    #[must_use]
    pub fn authorization_rejected(input: AuthorizationRejectedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "attemptedActorId",
            "attemptedAction",
            "targetResourceId",
            "rejectionReason",
            "policyDecisionRef",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "attemptedActorId".to_string(),
            serde_json::Value::String(input.attempted_actor_id.to_string()),
        );
        data.insert(
            "attemptedAction".to_string(),
            serde_json::Value::String(input.attempted_action.to_string()),
        );
        data.insert(
            "targetResourceId".to_string(),
            serde_json::Value::String(input.target_resource_id.to_string()),
        );
        data.insert(
            "rejectionReason".to_string(),
            serde_json::Value::String(input.rejection_reason.to_string()),
        );
        if let Some(policy_decision_ref) = input.policy_decision_ref {
            data.insert(
                "policyDecisionRef".to_string(),
                serde_json::Value::String(policy_decision_ref.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::AuthorizationRejected);
        record.actor_id = Some(input.attempted_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Migration & version pins (ADR 0071) ──────────────────────

    /// Create a migration-pin-changed record (ADR 0071 §3).
    #[must_use]
    pub fn migration_pin_changed(input: MigrationPinChangedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "priorPinSet",
            "newPinSet",
            "authorizingActorId",
            "authorityBasis",
            "migrationRationale",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert("priorPinSet".to_string(), input.prior_pin_set);
        data.insert("newPinSet".to_string(), input.new_pin_set);
        data.insert(
            "authorizingActorId".to_string(),
            serde_json::Value::String(input.authorizing_actor_id.to_string()),
        );
        data.insert("authorityBasis".to_string(), input.authority_basis);
        data.insert(
            "migrationRationale".to_string(),
            serde_json::Value::String(input.migration_rationale.to_string()),
        );

        let mut record = Self::blank(ProvenanceKind::MigrationPinChanged);
        record.actor_id = Some(input.authorizing_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }
}

/// Merge an optional caller-supplied `context` map into a fresh data map,
/// dropping any keys that collide with the constructor's required fields.
/// Constructor args remain the source of truth — `context` (which may
/// originate from untrusted scratch) MUST NOT be able to overwrite the
/// schema-shaping discriminators.
fn merge_context(
    context: Option<serde_json::Map<String, serde_json::Value>>,
    required: &[&str],
) -> serde_json::Map<String, serde_json::Value> {
    let mut data = serde_json::Map::new();
    if let Some(context) = context {
        for (k, v) in context {
            if required.iter().any(|r| *r == k) {
                continue;
            }
            data.insert(k, v);
        }
    }
    data
}

impl std::fmt::Display for ProvenanceRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{:?}", self.id, self.record_kind)?;
        if !self.timestamp.is_empty() {
            write!(f, " at={}", self.timestamp)?;
        }
        if let Some(from) = &self.from_state {
            write!(f, " from={from}")?;
        }
        if let Some(to) = &self.to_state {
            write!(f, " to={to}")?;
        }
        if let Some(event) = &self.event {
            write!(f, " event={event}")?;
        }
        Ok(())
    }
}
