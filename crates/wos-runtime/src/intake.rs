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
use wos_core::provenance::ProvenanceRecord;

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

/// Final result returned by the runtime intake command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeAcceptanceDecision {
    /// Final host-visible outcome.
    pub outcome: IntakeAcceptanceOutcome,
    /// Provenance emitted by policy and binding finalization.
    pub provenance: Vec<ProvenanceRecord>,
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
        }
    }

    /// Build a rejected decision without auxiliary provenance.
    pub fn rejected(code: impl Into<String>) -> Self {
        Self {
            outcome: IntakeAcceptanceOutcome::Rejected { code: code.into() },
            provenance: Vec::new(),
        }
    }

    /// Build a deferred decision without auxiliary provenance.
    pub fn deferred(code: impl Into<String>) -> Self {
        Self {
            outcome: IntakeAcceptanceOutcome::Deferred { code: code.into() },
            provenance: Vec::new(),
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
