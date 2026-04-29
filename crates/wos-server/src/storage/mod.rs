//! Storage adapter dispatch.
//!
//! Trait + row types live in [`wos_server_ports::storage`]; this module re-exports
//! them and adds the concrete SQLite adapter + build helper. When Postgres lands
//! (WS-020), it adds a sibling `wos_server_postgres` crate that also implements
//! `wos_server_ports::Storage` — no edits to this file needed.

#[cfg(feature = "storage-sqlite")]
pub use wos_server_sqlite::SqliteStorage;
#[cfg(feature = "storage-postgres")]
pub use wos_server_postgres::PostgresStorage;

// Re-export everything from the ports crate so existing call sites (`use
// crate::storage::{InstanceRow, Storage, …}`) resolve unchanged.
pub use wos_server_ports::storage::{
    AgentRow, DelegationRow, IdentityFactRow, InboundCloudEventRow, InstanceMutator, InstanceQuery,
    InstanceRow, IntakeRecordRow, KernelRow, Page, ProvenanceRow, SessionRow, Storage, StorageError,
    StorageHandle, StorageResult, UserRow, LIST_INSTANCES_PAGE_SIZE_MAX,
};

/// Walk [`Storage::list_instances`] until every row matching `query` filters is
/// collected. The requested `page_size` argument is clamped to
/// \[1, [`LIST_INSTANCES_PAGE_SIZE_MAX`]\].
pub async fn list_instances_all_pages(
    storage: &StorageHandle,
    mut query: InstanceQuery,
    page_size: u32,
) -> StorageResult<Vec<InstanceRow>> {
    let page_size = page_size.clamp(1, LIST_INSTANCES_PAGE_SIZE_MAX);
    let mut out = Vec::new();
    let mut page_num = 1u32;
    loop {
        query.page = page_num;
        query.page_size = page_size;
        let page = storage.list_instances(query.clone()).await?;
        if page.items.is_empty() {
            break;
        }
        let n = page.items.len();
        out.extend(page.items);
        if n < page_size as usize {
            break;
        }
        page_num = page_num.saturating_add(1);
    }
    Ok(out)
}

use crate::config::{ServerConfig, StorageKind};

pub async fn build(cfg: &ServerConfig) -> anyhow::Result<StorageHandle> {
    match cfg.storage {
        StorageKind::Sqlite => {
            #[cfg(feature = "storage-sqlite")]
            {
                let store = SqliteStorage::connect(&cfg.database_url).await?;
                store.migrate().await?;
                return Ok(std::sync::Arc::new(store));
            }
            #[cfg(not(feature = "storage-sqlite"))]
            anyhow::bail!(
                "WOS_STORAGE=sqlite requested but crate built without feature `storage-sqlite`"
            )
        }
        StorageKind::Postgres => {
            #[cfg(feature = "storage-postgres")]
            {
                let store = PostgresStorage::connect(&cfg.database_url)?;
                return Ok(std::sync::Arc::new(store));
            }
            #[cfg(not(feature = "storage-postgres"))]
            anyhow::bail!(
                "WOS_STORAGE=postgres requested but crate built without feature `storage-postgres`"
            )
        }
        StorageKind::Embedded => anyhow::bail!(
            "WOS_STORAGE=embedded is not wired yet (WS-095 scaffold only)"
        ),
    }
}
