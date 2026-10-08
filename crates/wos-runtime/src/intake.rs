// Rust guideline compliant 2026-02-21

//! Host-side intake-acceptance seam.
//!
//! Task bindings cover task presentation and task submission. Intake handoff
//! acceptance is a separate host-side boundary: a host interprets
//! binding-native intake evidence, applies local acceptance policy, and may
//! emit auxiliary provenance during that acceptance step.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wos_core::ProvenanceRecord;

use crate::binding::BindingError;
use crate::runtime::RuntimeError;

/// Binding-derived case intent before host policy decides what to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum IntakeCaseIntent {
    /// The handoff targets an already-governed case.
    AttachToExistingCase {
        /// Existing governed case reference carried by the handoff.
        case_ref: String,
    },

    /// The handoff requests creation of a governed case after acceptance.
    RequestGovernedCaseCreation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeCaseDefinition {
    /// Governing kernel URL for a newly created case.
    pub definition_url: String,
    /// Governing kernel version for a newly created case.
    pub definition_version: String,
}

/// Host-visible accepted case action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum IntakeCaseDisposition {
    /// Acceptance attaches evidence to an already-governed case.
    AttachToExistingCase {
        /// Existing governed case reference.
        case_ref: String,
    },

    /// Acceptance created a new governed case from public intake evidence.
    CreateGovernedCase {
        /// Newly created governed case reference.
        case_ref: String,
        /// Kernel definition used for the created governed case.
        definition: IntakeCaseDefinition,
        /// Initial case-state seed supplied by host policy.
        initial_case_state: Option<serde_json::Value>,
    },
}

/// Input for an intake-interpretation adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeAcceptanceRequest {
    /// Binding-native intake handoff document to parse and validate.
    pub document: serde_json::Value,
    /// Actor or service handling the intake boundary.
    pub actor_id: Option<String>,
    /// Governed case reference chosen by the host when one is already known.
    ///
    /// This is optional because host policy may reject or defer an intake
    /// handoff before choosing or minting a governed case identity.
    pub governed_case_ref: Option<String>,
    /// Governing kernel definition for a newly created case.
    pub governed_case_definition: Option<IntakeCaseDefinition>,
    /// Initial case-state seed for a newly created case.
    pub initial_case_state: Option<serde_json::Value>,
}

/// Binding-owned interpretation of an intake handoff.
#[derive(Debug, Clone)]
pub struct IntakeInterpretation {
    /// Stable idempotency and receipt identifier for this intake request.
    pub intake_id: String,
    /// Binding-derived case intent before host policy runs.
    pub case_intent: IntakeCaseIntent,
}

/// Host-policy context for an intake handoff.
#[derive(Debug, Clone)]
pub struct IntakePolicyContext {
    /// Binding discriminator being evaluated.
    pub binding: String,
    /// Original host-side request.
    pub request: IntakeAcceptanceRequest,
    /// Binding-owned interpretation of the intake handoff.
    pub interpretation: IntakeInterpretation,
}

/// Final host-visible outcome of intake acceptance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum IntakeAcceptanceOutcome {
    /// Intake was accepted and attached or promoted to a governed case.
    Accepted {
        /// Final case disposition after acceptance.
        case_disposition: IntakeCaseDisposition,
    },

    /// Intake was rejected by host policy.
    Rejected {
        /// Machine-readable rejection code.
        code: String,
    },

    /// Intake was deferred by host policy.
    Deferred {
        /// Machine-readable deferral code.
        code: String,
    },
}

/// Original metadata retained atomically with process creation.
///
/// This internal runtime evidence never grants task access. Hosts must validate
/// the actual scoped process association and current creation-disclosure policy.
/// Persisted creation evidence is required and never reconstructed from mutable task state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProcessCreationSnapshot {
    /// Actual internal process identifier.
    pub process_id: String,
    /// Actual case ledger bound at creation.
    pub case_ledger_id: String,
    /// Tenant resolved at creation.
    pub tenant: String,
    /// Original governing definition URL.
    pub definition_url: String,
    /// Original governing definition version.
    pub definition_version: String,
    /// Original process creation timestamp.
    pub created_at: String,
    /// Task metadata after creation staging, in creation order.
    pub tasks: Vec<TaskCreationSnapshot>,
}

/// Original task metadata without responses or presentation bodies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskCreationSnapshot {
    /// Actual internal task identifier.
    pub task_id: String,
    /// Original workflow task reference used for title and kind mapping.
    pub task_ref: String,
    /// Original selected authored catalog title, if present.
    pub catalog_title: Option<String>,
    /// Original selected authored catalog pattern, if present.
    pub catalog_pattern: Option<String>,
    /// Original nonterminal runtime task status.
    pub status: wos_core::instance::ActiveTaskStatus,
    /// Original assigned actor.
    pub assigned_actor: Option<String>,
    /// Original deadline.
    pub deadline: Option<String>,
    /// Original impact level.
    pub impact_level: Option<wos_core::model::kernel::ImpactLevel>,
    /// Original task creation timestamp.
    pub created_at: String,
    /// Original last update timestamp at creation commit.
    pub updated_at: String,
}

impl ProcessCreationSnapshot {
    pub(crate) fn capture(
        process: &wos_core::instance::WorkflowProcess,
        kernel: &wos_core::model::kernel::KernelDocument,
    ) -> Result<Self, crate::store::StoreError> {
        let catalog = kernel
            .governance
            .as_ref()
            .and_then(|value| value.get("taskCatalog"));
        let catalog = catalog
            .map(|value| {
                value
                    .as_array()
                    .ok_or_else(|| invalid_creation("task catalog is not an array"))
            })
            .transpose()?;
        let mut tasks = Vec::with_capacity(process.active_tasks.len());
        for task in &process.active_tasks {
            let matches: Vec<_> = catalog
                .into_iter()
                .flatten()
                .filter(|entry| {
                    entry.get("id").and_then(serde_json::Value::as_str)
                        == Some(task.task_ref.as_str())
                })
                .collect();
            if matches.len() > 1 {
                return Err(invalid_creation(
                    "duplicate original task catalog reference",
                ));
            }
            let metadata = |field: &str| -> Result<Option<String>, crate::store::StoreError> {
                matches
                    .first()
                    .and_then(|entry| entry.get(field))
                    .map(|value| {
                        value
                            .as_str()
                            .filter(|text| !text.is_empty())
                            .map(str::to_string)
                            .ok_or_else(|| {
                                invalid_creation("original task catalog metadata is invalid")
                            })
                    })
                    .transpose()
            };
            tasks.push(TaskCreationSnapshot {
                task_id: task.task_id.clone(),
                task_ref: task.task_ref.clone(),
                catalog_title: metadata("title")?,
                catalog_pattern: metadata("pattern")?,
                status: task.status,
                assigned_actor: task.assigned_actor.clone(),
                deadline: task.deadline.clone(),
                impact_level: task.impact_level,
                created_at: task.created_at.clone(),
                updated_at: task.updated_at.clone(),
            });
        }
        let snapshot = Self {
            process_id: process.process_id.clone(),
            case_ledger_id: process.case_ledger_id.clone(),
            tenant: process.tenant.clone(),
            definition_url: process.definition_url.clone(),
            definition_version: process.definition_version.clone(),
            created_at: process.created_at.clone(),
            tasks,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Checks original identity and metadata without reading current task state.
    ///
    /// This validates retained evidence, never access or current assignment.
    ///
    /// # Errors
    /// Returns invalid-creation evidence for malformed identity, metadata, or duplicate task IDs.
    pub fn validate(&self) -> Result<(), crate::store::StoreError> {
        use stack_common_typeid as typeid;
        if !typeid::is_valid_type_id(&self.process_id, Some(typeid::PROCESS_PREFIX))
            || !typeid::is_valid_type_id(&self.case_ledger_id, Some(typeid::CASE_PREFIX))
            || typeid::extract_tenant(&self.process_id) != Some(self.tenant.as_str())
            || typeid::extract_tenant(&self.case_ledger_id) != Some(self.tenant.as_str())
            || self.definition_url.is_empty()
            || self.definition_version.is_empty()
            || chrono::DateTime::parse_from_rfc3339(&self.created_at).is_err()
        {
            return Err(invalid_creation(
                "original process identity or metadata is invalid",
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for task in &self.tasks {
            if crate::task_process_id(&task.task_id).ok().as_deref()
                != Some(self.process_id.as_str())
                || !seen.insert(&task.task_id)
                || task.task_ref.is_empty()
                || task.catalog_title.as_ref().is_some_and(String::is_empty)
                || task.catalog_pattern.as_ref().is_some_and(String::is_empty)
                || task.assigned_actor.as_ref().is_some_and(String::is_empty)
                || chrono::DateTime::parse_from_rfc3339(&task.created_at).is_err()
                || chrono::DateTime::parse_from_rfc3339(&task.updated_at).is_err()
                || task
                    .deadline
                    .as_ref()
                    .is_some_and(|value| chrono::DateTime::parse_from_rfc3339(value).is_err())
            {
                return Err(invalid_creation(
                    "original task identity or metadata is invalid",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn validate_created_process(
        &self,
        process: &wos_core::instance::WorkflowProcess,
    ) -> Result<(), crate::store::StoreError> {
        self.validate_process(process)?;
        if self.definition_version != process.definition_version
            || self.tasks.len() != process.active_tasks.len()
        {
            return Err(invalid_creation(
                "creation snapshot differs from the original staged process",
            ));
        }
        for (original, actual) in self.tasks.iter().zip(&process.active_tasks) {
            if original.task_id != actual.task_id
                || original.task_ref != actual.task_ref
                || original.status != actual.status
                || original.assigned_actor != actual.assigned_actor
                || original.deadline != actual.deadline
                || original.impact_level != actual.impact_level
                || original.created_at != actual.created_at
                || original.updated_at != actual.updated_at
            {
                return Err(invalid_creation(
                    "creation snapshot differs from the original staged task metadata",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn validate_process(
        &self,
        process: &wos_core::instance::WorkflowProcess,
    ) -> Result<(), crate::store::StoreError> {
        self.validate()?;
        // Definition version may legitimately migrate; the original pin remains unchanged.
        if self.process_id != process.process_id
            || self.case_ledger_id != process.case_ledger_id
            || self.tenant != process.tenant
            || self.definition_url != process.definition_url
            || self.created_at != process.created_at
        {
            return Err(invalid_creation(
                "creation snapshot does not belong to the actual process",
            ));
        }
        Ok(())
    }

    pub(crate) fn validate_intake(
        &self,
        outcome: &IntakeAcceptanceOutcome,
        request: &IntakeAcceptanceRequest,
    ) -> Result<(), crate::store::StoreError> {
        self.validate()?;
        let IntakeAcceptanceOutcome::Accepted {
            case_disposition:
                IntakeCaseDisposition::CreateGovernedCase {
                    case_ref,
                    definition,
                    ..
                },
        } = outcome
        else {
            return Err(invalid_creation(
                "creation snapshot is not a create-case outcome",
            ));
        };
        let raw_case =
            wos_core::instance::WorkflowProcess::extract_urn_type_id(case_ref).unwrap_or(case_ref);
        if raw_case != self.case_ledger_id
            || definition.definition_url != self.definition_url
            || definition.definition_version != self.definition_version
            || request.governed_case_ref.as_deref().is_some_and(|value| {
                wos_core::instance::WorkflowProcess::extract_urn_type_id(value).unwrap_or(value)
                    != self.case_ledger_id
            })
            || request
                .governed_case_definition
                .as_ref()
                .is_some_and(|value| {
                    value.definition_url != self.definition_url
                        || value.definition_version != self.definition_version
                })
        {
            return Err(invalid_creation(
                "creation snapshot conflicts with the intake association",
            ));
        }
        Ok(())
    }
}

fn invalid_creation(message: &str) -> crate::store::StoreError {
    crate::store::StoreError::InvalidCreationEvidence(message.to_string())
}

/// Final result returned by the runtime intake command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeAcceptanceDecision {
    /// Final host-visible outcome.
    pub outcome: IntakeAcceptanceOutcome,
    /// Provenance emitted by policy and binding finalization.
    pub provenance: Vec<ProvenanceRecord>,
    /// Retained creation facts, never current task state or an access grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation: Option<ProcessCreationSnapshot>,
}

/// Persistence state for a durable intake receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IntakeRecordStatus {
    /// The intake receipt exists, but the command has not completed.
    Pending,
    /// The intake outcome and provenance are finalized, but side effects may still be replaying.
    Prepared,
    /// The intake command completed and the receipt is replayable.
    Applied,
}

impl IntakeAcceptanceDecision {
    /// Build an accepted decision without auxiliary provenance.
    pub fn accepted(case_disposition: IntakeCaseDisposition) -> Self {
        Self {
            outcome: IntakeAcceptanceOutcome::Accepted { case_disposition },
            provenance: Vec::new(),
            creation: None,
        }
    }

    /// Build a rejected decision without auxiliary provenance.
    pub fn rejected(code: impl Into<String>) -> Self {
        Self {
            outcome: IntakeAcceptanceOutcome::Rejected { code: code.into() },
            provenance: Vec::new(),
            creation: None,
        }
    }

    /// Build a deferred decision without auxiliary provenance.
    pub fn deferred(code: impl Into<String>) -> Self {
        Self {
            outcome: IntakeAcceptanceOutcome::Deferred { code: code.into() },
            provenance: Vec::new(),
            creation: None,
        }
    }
}

fn create_disposition_from_request(
    request: &IntakeAcceptanceRequest,
) -> Result<IntakeCaseDisposition, RuntimeError> {
    let case_ref = request.governed_case_ref.clone().ok_or_else(|| {
        RuntimeError::MissingMetadata(
            "governedCaseRef required for accepted public intake".to_string(),
        )
    })?;
    let definition = request.governed_case_definition.clone().ok_or_else(|| {
        RuntimeError::MissingMetadata(
            "governedCaseDefinition required for accepted public intake".to_string(),
        )
    })?;
    Ok(IntakeCaseDisposition::CreateGovernedCase {
        case_ref,
        definition,
        initial_case_state: request.initial_case_state.clone(),
    })
}

/// Binding-owned intake-interpretation adapter.
pub trait IntakeAcceptanceAdapter: Send + Sync {
    /// Binding discriminator handled by this adapter.
    fn binding(&self) -> &'static str;

    /// Parse, validate, and interpret a binding-native intake handoff.
    fn interpret_intake_handoff(
        &self,
        request: &IntakeAcceptanceRequest,
    ) -> Result<IntakeInterpretation, BindingError>;

    /// Emit binding-owned provenance for the final acceptance decision.
    fn finalize_intake_acceptance(
        &self,
        request: &IntakeAcceptanceRequest,
        outcome: &IntakeAcceptanceOutcome,
    ) -> Result<Vec<ProvenanceRecord>, BindingError>;
}

/// Host-owned intake-acceptance policy.
pub trait IntakeAcceptancePolicy: Send + Sync {
    /// Apply host policy to a binding-owned intake interpretation.
    fn evaluate_intake_acceptance(
        &self,
        context: &IntakePolicyContext,
    ) -> Result<IntakeAcceptanceDecision, RuntimeError>;
}

/// Default host policy: accept the interpreted handoff as-is.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopIntakeAcceptancePolicy;

impl IntakeAcceptancePolicy for NoopIntakeAcceptancePolicy {
    fn evaluate_intake_acceptance(
        &self,
        context: &IntakePolicyContext,
    ) -> Result<IntakeAcceptanceDecision, RuntimeError> {
        match &context.interpretation.case_intent {
            IntakeCaseIntent::AttachToExistingCase { case_ref } => Ok(
                IntakeAcceptanceDecision::accepted(IntakeCaseDisposition::AttachToExistingCase {
                    case_ref: case_ref.clone(),
                }),
            ),
            IntakeCaseIntent::RequestGovernedCaseCreation => {
                Ok(IntakeAcceptanceDecision::accepted(
                    create_disposition_from_request(&context.request)?,
                ))
            }
        }
    }
}

/// Default policy that explicitly enables public-intake auto-creation.
#[derive(Debug, Clone, Copy, Default)]
pub struct AutoCreatePublicIntakePolicy;

impl IntakeAcceptancePolicy for AutoCreatePublicIntakePolicy {
    fn evaluate_intake_acceptance(
        &self,
        context: &IntakePolicyContext,
    ) -> Result<IntakeAcceptanceDecision, RuntimeError> {
        NoopIntakeAcceptancePolicy.evaluate_intake_acceptance(context)
    }
}

/// Default policy that blocks public-intake creation while allowing workflow attachments.
#[derive(Debug, Clone, Copy, Default)]
pub struct PublicIntakeDisabledPolicy;

impl IntakeAcceptancePolicy for PublicIntakeDisabledPolicy {
    fn evaluate_intake_acceptance(
        &self,
        context: &IntakePolicyContext,
    ) -> Result<IntakeAcceptanceDecision, RuntimeError> {
        match &context.interpretation.case_intent {
            IntakeCaseIntent::AttachToExistingCase { case_ref } => Ok(
                IntakeAcceptanceDecision::accepted(IntakeCaseDisposition::AttachToExistingCase {
                    case_ref: case_ref.clone(),
                }),
            ),
            IntakeCaseIntent::RequestGovernedCaseCreation => {
                Ok(IntakeAcceptanceDecision::rejected("publicIntakeDisabled"))
            }
        }
    }
}

/// Default policy that defers public-intake creation to manual review.
#[derive(Debug, Clone, Copy, Default)]
pub struct ManualReviewIntakePolicy;

impl IntakeAcceptancePolicy for ManualReviewIntakePolicy {
    fn evaluate_intake_acceptance(
        &self,
        context: &IntakePolicyContext,
    ) -> Result<IntakeAcceptanceDecision, RuntimeError> {
        match &context.interpretation.case_intent {
            IntakeCaseIntent::AttachToExistingCase { case_ref } => Ok(
                IntakeAcceptanceDecision::accepted(IntakeCaseDisposition::AttachToExistingCase {
                    case_ref: case_ref.clone(),
                }),
            ),
            IntakeCaseIntent::RequestGovernedCaseCreation => {
                Ok(IntakeAcceptanceDecision::deferred("manualReviewRequired"))
            }
        }
    }
}

/// Registry of available intake-acceptance adapters.
#[derive(Clone, Default)]
pub struct IntakeAcceptanceRegistry {
    adapters: HashMap<String, Arc<dyn IntakeAcceptanceAdapter>>,
}

impl IntakeAcceptanceRegistry {
    /// Create an empty intake-acceptance registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an intake-acceptance adapter by its binding discriminator.
    pub fn register<A>(&mut self, adapter: A)
    where
        A: IntakeAcceptanceAdapter + 'static,
    {
        self.adapters
            .insert(adapter.binding().to_string(), Arc::new(adapter));
    }

    /// Resolve an intake-acceptance adapter for the requested binding.
    pub fn get(&self, binding: &str) -> Option<Arc<dyn IntakeAcceptanceAdapter>> {
        self.adapters.get(binding).cloned()
    }
}
