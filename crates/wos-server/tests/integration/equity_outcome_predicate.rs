//! `outcomePredicate` on equity evaluate is rejected until implemented.
//! Equity aggregation walks every matching instance via
//! `list_instances_all_pages`, which relies on the port-wide
//! `LIST_INSTANCES_PAGE_SIZE_MAX` bound.

use std::sync::Arc;

use chrono::Utc;
use wos_server::error::ApiError;
use wos_server::services::advanced_service::{EquityEvaluateRequest, evaluate_equity};
use wos_server::storage::{InstanceRow, SqliteStorage, StorageHandle};

async fn empty_store() -> StorageHandle {
    let store = SqliteStorage::connect("sqlite::memory:?cache=shared")
        .await
        .expect("connect");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

#[tokio::test]
async fn equity_rejects_outcome_predicate_with_bad_request() {
    let storage = empty_store().await;
    let req = EquityEvaluateRequest {
        workflow_url: "urn:wos:workflow:any".into(),
        group_by_path: "applicant.region".into(),
        outcome_predicate: Some("status == \"won\"".into()),
    };
    let err = evaluate_equity(&storage, &req)
        .await
        .expect_err("predicate must be rejected");
    match err {
        ApiError::BadRequest(msg) => {
            assert!(msg.contains("outcomePredicate"), "{msg}");
        }
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[tokio::test]
async fn equity_evaluate_counts_all_matching_instances_across_pages() {
    let storage = empty_store().await;
    let wf = "urn:wos:workflow:equity-page-test";
    let now = Utc::now();
    for i in 0..250 {
        storage
            .create_instance(&InstanceRow {
                instance_id: format!("eq-page-{i}"),
                definition_url: wf.into(),
                definition_version: "1".into(),
                status: "completed".into(),
                impact_level: "operational".into(),
                instance_json: serde_json::json!({
                    "configuration": ["intake"],
                    "caseState": { "cohort": "A" }
                }),
                runtime_aux_json: serde_json::json!({}),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
    }

    let report = evaluate_equity(
        &storage,
        &EquityEvaluateRequest {
            workflow_url: wf.into(),
            group_by_path: "cohort".into(),
            outcome_predicate: None,
        },
    )
    .await
    .expect("equity over 250 rows");

    let g = report
        .groups
        .iter()
        .find(|g| g.group == "A")
        .expect("single cohort");
    assert_eq!(g.total, 250);
    assert_eq!(g.positive, 250);
}
