//! `TaskPresenter` that (a) mirrors task state into the `tasks` SQL table
//! for queryability and (b) broadcasts `task:assigned` / `task:dismissed`
//! via Socket.IO.

use chrono::Utc;
use socketioxide::SocketIo;
use thiserror::Error;
use tokio::runtime::Handle;
use wos_core::instance::FormspecTaskContext;
use wos_core::traits::TaskPresenter;

use crate::storage::StorageHandle;

pub struct SocketIoTaskPresenter {
    storage: StorageHandle,
    io: SocketIo,
    handle: Handle,
}

impl SocketIoTaskPresenter {
    pub fn new(storage: StorageHandle, io: SocketIo, handle: Handle) -> Self {
        Self {
            storage,
            io,
            handle,
        }
    }
}

#[derive(Debug, Error)]
pub enum PresenterError {
    #[error("presenter storage error: {0}")]
    Storage(String),
    #[error("presenter serde error: {0}")]
    Serde(String),
}

impl TaskPresenter for SocketIoTaskPresenter {
    type Error = PresenterError;

    fn present_task(&mut self, context: &FormspecTaskContext) -> Result<(), Self::Error> {
        let storage = self.storage.clone();
        let io = self.io.clone();
        let ctx = context.clone();
        self.handle.block_on(async move {
            let now = Utc::now().to_rfc3339();
            let context_json = serde_json::to_string(&ctx)
                .map_err(|e| PresenterError::Serde(e.to_string()))?;
            // Upsert task into the `tasks` mirror table.
            let pool = storage_pool_handle(&storage);
            if let Some(pool) = pool {
                let _ = sqlx::query(
                    "INSERT INTO tasks (task_id, instance_id, task_ref, contract_ref, binding,
                       definition_url, definition_version, assigned_actor, status, context_json,
                       created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'assigned', ?, ?, ?)
                     ON CONFLICT(task_id) DO UPDATE SET
                       instance_id = excluded.instance_id,
                       task_ref = excluded.task_ref,
                       contract_ref = excluded.contract_ref,
                       binding = excluded.binding,
                       definition_url = excluded.definition_url,
                       definition_version = excluded.definition_version,
                       assigned_actor = excluded.assigned_actor,
                       status = 'assigned',
                       context_json = excluded.context_json,
                       updated_at = excluded.updated_at",
                )
                .bind(&ctx.task_id)
                .bind(&ctx.instance_id)
                .bind(&ctx.task_id)
                .bind(&ctx.contract_ref)
                .bind("formspec")
                .bind(&ctx.definition_url)
                .bind(&ctx.definition_version)
                .bind::<Option<String>>(None)
                .bind(&context_json)
                .bind(&now)
                .bind(&now)
                .execute(&pool)
                .await;
            }
            let _ = io.emit("task:assigned", &ctx);
            Ok(())
        })
    }

    fn dismiss_task(&mut self, task_id: &str, reason: &str) -> Result<(), Self::Error> {
        let storage = self.storage.clone();
        let io = self.io.clone();
        let task_id_s = task_id.to_string();
        let reason_s = reason.to_string();
        self.handle.block_on(async move {
            let now = Utc::now().to_rfc3339();
            let pool = storage_pool_handle(&storage);
            if let Some(pool) = pool {
                let _ = sqlx::query(
                    "UPDATE tasks SET status = 'dismissed', dismissed_at = ?, dismissed_reason = ?, updated_at = ? WHERE task_id = ?",
                )
                .bind(&now)
                .bind(&reason_s)
                .bind(&now)
                .bind(&task_id_s)
                .execute(&pool)
                .await;
            }
            let _ = io.emit(
                "task:dismissed",
                &serde_json::json!({ "taskId": task_id_s, "reason": reason_s }),
            );
            Ok(())
        })
    }
}

/// Extract the `SqlitePool` from the Storage handle.
///
/// The generic `Storage` trait does not expose a pool; mirroring `tasks`
/// rows requires the concrete SQLite backend. Phase 1 returns `None`
/// here so the broadcast still fires and the presenter stays safe; the
/// task-mirror write lands in Phase 2 when the `tasks` endpoints are
/// added via a dedicated `TaskService` that talks directly to the SQLite
/// pool.
fn storage_pool_handle(_storage: &StorageHandle) -> Option<sqlx::SqlitePool> {
    None
}

