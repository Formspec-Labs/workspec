//! Regression: timer polling must walk every instance page (see
//! `timer_task::tick_once`). This test locks the same pagination loop as that
//! task against `Storage::list_instances` (page size capped at
//! `LIST_INSTANCES_PAGE_SIZE_MAX`) without spinning a full `AppState` /
//! runtime.

use chrono::Utc;
use wos_server::storage::LIST_INSTANCES_PAGE_SIZE_MAX;
use wos_server::storage::{InstanceQuery, InstanceRow, SqliteStorage, Storage};

async fn fresh() -> SqliteStorage {
    let store = SqliteStorage::connect("sqlite::memory:?cache=shared")
        .await
        .expect("connect");
    store.migrate().await.expect("migrate");
    store
}

/// Mirrors `timer_task::tick_once` instance iteration (pages until short/empty).
async fn collect_all_instance_ids(store: &SqliteStorage) -> Vec<String> {
    let mut ids = Vec::new();
    let mut page_num: u32 = 1;
    loop {
        let page = store
            .list_instances(InstanceQuery {
                page: page_num,
                page_size: LIST_INSTANCES_PAGE_SIZE_MAX,
                ..Default::default()
            })
            .await
            .expect("list");

        if page.items.is_empty() {
            break;
        }
        for row in &page.items {
            ids.push(row.instance_id.clone());
        }
        if page.items.len() < page.page_size as usize {
            break;
        }
        page_num = page_num.saturating_add(1);
    }
    ids
}

#[tokio::test]
async fn timer_style_pagination_visits_all_instances_over_200() {
    let store = fresh().await;
    let now = Utc::now();
    let total: u32 = LIST_INSTANCES_PAGE_SIZE_MAX + 1;

    for i in 0..total {
        store
            .create_instance(&InstanceRow {
                instance_id: format!("timer-page-{i}"),
                definition_url: "urn:wos:workflow:timer-page-test".into(),
                definition_version: "1".into(),
                status: "active".into(),
                impact_level: "operational".into(),
                instance_json: serde_json::json!({
                    "configuration": ["intake"],
                    "timers": []
                }),
                runtime_aux_json: serde_json::json!({}),
                created_at: now,
                updated_at: now,
            })
            .await
            .expect("create");
    }

    let listed = store
        .list_instances(InstanceQuery {
            page: 1,
            page_size: LIST_INSTANCES_PAGE_SIZE_MAX,
            ..Default::default()
        })
        .await
        .expect("page1");
    assert_eq!(
        listed.items.len(),
        LIST_INSTANCES_PAGE_SIZE_MAX as usize
    );

    let ids = collect_all_instance_ids(&store).await;
    assert_eq!(ids.len(), total as usize);
}
