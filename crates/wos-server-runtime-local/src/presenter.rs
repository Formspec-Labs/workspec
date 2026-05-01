use chrono::Utc;
use socketioxide::SocketIo;
use thiserror::Error;
use tokio::runtime::Handle;
use wos_core::instance::FormspecTaskContext;
use wos_core::traits::TaskPresenter;
use wos_server_ports::storage::StorageHandle;

pub struct SocketIoTaskPresenter {
    #[allow(dead_code)]
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
        let io = self.io.clone();
        let ctx = context.clone();
        self.handle.block_on(async move {
            let _ = io.emit("task:assigned", &ctx);
            Ok(())
        })
    }

    fn dismiss_task(&mut self, task_id: &str, reason: &str) -> Result<(), Self::Error> {
        let io = self.io.clone();
        let task_id_s = task_id.to_string();
        let reason_s = reason.to_string();
        self.handle.block_on(async move {
            let _ = io.emit(
                "task:dismissed",
                &serde_json::json!({
                    "taskId": task_id_s,
                    "reason": reason_s,
                    "dismissedAt": Utc::now().to_rfc3339(),
                }),
            );
            Ok(())
        })
    }
}
