use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;
use tower::ServiceExt;
use wos_server::http;

use crate::common;

const SLUG: &str = "govsurface";
const WORKFLOW_URL: &str = "urn:wos:workflow:govsurface:1.0.0";

fn setup_tempdir() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();

    let kernel = serde_json::json!({
        "$wosWorkflow": "1.0",
        "url": WORKFLOW_URL,
        "version": "1.0.0",
        "title": "Governance coverage fixture",
        "status": "active",
        "impactLevel": "rightsImpacting",
        "actors": [{ "id": "sys", "type": "system" }],
        "lifecycle": {
            "initialState": "done",
            "states": { "done": { "type": "final" } }
        },
        "contracts": {}
    });
    std::fs::create_dir_all(root.join("kernel")).expect("kernel dir");
    std::fs::write(
        root.join("kernel").join(format!("{SLUG}.json")),
        serde_json::to_vec_pretty(&kernel).expect("kernel json"),
    )
    .expect("kernel write");

    let governance = serde_json::json!({
        "qualityControls": {
            "reviewSampling": { "rate": 0.1, "method": "risk", "scope": "all" },
            "separationOfDuties": { "scope": "approval", "excludeRoles": ["Applicant"] },
            "overrideAuthority": {
                "requireStructuredRationale": true,
                "requireAuthorityVerification": true,
                "requireSupportingEvidence": false
            }
        },
        "pipelines": [
            {
                "id": "pipe-1",
                "description": "Eligibility gate",
                "stages": [{ "id": "s1", "type": "assertion", "assertions": [{ "type": "fel", "expression": "true" }] }]
            }
        ]
    });
    std::fs::create_dir_all(root.join("governance")).expect("governance dir");
    std::fs::write(
        root.join("governance").join(format!("{SLUG}.json")),
        serde_json::to_vec_pretty(&governance).expect("governance json"),
    )
    .expect("governance write");

    let equity = serde_json::json!({
        "protectedCategories": [{ "id": "zip", "groupByPath": "applicant.zip", "groups": ["A", "B"] }],
        "disparityMethods": [{ "id": "m1", "method": "ratio" }],
        "reportingSchedule": { "frequency": "monthly", "recipientRoles": ["Supervisor"] },
        "remediationTriggers": [{ "condition": "ratio < 0.8", "action": "review", "notifyRoles": ["Supervisor"] }]
    });
    std::fs::create_dir_all(root.join("equity")).expect("equity dir");
    std::fs::write(
        root.join("equity").join(format!("{SLUG}.json")),
        serde_json::to_vec_pretty(&equity).expect("equity json"),
    )
    .expect("equity write");

    let policy_parameters = serde_json::json!({
        "versions": [
            {
                "id": "v1",
                "label": "Initial",
                "effectiveDate": "2025-01-01T00:00:00Z",
                "parameters": { "maxAppealDays": 30 }
            }
        ]
    });
    std::fs::create_dir_all(root.join("policy-parameters")).expect("policy dir");
    std::fs::write(
        root.join("policy-parameters").join(format!("{SLUG}.json")),
        serde_json::to_vec_pretty(&policy_parameters).expect("policy json"),
    )
    .expect("policy write");

    let calendar = serde_json::json!({
        "events": [
            { "id": "ev-1", "name": "Agency holiday", "date": "2026-01-02", "type": "agency", "impactsDeadlines": true }
        ]
    });
    std::fs::create_dir_all(root.join("business-calendar")).expect("calendar dir");
    std::fs::write(
        root.join("business-calendar").join(format!("{SLUG}.json")),
        serde_json::to_vec_pretty(&calendar).expect("calendar json"),
    )
    .expect("calendar write");

    dir
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn governance_read_routes_and_delegation_lifecycle() {
    let tmp = setup_tempdir();
    let state = common::bring_up_with_cfg(common::stub_config(tmp.path().to_path_buf())).await;
    let app = http::router(state);
    let enc = common::path_encode(WORKFLOW_URL);

    for path in [
        format!("/api/governance/{enc}/quality-controls"),
        format!("/api/governance/{enc}/pipelines"),
        format!("/api/governance/{enc}/equity-config"),
        format!("/api/governance/{enc}/policy-versions"),
        format!("/api/governance/{enc}/calendar-events"),
    ] {
        let res = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert_eq!(res.status(), StatusCode::OK);
    }

    let create_body = serde_json::json!({
        "id": "deleg-1",
        "delegator": "urn:wos:actor:supervisor",
        "delegate": "urn:wos:actor:adjudicator",
        "scope": "appeals",
        "authority": "statute-123",
        "legalInstrument": "memo-1",
        "startDate": "2026-01-01T00:00:00Z",
        "endDate": "2026-12-31T00:00:00Z",
        "status": "active"
    });

    let unauth = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/governance/{enc}/delegations"))
                .header("content-type", "application/json")
                .body(Body::from(create_body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/governance/{enc}/delegations"))
                .header("authorization", "Bearer mock-access")
                .header("content-type", "application/json")
                .body(Body::from(create_body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(create.status(), StatusCode::OK);
    let create_bytes = axum::body::to_bytes(create.into_body(), 8192)
        .await
        .expect("create body");
    let create_json: serde_json::Value = serde_json::from_slice(&create_bytes).expect("create json");
    assert_eq!(create_json["ok"], true);
    assert_eq!(create_json["id"], "deleg-1");

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/governance/{enc}/delegations"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(list.status(), StatusCode::OK);
    let list_bytes = axum::body::to_bytes(list.into_body(), 8192)
        .await
        .expect("list body");
    let list_json: serde_json::Value = serde_json::from_slice(&list_bytes).expect("list json");
    assert_eq!(list_json.as_array().expect("array").len(), 1);
    assert_eq!(list_json[0]["id"], "deleg-1");

    let revoke = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/governance/{enc}/delegations/deleg-1"))
                .header("authorization", "Bearer mock-access")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(revoke.status(), StatusCode::OK);
}
