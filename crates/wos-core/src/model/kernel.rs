// Rust guideline compliant 2026-02-21

//! Typed model for WOS Kernel Documents (Layer 0).
//!
//! Deserialized from JSON via serde. The evaluation algorithm in [`crate::eval`]
//! operates on these types, not on raw `serde_json::Value`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A WOS Kernel Document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelDocument {
    /// Document type marker. Must be `"1.0"`.
    #[serde(rename = "$wosKernel")]
    pub wos_kernel: String,

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActorKind {
    Human,
    System,
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
    /// Event that triggers this transition.
    pub event: String,

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
    #[serde(default)]
    pub event: Option<String>,

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseFile {
    /// Field definitions.
    #[serde(default)]
    pub fields: HashMap<String, FieldDefinition>,

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
}
