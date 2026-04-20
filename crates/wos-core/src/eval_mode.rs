// Rust guideline compliant 2026-04-11

//! Continuous evaluation mode (Runtime Companion S10).
//!
//! In `event-driven` mode (default), transition guards are evaluated only
//! when an explicit event arrives. In `continuous` mode, after any case
//! state mutation the processor re-evaluates all guards in the current
//! configuration. If a previously-false guard becomes true, the
//! corresponding transition fires.
//!
//! A convergence cap of 100 re-evaluation cycles prevents infinite loops.
//! When the cap is reached, the processor halts re-evaluation and records
//! a `ProvenanceKind::ConvergenceCapReached` record carrying the kernel
//! `$defs/ProvenanceOutcome` reserved literal `"convergenceCapReached"` on
//! its `outcome` field.

use crate::eval::Evaluator;
use crate::model::kernel::EvaluationMode;
use crate::provenance::{ProvenanceKind, ProvenanceRecord};

/// Maximum re-evaluation cycles per triggering mutation (Runtime S10.3).
pub const CONVERGENCE_CAP: u32 = 100;

/// Result of a continuous-mode re-evaluation pass.
#[derive(Debug, Clone)]
pub struct ContinuousEvalResult {
    /// Number of transitions fired during re-evaluation.
    pub transitions_fired: u32,

    /// Number of re-evaluation cycles consumed.
    pub cycles_used: u32,

    /// Whether the convergence cap was reached.
    pub convergence_cap_reached: bool,
}

/// Perform continuous-mode re-evaluation on the evaluator.
///
/// After a case state mutation, this function re-evaluates all guards
/// in the current configuration. Each time a previously-false guard
/// becomes true, the corresponding eventless transition fires and the
/// cycle count increments. Re-evaluation stops when either:
///
/// - No new transition fires (stable configuration reached), or
/// - The convergence cap (100 cycles) is reached.
///
/// The `triggering_mutation` string is recorded in provenance if the
/// cap is reached. It should describe what caused the re-evaluation
/// (e.g., a `setData` path or timer event name).
///
/// Returns `Ok(result)` with cycle statistics, or an evaluation error.
///
/// # Spec Reference
///
/// Runtime Companion S10.3: "after any case state mutation [...] the
/// processor re-evaluates all guards in the current configuration."
pub fn continuous_reevaluate(
    evaluator: &mut Evaluator,
    triggering_mutation: &str,
) -> Result<ContinuousEvalResult, crate::eval::EvalError> {
    let mode = evaluator
        .kernel()
        .evaluation_mode
        .unwrap_or(EvaluationMode::EventDriven);

    if mode != EvaluationMode::Continuous {
        return Ok(ContinuousEvalResult {
            transitions_fired: 0,
            cycles_used: 0,
            convergence_cap_reached: false,
        });
    }

    let mut total_fired: u32 = 0;
    let mut cycles: u32 = 0;

    loop {
        if cycles >= CONVERGENCE_CAP {
            // Record convergence cap provenance (Runtime §10.3).
            //
            // Emission shape completes ADR 0059 Task 3 (§4.3b #F5c / F3b Task 3):
            //   - `record_kind` = `ConvergenceCapReached` (dedicated variant,
            //     not the prior `CaseStateMutation` overload),
            //   - `outcome` = `"convergenceCapReached"` (kernel `$defs/ProvenanceOutcome`
            //     reserved literal), and
            //   - `data` carries the triggering mutation + cycles used for
            //     downstream tooling to locate the cycle.
            evaluator.record_provenance(ProvenanceRecord {
                record_kind: ProvenanceKind::ConvergenceCapReached,
                timestamp: String::new(),
                actor_id: None,
                from_state: None,
                to_state: None,
                event: None,
                data: Some(serde_json::json!({
                    "triggeringMutation": triggering_mutation,
                    "cyclesUsed": cycles,
                })),
                audit_layer: None,
                actor_type: None,
                lifecycle_state: None,
                definition_version: None,
                inputs: Vec::new(),
                outputs: Vec::new(),
                input_digest: None,
                output_digest: None,
                transition_tags: Vec::new(),
                case_file_snapshot: None,
                outcome: Some("convergenceCapReached".to_string()),
            });
            return Ok(ContinuousEvalResult {
                transitions_fired: total_fired,
                cycles_used: cycles,
                convergence_cap_reached: true,
            });
        }

        // Try to fire any eventless (guard-only) transition in the current config.
        let fired = evaluator.try_fire_guardless_transition()?;

        if fired {
            total_fired += 1;
            cycles += 1;
        } else {
            // Stable configuration — no more guards newly satisfied.
            break;
        }
    }

    Ok(ContinuousEvalResult {
        transitions_fired: total_fired,
        cycles_used: cycles,
        convergence_cap_reached: false,
    })
}

/// Check whether the evaluator's kernel document uses continuous mode.
pub fn is_continuous_mode(evaluator: &Evaluator) -> bool {
    evaluator.kernel().evaluation_mode == Some(EvaluationMode::Continuous)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::kernel::*;
    use indexmap::IndexMap;
    use std::collections::HashMap;

    /// Build a minimal state with defaults.
    fn atomic_state(transitions: Vec<Transition>) -> State {
        State {
            kind: StateKind::Atomic,
            description: None,
            tags: Vec::new(),
            on_entry: Vec::new(),
            on_exit: Vec::new(),
            transitions,
            states: IndexMap::new(),
            regions: IndexMap::new(),
            initial_state: None,
            history_state: None,
            cancellation_policy: None,
            extensions: HashMap::new(),
        }
    }

    fn final_state() -> State {
        State {
            kind: StateKind::Final,
            description: None,
            tags: Vec::new(),
            on_entry: Vec::new(),
            on_exit: Vec::new(),
            transitions: Vec::new(),
            states: IndexMap::new(),
            regions: IndexMap::new(),
            initial_state: None,
            history_state: None,
            cancellation_policy: None,
            extensions: HashMap::new(),
        }
    }

    /// Build a minimal kernel with continuous evaluation mode and a guard-based transition.
    fn continuous_kernel() -> KernelDocument {
        let mut states = IndexMap::new();

        states.insert(
            "waiting".to_string(),
            atomic_state(vec![Transition {
                event: "$continuous".to_string(),
                target: "approved".to_string(),
                guard: Some("caseFile.ready == true".to_string()),
                tags: Vec::new(),
                actions: Vec::new(),
                description: None,
            }]),
        );

        states.insert("approved".to_string(), final_state());

        KernelDocument {
            wos_kernel: "1.0".to_string(),
            schema: None,
            url: Some("https://example.org/test/continuous".to_string()),
            version: Some("1.0.0".to_string()),
            title: Some("Continuous Mode Test".to_string()),
            description: None,
            status: None,
            impact_level: Some(ImpactLevel::Operational),
            actors: Vec::new(),
            lifecycle: Lifecycle {
                initial_state: "waiting".to_string(),
                states,
                milestones: HashMap::new(),
            },
            case_file: Some(CaseFile {
                fields: {
                    let mut fields = HashMap::new();
                    fields.insert(
                        "ready".to_string(),
                        FieldDefinition {
                            kind: "boolean".to_string(),
                            description: None,
                            default: Some(serde_json::Value::Bool(false)),
                        },
                    );
                    fields
                },
                relationships: Vec::new(),
            }),
            contracts: HashMap::new(),
            provenance: None,
            execution: None,
            evaluation_mode: Some(EvaluationMode::Continuous),
            max_relationship_event_depth: None,
            extensions: HashMap::new(),
        }
    }

    #[test]
    fn continuous_mode_fires_when_guard_satisfied() {
        let kernel = continuous_kernel();
        let mut evaluator = Evaluator::new(kernel).unwrap();

        // Initially, ready=false so continuous re-eval should find nothing.
        let result = continuous_reevaluate(&mut evaluator, "test-init").unwrap();
        assert_eq!(result.transitions_fired, 0);
        assert!(!result.convergence_cap_reached);

        // Mutate case state so guard becomes true.
        evaluator
            .case_state_mut()
            .insert("ready".to_string(), serde_json::Value::Bool(true));

        // Now continuous re-eval should fire the transition.
        let result = continuous_reevaluate(&mut evaluator, "setData:ready").unwrap();
        assert_eq!(result.transitions_fired, 1);
        assert!(!result.convergence_cap_reached);
        assert!(evaluator.configuration().contains("approved"));
    }

    #[test]
    fn event_driven_mode_skips_reevaluation() {
        let mut kernel = continuous_kernel();
        kernel.evaluation_mode = Some(EvaluationMode::EventDriven);

        let mut evaluator = Evaluator::new(kernel).unwrap();
        evaluator
            .case_state_mut()
            .insert("ready".to_string(), serde_json::Value::Bool(true));

        let result = continuous_reevaluate(&mut evaluator, "setData:ready").unwrap();
        assert_eq!(result.transitions_fired, 0);
        assert!(!result.convergence_cap_reached);
        // Should NOT have fired — still in "waiting".
        assert!(evaluator.configuration().contains("waiting"));
    }

    #[test]
    fn convergence_cap_halts_infinite_loop() {
        // Build a kernel where a guardless $continuous transition always fires,
        // ping-ponging between two states until the cap is reached.
        let mut states = IndexMap::new();

        states.insert(
            "loop".to_string(),
            atomic_state(vec![Transition {
                event: "$continuous".to_string(),
                target: "loop2".to_string(),
                guard: None, // Always true.
                tags: Vec::new(),
                actions: Vec::new(),
                description: None,
            }]),
        );

        states.insert(
            "loop2".to_string(),
            atomic_state(vec![Transition {
                event: "$continuous".to_string(),
                target: "loop".to_string(),
                guard: None,
                tags: Vec::new(),
                actions: Vec::new(),
                description: None,
            }]),
        );

        let kernel = KernelDocument {
            wos_kernel: "1.0".to_string(),
            schema: None,
            url: Some("https://example.org/test/loop".to_string()),
            version: Some("1.0.0".to_string()),
            title: Some("Loop Test".to_string()),
            description: None,
            status: None,
            impact_level: Some(ImpactLevel::Operational),
            actors: Vec::new(),
            lifecycle: Lifecycle {
                initial_state: "loop".to_string(),
                states,
                milestones: HashMap::new(),
            },
            case_file: None,
            contracts: HashMap::new(),
            provenance: None,
            execution: None,
            evaluation_mode: Some(EvaluationMode::Continuous),
            max_relationship_event_depth: None,
            extensions: HashMap::new(),
        };

        let mut evaluator = Evaluator::new(kernel).unwrap();
        let result = continuous_reevaluate(&mut evaluator, "loop-trigger").unwrap();

        assert!(result.convergence_cap_reached);
        assert_eq!(result.cycles_used, CONVERGENCE_CAP);
        assert_eq!(result.transitions_fired, CONVERGENCE_CAP);
    }

    /// §4.3b #F5c / ADR 0059 Task 3: the cap-hit provenance record now
    /// emits as a dedicated `ConvergenceCapReached` kind carrying
    /// `outcome: "convergenceCapReached"`, not the prior
    /// `CaseStateMutation` overload with the flag hidden in `data`.
    #[test]
    fn convergence_cap_emits_dedicated_kind_and_outcome_field() {
        let mut states = IndexMap::new();

        states.insert(
            "loop".to_string(),
            atomic_state(vec![Transition {
                event: "$continuous".to_string(),
                target: "loop2".to_string(),
                guard: None,
                tags: Vec::new(),
                actions: Vec::new(),
                description: None,
            }]),
        );
        states.insert(
            "loop2".to_string(),
            atomic_state(vec![Transition {
                event: "$continuous".to_string(),
                target: "loop".to_string(),
                guard: None,
                tags: Vec::new(),
                actions: Vec::new(),
                description: None,
            }]),
        );

        let kernel = KernelDocument {
            wos_kernel: "1.0".to_string(),
            schema: None,
            url: Some("https://example.org/test/cap-shape".to_string()),
            version: Some("1.0.0".to_string()),
            title: Some("Cap Shape Test".to_string()),
            description: None,
            status: None,
            impact_level: Some(ImpactLevel::Operational),
            actors: Vec::new(),
            lifecycle: Lifecycle {
                initial_state: "loop".to_string(),
                states,
                milestones: HashMap::new(),
            },
            case_file: None,
            contracts: HashMap::new(),
            provenance: None,
            execution: None,
            evaluation_mode: Some(EvaluationMode::Continuous),
            max_relationship_event_depth: None,
            extensions: HashMap::new(),
        };

        let mut evaluator = Evaluator::new(kernel).unwrap();
        continuous_reevaluate(&mut evaluator, "loop-trigger").unwrap();

        let cap_record = evaluator
            .provenance()
            .records()
            .iter()
            .find(|record| record.record_kind == ProvenanceKind::ConvergenceCapReached)
            .expect("ConvergenceCapReached record must be emitted on cap hit");
        assert_eq!(
            cap_record.outcome.as_deref(),
            Some("convergenceCapReached"),
            "the dedicated ConvergenceCapReached record must carry \
             outcome = \"convergenceCapReached\" so kernel \
             $defs/ProvenanceOutcome validation and downstream tooling \
             can distinguish the cap-hit from an unrelated case-state \
             mutation",
        );
        let data = cap_record.data.as_ref().expect("cap record has data payload");
        assert_eq!(
            data.get("triggeringMutation").and_then(|v| v.as_str()),
            Some("loop-trigger"),
        );
        assert_eq!(
            data.get("cyclesUsed").and_then(|v| v.as_u64()),
            Some(CONVERGENCE_CAP as u64),
        );
        assert!(
            data.get("convergenceCapReached").is_none(),
            "the flag moved from `data.convergenceCapReached` to the \
             first-class `outcome` field; carrying both would invite \
             drift between the two signals",
        );
    }
}
