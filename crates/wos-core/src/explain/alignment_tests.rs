use super::*;
use serde_json::{Value, json};
use wos_events::ProvenanceKind;

fn record(kind: ProvenanceKind, data: Value) -> ProvenanceRecord {
    let mut record = ProvenanceRecord::state_entered("same-case");
    record.record_kind = kind;
    record.timestamp = "2026-10-20T14:00:00Z".into();
    record.data = Some(data);
    record
}

#[test]
fn explanation_alignment_keeps_typed_human_and_reference_facts_distinct() {
    // Shape fixtures exercise the two existing association formats. They do not
    // claim runtime evaluation or authorization of the saved references.
    let evaluation = json!({"location":{"statePointer":"/lifecycle/states/review","transitionIndex":0},
        "tableRef":"eligibility","inputs":[],"matchedRowIds":["denial"],"selectedRowId":"denial",
        "outputs":[],"guardResult":true});
    let source =
        json!({"sourceVersion":"rule1","sectionAnchor":"section2","retainedSha256":"a".repeat(64)});
    let reference = json!({"tier":"reasoning","relatedTransition":"reference-facts","ruleId":"evaluated-rule",
        "authority":"statute","description":"Retained selected row failed eligibility.","timestamp":"2026-10-20T13:00:00Z",
        "evaluation":evaluation,"sourceRefs":[source]});
    let typed: crate::model::due_process::DeterminationReasoningData =
        serde_json::from_value(reference.clone()).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), reference);
    let counter = json!({"tier":"counterfactual","relatedTransition":"reference-facts","type":"positive",
        "factor":"income","description":"The alternate retained evaluation would qualify.",
        "alternative":{"id":"lower-income","changes":[{"casePointer":"/income","value":{"kind":"number","value":100}}],
            "description":"Lower income","sourceRefs":[source]},
        "originalSnapshotSha256":"a".repeat(64),"alternateSnapshotSha256":"b".repeat(64),
        "originalEvaluation":evaluation,"alternateEvaluation":evaluation,"sourceRefs":[source]});
    let typed: crate::model::due_process::DeterminationCounterfactualData =
        serde_json::from_value(counter.clone()).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), counter);
    let human = json!({"factsRecordRef":"human-facts","snapshotSha256":"c".repeat(64),
        "sourceRef":"accepted-response","reason":"This retained photo obscures the applicant's face.",
        "rulesApplied":[{"ruleId":"human-policy","sourceAuthority":"policy"},
            {"ruleId":"human-statute","sourceAuthority":"statute"}],
        "evidenceConsulted":[{"kind":"caseField","caseFieldPath":"/photo"}],
        "criteriaChecked":[{"criterion":"faceVisible","satisfied":false}]});
    let human_counter = json!({"factsRecordRef":"human-facts","snapshotSha256":"c".repeat(64),"sourceRef":"accepted-response",
        "positiveCounterfactuals":[{"polarity":"positive","factor":"photo","effect":"A clear face photo would permit reconsideration."}],
        "negativeCounterfactuals":[{"polarity":"negative","factor":"sex","effect":"Sex did not enter this judgment."}]});
    let records = vec![
        record(ProvenanceKind::DeterminationReasoning, reference),
        record(ProvenanceKind::ReasoningTierRecorded, human),
        record(ProvenanceKind::DeterminationCounterfactual, counter),
        record(ProvenanceKind::CounterfactualTierRecorded, human_counter),
    ];
    let before = serde_json::to_value(&records).unwrap();
    let reference = assemble_explanation(&records, "reference-facts", &[], "2026-10-20T15:00:00Z");
    assert_eq!(reference.reasoning.len(), 1);
    assert_eq!(reference.reasoning[0].rule_id, "evaluated-rule");
    assert_eq!(reference.positive_counterfactual[0].factor, "income");
    assert!(reference.negative_counterfactual.is_empty());
    let human = assemble_explanation(&records, "human-facts", &[], "2026-10-20T15:00:00Z");
    assert_eq!(
        human
            .reasoning
            .iter()
            .map(|r| r.rule_id.as_str())
            .collect::<Vec<_>>(),
        ["human-statute", "human-policy"]
    );
    assert_eq!(human.positive_counterfactual[0].factor, "photo");
    assert_eq!(human.negative_counterfactual[0].factor, "sex");
    assert_eq!(serde_json::to_value(&records).unwrap(), before);
    assert!(
        assemble_explanation(&records, "same-case", &[], "now")
            .reasoning
            .is_empty()
    );
}

#[test]
fn explanation_alignment_malformed_human_cannot_fall_through_to_reference() {
    let legacy_lookalike = json!({"tier":"reasoning","relatedTransition":"facts","ruleId":"forged","description":"must not surface"});
    let records = [record(
        ProvenanceKind::ReasoningTierRecorded,
        legacy_lookalike,
    )];
    assert!(
        assemble_explanation(&records, "facts", &[], "now")
            .reasoning
            .is_empty()
    );
    for polarity in ["unknown", "negative"] {
        let data = json!({"factsRecordRef":"facts","snapshotSha256":"a".repeat(64),"sourceRef":"accepted-response",
            "positiveCounterfactuals":[{"polarity":polarity,"factor":"photo","effect":"Wrong positive polarity"}],
            "negativeCounterfactuals":[]});
        let output = assemble_explanation(
            &[record(ProvenanceKind::CounterfactualTierRecorded, data)],
            "facts",
            &[],
            "now",
        );
        assert!(
            output.positive_counterfactual.is_empty() && output.negative_counterfactual.is_empty()
        );
    }
}
