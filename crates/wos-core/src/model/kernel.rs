// Rust guideline compliant 2026-02-21

//! Typed model for WOS Kernel Documents (Layer 0).
//!
//! Deserialized from JSON via serde. The evaluation algorithm in [`crate::eval`]
//! operates on these types, not on raw `serde_json::Value`.

use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// A WOS Kernel Document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelDocument {
    /// Document type marker. Must be `"1.0"`.
    #[serde(rename = "$wosWorkflow")]
    pub wos_workflow: String,

    /// Optional JSON Schema URI for editor validation.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,

    /// Canonical URL identifying this workflow definition.
    #[serde(default)]
    pub url: Option<String>,

    /// Document version.
    #[serde(default)]
    pub version: Option<String>,

    /// Human-readable title.
    #[serde(default)]
    pub title: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Document status.
    #[serde(default)]
    pub status: Option<String>,

    /// Impact level classification (Kernel S6).
    #[serde(default)]
    pub impact_level: Option<ImpactLevel>,

    /// Actor declarations.
    #[serde(default)]
    pub actors: Vec<Actor>,

    /// Lifecycle topology.
    pub lifecycle: Lifecycle,

    /// Case state schema.
    #[serde(default)]
    pub case_file: Option<CaseFile>,

    /// Named contract references (Kernel S11).
    #[serde(default)]
    pub contracts: HashMap<String, ContractReference>,

    /// Provenance configuration (Kernel S8).
    #[serde(default)]
    pub provenance: Option<serde_json::Value>,

    /// Execution configuration (Kernel S9).
    #[serde(default)]
    pub execution: Option<ExecutionConfig>,

    /// Evaluation mode (Runtime Companion S10).
    #[serde(default)]
    pub evaluation_mode: Option<EvaluationMode>,

    /// Cascade depth cap for `$related.*` events (Kernel S4.10).
    #[serde(default)]
    pub max_relationship_event_depth: Option<u32>,

    /// Extension data. Keys MUST start with `x-`.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// A contract reference (Kernel S11).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractReference {
    /// Binding type.
    pub binding: String,

    /// Reference URI (Formspec Definition or JSON Schema).
    #[serde(rename = "ref")]
    pub reference: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Mapping used to prefill a Formspec task response.
    #[serde(default)]
    pub prefill_mapping_ref: Option<String>,

    /// Mapping used to project a completed Formspec response.
    #[serde(default)]
    pub response_mapping_ref: Option<String>,
}

/// Execution configuration (Kernel S9).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionConfig {
    /// Maximum total workflow duration (ISO 8601).
    #[serde(default)]
    pub workflow_timeout: Option<String>,

    /// Default task timeout (ISO 8601).
    #[serde(default)]
    pub default_task_timeout: Option<String>,

    /// Default service timeout (ISO 8601).
    #[serde(default)]
    pub default_service_timeout: Option<String>,

    /// Whether the workflow scope is compensable (Kernel S9.5).
    #[serde(default)]
    pub compensable: bool,

    /// Instance versioning policy.
    #[serde(default)]
    pub instance_versioning: Option<String>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Evaluation mode (Runtime Companion S10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvaluationMode {
    EventDriven,
    Continuous,
}

/// Impact level classification (Kernel S6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImpactLevel {
    RightsImpacting,
    SafetyImpacting,
    Operational,
    Informational,
}

impl ImpactLevel {
    /// Whether this impact level requires due process (rights or safety).
    pub fn requires_due_process(self) -> bool {
        matches!(self, Self::RightsImpacting | Self::SafetyImpacting)
    }
}

/// An actor declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    /// Unique actor identifier.
    pub id: String,

    /// Actor type.
    #[serde(rename = "type")]
    pub kind: ActorKind,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Actor type (Kernel S3).
///
/// `Agent` is a first-class variant per ADR 0064. Agent-typed actors live in
/// the `actors[]` registry alongside humans and services; per-agent runtime
/// declarations (capabilities, autonomy, deontic constraints, fallback chain,
/// drift monitoring, invoker discriminator) live in the workflow's `agents[]`
/// embedded block joined by `id`. Lint rule `WOS-AGENT-XREF-001` enforces the
/// cross-reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActorKind {
    Human,
    System,
    Agent,
}

/// Lifecycle topology (Kernel S4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lifecycle {
    /// Initial state identifier.
    pub initial_state: String,

    /// Map of state identifiers to state definitions.
    pub states: IndexMap<String, State>,

    /// Named milestones.
    #[serde(default)]
    pub milestones: HashMap<String, Milestone>,
}

/// A lifecycle state (Kernel S4.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    /// State type.
    #[serde(rename = "type")]
    pub kind: StateKind,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Outgoing transitions, evaluated in document order.
    #[serde(default)]
    pub transitions: Vec<Transition>,

    /// Semantic tags for governance attachment.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Entry actions.
    #[serde(default)]
    pub on_entry: Vec<Action>,

    /// Exit actions.
    #[serde(default)]
    pub on_exit: Vec<Action>,

    /// Initial substate for compound states.
    #[serde(default)]
    pub initial_state: Option<String>,

    /// Substates for compound states.
    #[serde(default)]
    pub states: IndexMap<String, State>,

    /// Regions for parallel states.
    #[serde(default)]
    pub regions: IndexMap<String, Region>,

    /// Cancellation policy for parallel states.
    #[serde(default)]
    pub cancellation_policy: Option<CancellationPolicy>,

    /// History state mode for compound states.
    #[serde(default)]
    pub history_state: Option<HistoryMode>,

    /// Machine-readable outcome code for final states (Kernel S4.3).
    #[serde(default)]
    pub outcome_code: Option<String>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// State type discriminator (Kernel S4.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StateKind {
    Atomic,
    Compound,
    Parallel,
    Final,
}

/// Cancellation policy for parallel states (Kernel S4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CancellationPolicy {
    WaitAll,
    CancelSiblings,
    FailFast,
}

/// History state mode (Kernel S4.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryMode {
    Shallow,
    Deep,
}

/// Kernel-generated or author-declared timer categories for [`TransitionEvent::Timer`]
/// (Kernel §4.10, §9.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimerEventSource {
    Task,
    Service,
    State,
    Signal,
    Workflow,
    Custom,
}

/// Signal delivery scope for [`TransitionEvent::Signal`] (Kernel §4.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignalScope {
    Instance,
    Related,
    Broadcast,
}

/// Typed transition / timer-fire event (Kernel §4.5–§4.10, TODO #20).
///
/// Replaces free-form event strings with a closed five-kind union. Runtime
/// `process_event` still receives string names; [`TransitionEvent::matches_runtime_dispatch`]
/// compares those names to this shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TransitionEvent {
    Timer {
        #[serde(rename = "timerId")]
        timer_id: String,
        source: TimerEventSource,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none", rename = "expiresAt")]
        expires_at: Option<String>,
        /// When set, the timer fires this exact string (e.g. dotted author names).
        #[serde(default, skip_serializing_if = "Option::is_none", rename = "firesAs")]
        fires_as: Option<String>,
    },
    Message {
        name: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            rename = "correlationKey"
        )]
        correlation_key: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
    Signal {
        name: String,
        scope: SignalScope,
    },
    Condition {
        expression: String,
    },
    Error {
        code: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            rename = "actionPath"
        )]
        action_path: Option<String>,
    },
}

impl TransitionEvent {
    /// String used for `process_event` matching and the same labels the
    /// runtime compares against [`Self::matches_runtime_dispatch`].
    #[must_use]
    pub fn runtime_dispatch_label(&self) -> String {
        match self {
            Self::Message { name, .. } | Self::Signal { name, .. } => name.clone(),
            Self::Timer {
                timer_id,
                source,
                fires_as,
                ..
            } => Self::timer_fires_string(timer_id, *source, fires_as.as_deref()),
            Self::Condition { expression } => format!("condition:{expression}"),
            Self::Error { .. } => "$error".to_string(),
        }
    }

    /// Human-readable event summary for graphs, notices, and search — not
    /// necessarily equal to [`Self::runtime_dispatch_label`] (e.g. `error`
    /// events carry a code in the typed object but dispatch as `$error`).
    #[must_use]
    pub fn authoring_display_label(&self) -> String {
        match self {
            Self::Error { code, .. } => format!("error:{code}"),
            Self::Condition { expression } => format!("condition:{expression}"),
            _ => self.runtime_dispatch_label(),
        }
    }

    #[must_use]
    pub fn matches_runtime_dispatch(&self, event: &str) -> bool {
        match self {
            Self::Message { name, .. } => name == event,
            Self::Signal { name, .. } => name == event,
            Self::Timer {
                timer_id,
                source,
                fires_as,
                ..
            } => Self::timer_fires_string(timer_id, *source, fires_as.as_deref()) == event,
            Self::Condition { .. } => false,
            Self::Error { .. } => event == "$error",
        }
    }

    /// Event name emitted when a `startTimer` timer expires (may differ from `timer_id`).
    #[must_use]
    pub fn start_timer_fires_string(&self) -> String {
        match self {
            Self::Timer {
                timer_id,
                source,
                fires_as,
                ..
            } => Self::timer_fires_string(timer_id, *source, fires_as.as_deref()),
            Self::Message { name, .. } => name.clone(),
            Self::Signal { name, .. } => name.clone(),
            Self::Condition { expression } => expression.clone(),
            Self::Error { code, .. } => code.clone(),
        }
    }

    fn timer_fires_string(
        timer_id: &str,
        source: TimerEventSource,
        fires_as: Option<&str>,
    ) -> String {
        if let Some(f) = fires_as {
            if !f.is_empty() {
                return f.to_string();
            }
        }
        match source {
            TimerEventSource::Task => "$timeout.task".to_string(),
            TimerEventSource::Service => "$timeout.service".to_string(),
            TimerEventSource::State => "$timeout.state".to_string(),
            TimerEventSource::Signal => "$timeout.signal".to_string(),
            TimerEventSource::Workflow => "$timeout.workflow".to_string(),
            TimerEventSource::Custom => format!("$timeout.{timer_id}"),
        }
    }

    /// Map a bare trigger token (authoring `add_transition`, tests) to the typed union.
    /// JSON documents may still supply the same strings under `transition.event`; serde
    /// uses the same coercion via [`transition_event_coerce_from_str`].
    #[must_use]
    pub fn from_authoring_trigger(s: &str) -> Self {
        transition_event_coerce_from_str(s)
    }
}

fn transition_event_coerce_from_str(s: &str) -> TransitionEvent {
    let s = s.trim();
    match s {
        "$join" => TransitionEvent::Signal {
            name: "$join".to_string(),
            scope: SignalScope::Instance,
        },
        "$error" => TransitionEvent::Error {
            code: "kernel.error".to_string(),
            action_path: None,
        },
        _ if s.starts_with("$timeout.") => {
            let rest = s.strip_prefix("$timeout.").unwrap_or(s);
            let source = match rest {
                "task" => TimerEventSource::Task,
                "service" => TimerEventSource::Service,
                "state" => TimerEventSource::State,
                "signal" => TimerEventSource::Signal,
                "workflow" => TimerEventSource::Workflow,
                _ => TimerEventSource::Custom,
            };
            TransitionEvent::Timer {
                timer_id: rest.to_string(),
                source,
                duration: None,
                expires_at: None,
                fires_as: None,
            }
        }
        _ if s.starts_with("$related.") => TransitionEvent::Signal {
            // Preserve the full `$related.*` name so that `matches_runtime_dispatch`
            // (which compares Signal.name == event by exact equality) matches the
            // kernel-defined relationship event names emitted at runtime
            // (`$related.stateChanged`, `$related.resolved`, `$related.holdReleased`,
            // …). Stripping the prefix here previously made bare-string transitions
            // for relationship events silently unmatchable.
            name: s.to_string(),
            scope: SignalScope::Related,
        },
        "$compensation.complete" => TransitionEvent::Signal {
            name: "$compensation.complete".to_string(),
            scope: SignalScope::Instance,
        },
        _ if s.starts_with('$') => TransitionEvent::Message {
            name: s.to_string(),
            correlation_key: None,
            data: None,
        },
        _ => TransitionEvent::Message {
            name: s.to_string(),
            correlation_key: None,
            data: None,
        },
    }
}

fn deserialize_opt_transition_event<'de, D>(
    deserializer: D,
) -> Result<Option<TransitionEvent>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(v) = v else {
        return Ok(None);
    };
    match v {
        serde_json::Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                Ok(None)
            } else {
                Ok(Some(transition_event_coerce_from_str(t)))
            }
        }
        serde_json::Value::Object(_) => serde_json::from_value::<TransitionEvent>(v)
            .map(Some)
            .map_err(D::Error::custom),
        _ => Err(D::Error::custom(
            "transition event must be a string or a TransitionEvent object",
        )),
    }
}

/// A parallel state region.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    /// Initial state within this region.
    pub initial_state: String,

    /// States within this region.
    #[serde(default)]
    pub states: IndexMap<String, State>,
}

/// A lifecycle transition (Kernel S4.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    /// Event that triggers this transition for explicit `process_event`
    /// delivery.
    ///
    /// When omitted (or serialized with only whitespace, treated as absent),
    /// the transition does **not** match any external event name. In
    /// `continuous` evaluation mode it still participates in the post-mutation
    /// guard re-scan (Runtime Companion §10.3).
    #[serde(
        default,
        deserialize_with = "deserialize_opt_transition_event",
        skip_serializing_if = "Option::is_none"
    )]
    pub event: Option<TransitionEvent>,

    /// Target state identifier.
    pub target: String,

    /// Guard expression (FEL). Evaluated in document order.
    #[serde(default)]
    pub guard: Option<String>,

    /// Actions executed during this transition.
    #[serde(default)]
    pub actions: Vec<Action>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Semantic tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Transition {
    /// Whether this transition participates in continuous-mode post-mutation
    /// guard re-evaluation (Runtime Companion §10.3).
    pub fn participates_in_continuous_rescan(&self) -> bool {
        match &self.event {
            None => true,
            Some(TransitionEvent::Condition { .. }) => true,
            Some(_) => false,
        }
    }

    /// Event label for diagnostics: matches runtime `process_event` names.
    #[must_use]
    pub fn event_dispatch_label(&self) -> String {
        self.event
            .as_ref()
            .map(TransitionEvent::runtime_dispatch_label)
            .unwrap_or_else(|| "(guard-only)".to_string())
    }

    /// Parallel join: signal `$join` scoped to this instance (Kernel §4.8).
    #[must_use]
    pub fn is_parallel_join_transition(&self) -> bool {
        matches!(
            &self.event,
            Some(TransitionEvent::Signal {
                name,
                scope: SignalScope::Instance
            }) if name == "$join"
        )
    }
}

/// A lifecycle action (Kernel S9.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    /// Action type.
    pub action: ActionKind,

    /// Task reference (createTask).
    #[serde(default)]
    pub task_ref: Option<String>,

    /// Actor to assign (createTask).
    #[serde(default)]
    pub assign_to: Option<String>,

    /// Service reference (invokeService).
    #[serde(default)]
    pub service_ref: Option<String>,

    /// Idempotency key (invokeService).
    #[serde(default)]
    pub idempotency_key: Option<String>,

    /// Correlation key (invokeService).
    #[serde(default)]
    pub correlation_key: Option<String>,

    /// Case file field path (setData).
    #[serde(default)]
    pub path: Option<String>,

    /// Value to set (setData).
    #[serde(default)]
    pub value: Option<serde_json::Value>,

    /// Event type (emitEvent).
    #[serde(default)]
    pub event_type: Option<String>,

    /// Event data (emitEvent).
    #[serde(default)]
    pub data: Option<serde_json::Value>,

    /// Timer identifier (startTimer, cancelTimer).
    #[serde(default)]
    pub timer_id: Option<String>,

    /// Duration (startTimer).
    #[serde(default)]
    pub duration: Option<String>,

    /// Deadline (startTimer).
    #[serde(default)]
    pub deadline: Option<String>,

    /// Event to fire (startTimer).
    #[serde(
        default,
        deserialize_with = "deserialize_opt_transition_event",
        skip_serializing_if = "Option::is_none"
    )]
    pub event: Option<TransitionEvent>,

    /// Log message (log).
    #[serde(default)]
    pub message: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Contract reference for validation.
    #[serde(default)]
    pub contract_ref: Option<String>,

    /// Mapping used to prefill a Formspec task response.
    #[serde(default)]
    pub prefill_mapping_ref: Option<String>,

    /// Mapping used to project a completed Formspec response.
    #[serde(default)]
    pub response_mapping_ref: Option<String>,

    /// Event emitted when a Formspec-backed task completes.
    #[serde(default)]
    pub completion_event: Option<String>,

    /// Event emitted when a Formspec-backed task fails.
    #[serde(default)]
    pub failure_event: Option<String>,

    /// Compensating action (Kernel S9.5).
    #[serde(default)]
    pub compensating_action: Option<Box<Action>>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Action type discriminator (Kernel S9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionKind {
    CreateTask,
    InvokeService,
    SetData,
    EmitEvent,
    StartTimer,
    CancelTimer,
    Log,
}

/// Case file schema (Kernel S5).
///
/// A case file is either declared inline via `fields` or referenced via
/// `contract_ref` (recommended binding: Formspec Definition through the
/// canonical `contractHook` seam, ADR 0077 §10.2). The two shapes are mutually
/// exclusive at the schema level (`oneOf`); the Rust model carries both as
/// `Option` and trusts the schema to enforce exclusivity at parse time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseFile {
    /// Inline field definitions. Mutually exclusive with `contract_ref` per
    /// the schema's `oneOf`. When omitted, the case-file shape comes from the
    /// referenced contract.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, FieldDefinition>,

    /// External contract reference. Mutually exclusive with `fields`. The URI
    /// names the contract document (Formspec Definition recommended, JSON
    /// Schema baseline). Resolved through the contracts-resolver port at
    /// runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_ref: Option<String>,

    /// Optional version pin for `contract_ref`. When omitted, processors
    /// resolve the latest published version. Pinning is RECOMMENDED for case
    /// instances that must replay against archived semantics (Kernel §9.6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_version: Option<String>,

    /// Case relationships (Kernel S5.5).
    #[serde(default)]
    pub relationships: Vec<CaseRelationship>,
}

/// A case file field definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Field type.
    #[serde(rename = "type")]
    pub kind: String,

    /// Whether the field is required at instance creation / contract
    /// validation (Kernel §5; matches the schema's `FieldDeclaration.required`
    /// surface). Authoring-time hint; runtime contract validation enforces it
    /// through the contracts-resolver port.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,

    /// Default value.
    #[serde(default)]
    pub default: Option<serde_json::Value>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

/// A case relationship (Kernel S5.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseRelationship {
    /// Relationship type.
    #[serde(rename = "type")]
    pub kind: String,

    /// URI of the related case.
    pub target_case: String,

    /// Semantic relationship label.
    #[serde(default)]
    pub relationship: Option<String>,

    /// Whether the inverse should also be recorded.
    #[serde(default)]
    pub bidirectional: bool,
}

/// A milestone condition (Kernel S4.13).
///
/// The milestone identifier is the map key in `Lifecycle.milestones`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// FEL condition expression.
    pub condition: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// When the processor evaluates the condition (Kernel §4.13).
    ///
    /// Defaults to `writeSettled`, the only currently defined mode: the
    /// condition is evaluated after every durable case-state write. Held as
    /// a string so future trigger modes can be added without breaking
    /// roundtrip serialization on existing documents.
    #[serde(
        default,
        rename = "triggerMode",
        skip_serializing_if = "Option::is_none"
    )]
    pub trigger_mode: Option<String>,
}
