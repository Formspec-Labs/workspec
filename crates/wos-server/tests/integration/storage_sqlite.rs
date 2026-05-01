//! SQLite storage round-trips: kernels, instances, provenance chain.
//!
//! Uses an in-memory SQLite so tests don't leave database files behind and
//! run concurrently without contention.

use std::sync::Arc;

use chrono::Utc;
use wos_server::storage::{
    self, InboundCloudEventRow, InstanceQuery, InstanceRow, KernelRow, ProvenanceRow,
    SqliteStorage, Storage, StorageHandle, UserRow,
};

async fn fresh() -> SqliteStorage {
    // `:memory:` is unique per connection; wrap with `sqlite::memory:?cache=shared`
    // if we ever need multiple connections pointing at the same DB. The
    // default pool size is >1 so each checkout would get its own schema
    // unless we share. For these tests a cache-shared URL is correct.
    let store = SqliteStorage::connect("sqlite::memory:?cache=shared")
        .await
        .expect("connect in-memory sqlite");
    store.migrate().await.expect("migrate");
    store
}

#[tokio::test]
async fn kernel_roundtrip_upsert_then_list() {
    let store = fresh().await;
    let row = KernelRow {
        url: "urn:wos:workflow:demo:1.0.0".into(),
        title: "Demo".into(),
        version: "1.0.0".into(),
        status: "active".into(),
        impact_level: "operational".into(),
        document: serde_json::json!({"url": "urn:wos:workflow:demo:1.0.0"}),
        updated_at: Utc::now(),
    };
    store.upsert_kernel(&row).await.unwrap();
    let listed = store.list_kernels().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title, "Demo");

    // Upsert overwrites in place (same url).
    let mut v2 = row.clone();
    v2.title = "Demo v2".into();
    store.upsert_kernel(&v2).await.unwrap();
    let listed = store.list_kernels().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title, "Demo v2");
}

#[tokio::test]
async fn instance_pagination_and_filters() {
    let store = fresh().await;
    let now = Utc::now();
    for (i, status, impact) in [
        (1, "active", "operational"),
        (2, "active", "rights-impacting"),
        (3, "completed", "operational"),
        (4, "suspended", "operational"),
    ] {
        store
            .create_instance(&InstanceRow {
                instance_id: format!("inst-{i}"),
                definition_url: "urn:wos:workflow:demo:1".into(),
                definition_version: "1".into(),
                status: status.into(),
                impact_level: impact.into(),
                instance_json: serde_json::json!({ "configuration": ["intake"] }),
                runtime_aux_json: serde_json::json!({}),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
    }
    let page = store
        .list_instances(InstanceQuery {
            page: 1,
            page_size: 2,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(page.total, 4);
    assert_eq!(page.items.len(), 2);

    let active = store
        .list_instances(InstanceQuery {
            status: Some(vec!["active".into()]),
            page: 1,
            page_size: 20,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(active.total, 2);

    let rights = store
        .list_instances(InstanceQuery {
            impact_level: Some(vec!["rights-impacting".into()]),
            page: 1,
            page_size: 20,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(rights.total, 1);
    assert_eq!(rights.items[0].instance_id, "inst-2");

    let empty_filter = store
        .list_instances(InstanceQuery {
            status: Some(vec![]),
            page: 1,
            page_size: 20,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(empty_filter.total, 4);
}

#[tokio::test]
async fn atomic_update_appends_provenance_in_same_txn() {
    let store = fresh().await;
    let now = Utc::now();
    store
        .create_instance(&InstanceRow {
            instance_id: "inst-x".into(),
            definition_url: "u".into(),
            definition_version: "1".into(),
            status: "active".into(),
            impact_level: "operational".into(),
            instance_json: serde_json::json!({"caseState": {}}),
            runtime_aux_json: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        })
        .await
        .unwrap();

    let prov = ProvenanceRow {
        id: "rec-1".into(),
        instance_id: "inst-x".into(),
        seq: 1,
        timestamp: now,
        tier: "facts".into(),
        payload: serde_json::json!({"event": "approve"}),
        hash: "sha256:abc".into(),
        previous_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            .into(),
    };

    let prov_ref = prov.clone();
    let updated = store
        .update_instance_atomic("inst-x", &move |row| {
            let obj = row
                .instance_json
                .as_object_mut()
                .expect("instance_json object");
            obj.insert("status".into(), serde_json::json!("completed"));
            Ok(vec![prov_ref.clone()])
        })
        .await
        .unwrap();

    // Instance row reflects mutation.
    assert_eq!(
        updated
            .instance_json
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap(),
        "completed"
    );

    // Provenance row was appended in the same txn.
    let tail = store.last_provenance("inst-x").await.unwrap().unwrap();
    assert_eq!(tail.seq, 1);
    assert_eq!(tail.tier, "facts");
}

#[tokio::test]
async fn session_lifecycle_revocation() {
    let store = fresh().await;
    let future = Utc::now() + chrono::Duration::hours(1);
    let past = Utc::now() - chrono::Duration::hours(1);

    store
        .upsert_session(&wos_server::storage::SessionRow {
            jti: "jti-valid".into(),
            user_id: "u".into(),
            expires_at: future,
            revoked: false,
        })
        .await
        .unwrap();
    assert!(store.session_is_valid("jti-valid").await.unwrap());

    store.revoke_session("jti-valid").await.unwrap();
    assert!(!store.session_is_valid("jti-valid").await.unwrap());

    store
        .upsert_session(&wos_server::storage::SessionRow {
            jti: "jti-expired".into(),
            user_id: "u".into(),
            expires_at: past,
            revoked: false,
        })
        .await
        .unwrap();
    assert!(!store.session_is_valid("jti-expired").await.unwrap());
    assert!(!store.session_is_valid("never-issued").await.unwrap());
}

#[tokio::test]
async fn upsert_user_on_conflict_preserves_auth_epoch() {
    let store = fresh().await;
    let now = Utc::now();
    store
        .upsert_user(&UserRow {
            id: "u1".into(),
            email: "a@example.com".into(),
            name: "A".into(),
            role: "Supervisor".into(),
            password_hash: "hash1".into(),
            avatar: None,
            auth_epoch: 0,
            created_at: now,
        })
        .await
        .unwrap();
    store.bump_user_auth_epoch("u1").await.unwrap();
    store
        .upsert_user(&UserRow {
            id: "u1".into(),
            email: "b@example.com".into(),
            name: "B".into(),
            role: "Caseworker".into(),
            password_hash: "hash2".into(),
            avatar: None,
            auth_epoch: 0,
            created_at: now,
        })
        .await
        .unwrap();
    let u = store.get_user("u1").await.unwrap().unwrap();
    assert_eq!(u.auth_epoch, 1);
    assert_eq!(u.email, "b@example.com");
    assert_eq!(u.password_hash, "hash1");
}

#[tokio::test]
async fn revoke_sessions_for_user_revokes_all_jtis_for_that_user() {
    let store = fresh().await;
    let exp = Utc::now() + chrono::Duration::hours(1);
    for jti in ["a1", "a2", "other-user"] {
        let uid = if jti == "other-user" { "u2" } else { "u1" };
        store
            .upsert_session(&wos_server::storage::SessionRow {
                jti: jti.into(),
                user_id: uid.into(),
                expires_at: exp,
                revoked: false,
            })
            .await
            .unwrap();
    }
    assert!(store.session_is_valid("a1").await.unwrap());
    assert!(store.session_is_valid("a2").await.unwrap());
    assert!(store.session_is_valid("other-user").await.unwrap());

    store.revoke_sessions_for_user("u1").await.unwrap();
    assert!(!store.session_is_valid("a1").await.unwrap());
    assert!(!store.session_is_valid("a2").await.unwrap());
    assert!(store.session_is_valid("other-user").await.unwrap());
}

#[tokio::test]
async fn inbound_cloud_event_insert_returns_true_then_false() {
    let store = fresh().await;
    let row = InboundCloudEventRow {
        cloud_event_id: "ce-dedupe-1".into(),
        instance_id: "inst-a".into(),
        binding: "com.example.event".into(),
        received_at: Utc::now(),
        payload_json: serde_json::json!({}),
    };
    assert!(store.insert_inbound_cloud_event(&row).await.unwrap());
    assert!(!store.insert_inbound_cloud_event(&row).await.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inbound_cloud_event_concurrent_inserts_one_wins() {
    let store = Arc::new(fresh().await);
    let row = InboundCloudEventRow {
        cloud_event_id: "ce-race-1".into(),
        instance_id: "inst-r".into(),
        binding: "com.example.race".into(),
        received_at: Utc::now(),
        payload_json: serde_json::json!({"k": 1}),
    };
    let s2 = store.clone();
    let row_b = row.clone();
    let (a, b) = tokio::join!(
        async { store.insert_inbound_cloud_event(&row).await.unwrap() },
        async { s2.insert_inbound_cloud_event(&row_b).await.unwrap() },
    );
    assert_ne!(a, b, "exactly one insert should succeed under contention");
    assert!(a || b);
}

#[tokio::test]
async fn list_instances_all_pages_collects_beyond_single_sqlite_page() {
    let store = fresh().await;
    let now = Utc::now();
    for i in 0..250 {
        store
            .create_instance(&InstanceRow {
                instance_id: format!("all-pages-{i}"),
                definition_url: "urn:wos:workflow:demo".into(),
                definition_version: "1".into(),
                status: "active".into(),
                impact_level: "operational".into(),
                instance_json: serde_json::json!({ "configuration": ["intake"] }),
                runtime_aux_json: serde_json::json!({}),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
    }
    let handle: StorageHandle = Arc::new(store);
    let rows = storage::list_instances_all_pages(
        &handle,
        InstanceQuery::default(),
        storage::LIST_INSTANCES_PAGE_SIZE_MAX,
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 250);
}
