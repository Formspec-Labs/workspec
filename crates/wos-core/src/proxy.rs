// Rust guideline compliant 2026-02-21

//! Honest policy comparison evidence without inventing proxy execution.
//!
//! The authored proxy configuration is reserved. A local evaluator can observe
//! native policy but cannot establish an Assist transport/provider invocation.

use crate::ai_policy::{NativeAiPolicy, PolicyExerciseContext};
use serde::{Deserialize, Serialize};

/// Evidence distinguishes local policy comparison from actual proxy execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistGovernanceProxyEvidence {
    /// Actual proxy execution must be observed for this to pass.
    pub differential_check_passed: bool,
    /// The native calculation itself is unchanged.
    pub strictness_preserved: bool,
    /// Actual proxy provenance was not observed by a policy-only exercise.
    pub provenance_preserved: bool,
}
/// Observe local policy without claiming an unexecuted Assist proxy operation.
pub fn observe_assist_governance_proxy(
    policy: &NativeAiPolicy,
    context: &PolicyExerciseContext<'_>,
) -> AssistGovernanceProxyEvidence {
    let observed = crate::deontic::evaluate_native_policy(policy, context);
    AssistGovernanceProxyEvidence {
        differential_check_passed: false,
        strictness_preserved: observed.errors.is_empty() && observed.refusals.is_empty(),
        provenance_preserved: false,
    }
}
