// Rust guideline compliant 2026-02-21
//! Derives causal explanation only from actual native table evaluations.
use super::admission::*;
use super::*;
use serde_json::Value;
use wos_core::eval::{Evaluator, GuardEvaluation};
use wos_core::model::due_process::*;
use wos_core::{ProvenanceKind, ProvenanceRecord};
use wos_events::provenance::CaseFileSnapshot;

fn observation(guard: &GuardEvaluation) -> Result<NativeDecisionEvaluation, RefusalCode> {
    let table = guard
        .decision_table
        .as_ref()
        .ok_or(RefusalCode::ExplanationRejected)?;
    let mut inputs = Vec::new();
    fn flatten(value: &Value, prefix: &str, out: &mut Vec<NamedScalar>) -> Result<(), RefusalCode> {
        if let Some(obj) = value.as_object() {
            for (k, v) in obj {
                flatten(
                    v,
                    &if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{prefix}.{k}")
                    },
                    out,
                )?;
            }
        } else {
            out.push(NamedScalar {
                name: prefix.into(),
                value: Scalar::from_value(value).ok_or(RefusalCode::ExplanationRejected)?,
            });
        }
        Ok(())
    }
    flatten(&guard.inputs, "", &mut inputs)?;
    // Named object inputs are unordered evidence, unlike matched table rows.
    inputs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(NativeDecisionEvaluation {
        location: guard
            .location
            .clone()
            .ok_or(RefusalCode::ExplanationRejected)?,
        table_ref: table.table_ref.clone(),
        inputs,
        matched_row_ids: table.matched_row_ids.clone(),
        selected_row_id: table.selected_row_id.clone(),
        outputs: vec![NamedScalar {
            name: table.output_column.clone(),
            value: Scalar::from_value(
                table
                    .output
                    .as_ref()
                    .ok_or(RefusalCode::ExplanationRejected)?,
            )
            .ok_or(RefusalCode::ExplanationRejected)?,
        }],
        guard_result: guard.result,
    })
}
pub(super) fn records(
    minter: &stack_common_typeid::TypeIdMinter,
    kernel: &KernelDocument,
    binding: &DeterminationBinding,
    determination: &DeterminationState,
    snapshot: &Value,
    guard: &GuardEvaluation,
    now: &str,
) -> Result<Vec<ProvenanceRecord>, RefusalCode> {
    let original = observation(guard)?;
    let policy = &binding.explanation;
    if !original.guard_result
        || original.table_ref != policy.table_ref
        || original.outputs.len() != 1
        || original.outputs[0].name != policy.output_column
        || original.outputs[0].value == policy.beneficial_output
        || original.matched_row_ids.is_empty()
        || original.selected_row_id.is_none()
        || guard.firing_record_id.as_deref() != Some(determination.id.as_str())
    {
        return Err(RefusalCode::ExplanationRejected);
    }
    let replay = Evaluator::evaluate_reference_alternative(
        kernel,
        minter,
        &binding.transition,
        snapshot,
        timestamp(&determination.determined_at)?,
    )
    .map_err(|_| RefusalCode::ExplanationRejected)?;
    if observation(&replay)? != original {
        return Err(RefusalCode::ExplanationRejected);
    }
    let reasons: Vec<_> = policy
        .reasons
        .iter()
        .filter(|r| Some(&r.row_id) == original.selected_row_id.as_ref())
        .collect();
    if reasons.len() != 1 {
        return Err(RefusalCode::ExplanationRejected);
    }
    let reason = reasons[0];
    let mut record = ProvenanceRecord::blank(minter, ProvenanceKind::DeterminationReasoning);
    record.actor_id = Some(determination.determining_human.clone());
    record.timestamp = now.into();
    record.data = Some(
        serde_json::to_value(DeterminationReasoningData {
            tier: ReasoningTier::Reasoning,
            related_transition: determination.id.clone(),
            rule_id: reason.reason_code.clone(),
            authority: reason.source_authority.clone(),
            description: reason.description.clone(),
            timestamp: now.into(),
            evaluation: original.clone(),
            source_refs: reason.source_refs.clone(),
        })
        .map_err(|_| RefusalCode::ExplanationRejected)?,
    );
    let mut result = vec![record];
    for (kind, alternatives) in [
        (CounterfactualKind::Positive, &policy.positive),
        (CounterfactualKind::Negative, &policy.negative),
    ] {
        for alternative in alternatives {
            let mut changed = snapshot.clone();
            for change in &alternative.changes {
                let value = changed
                    .pointer_mut(&change.case_pointer)
                    .ok_or(RefusalCode::ExplanationRejected)?;
                Scalar::from_value(value).ok_or(RefusalCode::ExplanationRejected)?;
                if kind == CounterfactualKind::Negative
                    && digest(value) == digest(&change.value.value())
                {
                    return Err(RefusalCode::ExplanationRejected);
                }
                *value = change.value.value();
            }
            let alternate = Evaluator::evaluate_reference_alternative(
                kernel,
                minter,
                &binding.transition,
                &changed,
                timestamp(&determination.determined_at)?,
            )
            .map_err(|_| RefusalCode::ExplanationRejected)?;
            let alternate = observation(&alternate)?;
            if alternate.matched_row_ids.is_empty()
                || alternate.selected_row_id.is_none()
                || alternate.table_ref != original.table_ref
                || alternate.outputs[0].name != policy.output_column
            {
                return Err(RefusalCode::ExplanationRejected);
            }
            match kind {
                CounterfactualKind::Positive
                    if alternate.outputs[0].value != policy.beneficial_output =>
                {
                    return Err(RefusalCode::ExplanationRejected);
                }
                CounterfactualKind::Negative
                    if alternate.outputs != original.outputs || !alternate.guard_result =>
                {
                    return Err(RefusalCode::ExplanationRejected);
                }
                _ => {}
            }
            let mut record =
                ProvenanceRecord::blank(minter, ProvenanceKind::DeterminationCounterfactual);
            record.actor_id = Some(determination.determining_human.clone());
            record.timestamp = now.into();
            record.data = Some(
                serde_json::to_value(DeterminationCounterfactualData {
                    tier: CounterfactualTier::Counterfactual,
                    related_transition: determination.id.clone(),
                    kind: kind.clone(),
                    factor: alternative.id.clone(),
                    description: alternative.description.clone(),
                    alternative: alternative.clone(),
                    original_snapshot_sha256: determination.snapshot.sha256.clone(),
                    alternate_snapshot_sha256: CaseFileSnapshot::from_case_state(&changed).sha256,
                    original_evaluation: original.clone(),
                    alternate_evaluation: alternate,
                    source_refs: alternative.source_refs.clone(),
                })
                .map_err(|_| RefusalCode::ExplanationRejected)?,
            );
            result.push(record);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn named_input_order_does_not_change_actual_native_evaluation_evidence() {
        let kernel: KernelDocument = serde_json::from_value(json!({"$wosWorkflow":"1.0",
            "decisionTables":[{"id":"eligibility","hitPolicy":"first","inputs":[{"name":"income","type":"number"},{"name":"householdSize","type":"number"}],"outputs":[{"name":"adverse","type":"boolean"}],"rows":[
                {"id":"high","inputCells":["income > 100","householdSize > 0"],"outputCells":["true"]},
                {"id":"low","inputCells":["income <= 100","householdSize > 0"],"outputCells":["false"]}]}],
            "lifecycle":{"initialState":"open","states":{"open":{"type":"atomic","transitions":[{"target":"pending","guard":{"kind":"decisionTable","ref":"eligibility","outputColumn":"adverse","inputBindings":{"income":"caseFile.income","householdSize":"caseFile.householdSize"}}}]},"pending":{"type":"atomic"}}}
        })).unwrap();
        let mut original = Evaluator::evaluate_reference_alternative(
            &kernel,
            &stack_common_typeid::testing::fixture_minter(),
            &TransitionLocation {
                state_pointer: "/lifecycle/states/open".into(),
                transition_index: 0,
            },
            &json!({"income":150,"householdSize":1}),
            0,
        )
        .unwrap();
        let fields = vec![
            ("income".into(), json!(150)),
            ("householdSize".into(), json!(1)),
        ];
        original.inputs = Value::Object(fields.iter().cloned().collect());
        let mut reordered = original.clone();
        reordered.inputs = Value::Object(fields.into_iter().rev().collect());
        assert_ne!(
            original.inputs.to_string(),
            reordered.inputs.to_string(),
            "control must exercise actual insertion-order differences"
        );
        let expected = observation(&original).unwrap();
        assert_eq!(expected, observation(&reordered).unwrap());
        reordered.inputs["income"] = json!(151);
        assert_ne!(
            expected,
            observation(&reordered).unwrap(),
            "different actual scalar evidence must remain different"
        );
        let mut changed_row = original.clone();
        changed_row.decision_table.as_mut().unwrap().selected_row_id = Some("low".into());
        assert_ne!(
            expected,
            observation(&changed_row).unwrap(),
            "selected-row evidence must remain exact"
        );
    }
}
