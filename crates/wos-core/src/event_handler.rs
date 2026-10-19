// Rust guideline compliant 2026-02-21

//! Refusal checks for the explicitly scoped supplied-output AI exercise.
//!
//! Ordinary business events never manufacture operation provenance. Native
//! task, service, validation and migration owners emit their own records.

/// Identify event hints that cannot establish trusted AI operations or facts.
///
/// These requests require an owning operation. Ordinary payloads never prove
/// invocation, contract validation, fallback, model resolution or state changes.
pub fn unsupported_agent_hint(data: &serde_json::Value) -> Option<&'static str> {
    [
        "actorTypeOverride",
        "driftAlert",
        "narrativeTier",
        "invocationSource",
        "modelVersionAtInvocation",
        "expectedModelVersion",
        "agentStateChange",
        "constraintModification",
        "determinationBasis",
        "contractValidation",
        "error",
        "decayEvent",
        "cumulativeConfidence",
        "groundTruth",
        "escalationActive",
        "agentState",
        "configMaxAutonomy",
        "dynamicAutonomy",
        "effectiveAutonomy",
        "requestedAutonomy",
        "overridePriorDecision",
    ]
    .into_iter()
    .find(|field| data.get(*field).is_some())
}
