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
