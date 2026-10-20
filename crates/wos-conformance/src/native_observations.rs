// Rust guideline compliant 2026-02-21
//! Resolves closed selectors into actual retained native replay observations.
use crate::studio_api::{ObservationDifference, ObservationExpectation, ReplayResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wos_core::model::due_process::TransitionLocation;

/// One exact native replay attempt and subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeSelector {
    /// Native event/kind/occurrence identity.
    pub attempt: NativeAttemptSelector,
    /// Actual saved native subject.
    pub subject: NativeSubject,
}
/// Exact attempt identity, independent of descriptive event labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeAttemptSelector {
    /// Original event index.
    pub event_index: usize,
    /// Actual native attempt category.
    pub kind: NativeAttemptKind,
    /// Zero-based occurrence with that identity.
    pub occurrence: usize,
}
/// Supported native operation categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeAttemptKind {
    Event,
    TaskSubmission,
    GovernanceOperation,
    Clock,
    Migration,
}
impl NativeAttemptKind {
    fn label(&self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::TaskSubmission => "taskSubmission",
            Self::GovernanceOperation => "governanceOperation",
            Self::Clock => "clock",
            Self::Migration => "migration",
        }
    }
}
/// Closed native subject families; assertions do not create these facts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum NativeSubject {
    /// Actual saved determining transition.
    Determination {
        binding_ref: String,
        firing_occurrence: usize,
    },
    /// Actual guard/table evaluation, selected or tested without firing.
    Decision {
        transition: TransitionLocation,
        evaluation_occurrence: usize,
    },
    /// Actual saved notice and delivery evidence.
    Notice {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "present"
        )]
        determination_id: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "present"
        )]
        target: Option<crate::reference_recipes::DeterminationTarget>,
    },
    /// Actual appeal and hearing state.
    Appeal {
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "present"
        )]
        determination_id: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "present"
        )]
        target: Option<crate::reference_recipes::DeterminationTarget>,
    },
    /// Actual committed governed operation.
    Operation {
        operation_ref: String,
        operation_occurrence: usize,
    },
    /// Actual native task and accepted artifact association.
    Task {
        task_ref: String,
        task_occurrence: usize,
    },
    /// Actual saved pending native timer.
    Timer {
        timer_id: String,
        timer_occurrence: usize,
    },
    /// Actual retained override associated with its operation.
    Override {
        operation_ref: String,
        operation_occurrence: usize,
    },
}
/// Resolution points into the existing serialized ReplayResult.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum NativeResolution {
    /// One actual subject resolved.
    Resolved { pointer: String },
    /// Missing or ambiguous native facts refuse.
    Refused { code: &'static str },
}
/// Exact native comparison, retaining the actual difference.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum NativeComparison {
    /// The actual observation equals the independent expectation.
    Pass,
    /// The actual observation differs.
    Fail { difference: ObservationDifference },
    /// Required native facts are absent or ambiguous.
    Refused { code: &'static str },
}

fn attempt_index(result: &ReplayResult, selector: &NativeAttemptSelector) -> Option<usize> {
    result
        .attempts
        .iter()
        .enumerate()
        .filter(|(_, attempt)| {
            attempt.event_index == selector.event_index && attempt.kind == selector.kind.label()
        })
        .nth(selector.occurrence)
        .map(|(index, _)| index)
}
fn index_where(
    values: Option<&Vec<Value>>,
    field: &str,
    expected: &str,
    occurrence: usize,
) -> Option<usize> {
    values?
        .iter()
        .enumerate()
        .filter(|(_, value)| value.get(field).and_then(Value::as_str) == Some(expected))
        .nth(occurrence)
        .map(|(index, _)| index)
}
fn unique_where(values: Option<&Vec<Value>>, field: &str, expected: &str) -> Option<usize> {
    let mut matches = values?
        .iter()
        .enumerate()
        .filter(|(_, value)| value.get(field).and_then(Value::as_str) == Some(expected));
    let (index, _) = matches.next()?;
    matches.next().is_none().then_some(index)
}

/// Resolves exactly one actual subject without interpreting lifecycle behavior.
pub fn resolve_observation(result: &ReplayResult, selector: &NativeSelector) -> NativeResolution {
    let resolve = || -> Option<String> {
        let attempt = attempt_index(result, &selector.attempt)?;
        let document = serde_json::to_value(result).ok()?;
        let base = format!("/attempts/{attempt}/after");
        let snapshot = document.pointer(&base)?;
        if !snapshot.is_object() {
            return None;
        }
        let governance = format!("{base}/governanceState/dueProcess");
        let collection = |name: &str| {
            document
                .pointer(&format!("{governance}/{name}"))
                .and_then(Value::as_array)
        };
        match &selector.subject {
            NativeSubject::Determination {
                binding_ref,
                firing_occurrence,
            } => {
                let index = index_where(
                    collection("determinations"),
                    "bindingRef",
                    binding_ref,
                    *firing_occurrence,
                )?;
                Some(format!("{governance}/determinations/{index}"))
            }
            NativeSubject::Notice {
                determination_id,
                target,
            }
            | NativeSubject::Appeal {
                determination_id,
                target,
            } => {
                let resolved = match (determination_id.as_deref(), target.as_ref()) {
                    (Some(id), None) => id,
                    (None, Some(target)) => {
                        let index = index_where(
                            collection("determinations"),
                            "bindingRef",
                            &target.binding_ref,
                            target.firing_occurrence,
                        )?;
                        collection("determinations")?
                            .get(index)?
                            .get("id")?
                            .as_str()?
                    }
                    _ => return None,
                };
                let name = if matches!(&selector.subject, NativeSubject::Notice { .. }) {
                    "notices"
                } else {
                    "appeals"
                };
                let index = unique_where(collection(name), "determinationId", resolved)?;
                Some(format!("{governance}/{name}/{index}"))
            }
            NativeSubject::Operation {
                operation_ref,
                operation_occurrence,
            }
            | NativeSubject::Override {
                operation_ref,
                operation_occurrence,
            } => {
                let index = index_where(
                    collection("operations"),
                    "operationRef",
                    operation_ref,
                    *operation_occurrence,
                )?;
                if matches!(&selector.subject, NativeSubject::Override { .. }) {
                    let id = collection("operations")?
                        .get(index)?
                        .get("recordId")?
                        .as_str()?;
                    let extension =
                        unique_where(collection("extensions"), "operationRecordId", id)?;
                    Some(format!(
                        "{governance}/extensions/{extension}/overrideRecord"
                    ))
                } else {
                    Some(format!("{governance}/operations/{index}"))
                }
            }
            NativeSubject::Task {
                task_ref,
                task_occurrence,
            } => {
                let index = index_where(
                    snapshot.get("taskHistory").and_then(Value::as_array),
                    "taskRef",
                    task_ref,
                    *task_occurrence,
                )?;
                Some(format!("{base}/taskHistory/{index}"))
            }
            NativeSubject::Timer {
                timer_id,
                timer_occurrence,
            } => {
                let index = index_where(
                    snapshot.get("timerHistory").and_then(Value::as_array),
                    "timerId",
                    timer_id,
                    *timer_occurrence,
                )?;
                Some(format!("{base}/timerHistory/{index}"))
            }
            NativeSubject::Decision {
                transition,
                evaluation_occurrence,
            } => {
                let mut matches = Vec::new();
                for (drain_index, drain) in result.attempts[attempt].drains.iter().enumerate() {
                    for (guard_index, guard) in drain.guard_evaluations.iter().enumerate() {
                        if guard.location.as_ref() == Some(transition) {
                            matches.push(format!("/attempts/{attempt}/drains/{drain_index}/guardEvaluations/{guard_index}"));
                        }
                    }
                }
                matches.get(*evaluation_occurrence).cloned()
            }
        }
    };
    match resolve() {
        Some(pointer) => NativeResolution::Resolved { pointer },
        None => NativeResolution::Refused {
            code: "targetRejected",
        },
    }
}

/// Compares an independent expectation with the resolved actual native subject.
pub fn compare_observation(
    result: &ReplayResult,
    selector: &NativeSelector,
    expected: &ObservationExpectation,
) -> NativeComparison {
    let NativeResolution::Resolved { pointer } = resolve_observation(result, selector) else {
        return NativeComparison::Refused {
            code: "targetRejected",
        };
    };
    if !expected.pointer.is_empty() && !expected.pointer.starts_with('/') {
        return NativeComparison::Refused {
            code: "targetRejected",
        };
    }
    let full = format!("{pointer}{}", expected.pointer);
    let actual = serde_json::to_value(result)
        .ok()
        .and_then(|value| value.pointer(&full).cloned());
    let Some(actual) = actual else {
        return NativeComparison::Refused {
            code: "targetRejected",
        };
    };
    if actual == expected.expected {
        NativeComparison::Pass
    } else {
        NativeComparison::Fail {
            difference: ObservationDifference {
                pointer: full,
                expected: expected.expected.clone(),
                actual: Some(actual),
            },
        }
    }
}

/// Uses the maintained partial matcher on an exact attempt's actual provenance.
pub fn compare_provenance_observation(
    result: &ReplayResult,
    attempt: &NativeAttemptSelector,
    expected: &Value,
) -> NativeComparison {
    let Some(index) = attempt_index(result, attempt) else {
        return NativeComparison::Refused {
            code: "targetRejected",
        };
    };
    let Some(records) = result.attempts[index].provenance.as_ref() else {
        return NativeComparison::Refused {
            code: "targetRejected",
        };
    };
    if records
        .iter()
        .any(|actual| crate::engine::provenance_partial_match(expected, actual))
    {
        NativeComparison::Pass
    } else {
        NativeComparison::Fail {
            difference: ObservationDifference {
                pointer: format!("/attempts/{index}/provenance"),
                expected: expected.clone(),
                actual: serde_json::to_value(records).ok(),
            },
        }
    }
}

fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}
