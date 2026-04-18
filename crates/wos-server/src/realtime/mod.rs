//! Socket.IO realtime layer.
//!
//! Mirrors the event contract in `studio/src/services/SocketIORealtimePort.ts`:
//!
//! Client → Server
//! * `user:join`     `{ id?, name? }`   — register collaborator
//! * `cursor:move`   `{ x, y }`         — broadcast cursor position
//! * `kernel:update` `{ url, kernel }`  — persist + broadcast kernel edit
//!
//! Server → Client
//! * `kernel:init`           — bootstrap with the primary kernel on connect
//! * `kernel:changed`        — broadcast after a successful write
//! * `kernel:update-rejected`— emitted back to the sender on validation fail
//! * `users:update`          — current collaborator list
//! * `cursor:update`         — `{ userId, cursor: { x, y } }`

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use socketioxide::SocketIo;
use socketioxide::extract::{Data, SocketRef, State as SocketState};
use socketioxide::layer::SocketIoLayer;
use tokio::sync::RwLock;

use crate::AppState;
use crate::services::bundle_service::validate_kernel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaborator {
    pub id: String,
    pub name: String,
    pub cursor: Cursor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Cursor {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Default)]
pub struct RealtimeState {
    /// `sid -> collaborator` registry for `users:update` broadcasts.
    pub collaborators: RwLock<std::collections::HashMap<String, Collaborator>>,
}

#[derive(Debug, Clone, Deserialize)]
struct UserJoin {
    id: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct KernelUpdate {
    url: String,
    kernel: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
struct KernelChanged {
    url: String,
    kernel: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
struct KernelRejected {
    reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    issues: Option<serde_json::Value>,
}

/// Build the Socket.IO layer + `SocketIo` handle without registering any
/// namespaces. `lib.rs` needs a handle early so `AppRuntime::build` can
/// inject it into the `TaskPresenter`; namespace handlers are attached
/// afterwards via [`attach_namespaces`] once the full `AppState` exists.
pub fn build_io_only() -> (SocketIoLayer, SocketIo) {
    let realtime = Arc::new(RealtimeState::default());
    SocketIo::builder()
        .with_state(realtime)
        .build_layer()
}

/// Register the server's namespace handlers on the given `SocketIo` handle.
/// Must be called exactly once, after `AppState` is fully assembled.
pub fn attach_namespaces(io: &SocketIo, state: AppState) {
    io.ns("/", move |socket: SocketRef| {
        let state = state.clone();
        async move { on_connect(socket, state).await }
    });
}

async fn on_connect(socket: SocketRef, state: AppState) {
    // Emit the primary kernel straight away so the studio's `onKernelInit`
    // handler has something to render even on a cold connection.
    if let Some(primary) = state.services.bundle.primary_kernel().await {
        let _ = socket.emit(
            "kernel:init",
            &KernelChanged {
                url: primary.url,
                kernel: primary.document,
            },
        );
    }

    socket.on(
        "user:join",
        async |s: SocketRef, Data::<UserJoin>(body), SocketState::<Arc<RealtimeState>>(rt)| {
            let id = body.id.unwrap_or_else(|| s.id.to_string());
            let name = body.name.unwrap_or_else(|| "Anonymous".to_string());
            let collab = Collaborator {
                id: id.clone(),
                name,
                cursor: Cursor::default(),
            };
            rt.collaborators.write().await.insert(s.id.to_string(), collab);
            broadcast_users(&s, &rt).await;
        },
    );

    socket.on(
        "cursor:move",
        async |s: SocketRef, Data::<Cursor>(cursor), SocketState::<Arc<RealtimeState>>(rt)| {
            let user_id = {
                let mut w = rt.collaborators.write().await;
                if let Some(entry) = w.get_mut(&s.id.to_string()) {
                    entry.cursor = cursor;
                    entry.id.clone()
                } else {
                    return;
                }
            };
            let _ = s.broadcast().emit(
                "cursor:update",
                &serde_json::json!({ "userId": user_id, "cursor": cursor }),
            );
        },
    );

    let state_for_kernel = state.clone();
    socket.on(
        "kernel:update",
        move |s: SocketRef, Data::<KernelUpdate>(body)| {
            let state = state_for_kernel.clone();
            async move {
                let validation = validate_kernel(&body.kernel);
                if !validation.is_valid {
                    let _ = s.emit(
                        "kernel:update-rejected",
                        &KernelRejected {
                            reason: "validation_failed".into(),
                            issues: serde_json::to_value(&validation.issues).ok(),
                        },
                    );
                    return;
                }
                match state.services.bundle.replace(&body.url, body.kernel.clone()).await {
                    Ok(row) => {
                        let envelope = KernelChanged {
                            url: row.url,
                            kernel: row.document,
                        };
                        let _ = s.broadcast().emit("kernel:changed", &envelope);
                        let _ = s.emit("kernel:changed", &envelope);
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "kernel persist failed");
                        let _ = s.emit(
                            "kernel:update-rejected",
                            &KernelRejected {
                                reason: "persist_failed".into(),
                                issues: None,
                            },
                        );
                    }
                }
            }
        },
    );

    socket.on_disconnect(async |s: SocketRef, SocketState::<Arc<RealtimeState>>(rt)| {
        rt.collaborators.write().await.remove(&s.id.to_string());
        broadcast_users(&s, &rt).await;
    });
}

async fn broadcast_users(socket: &SocketRef, rt: &Arc<RealtimeState>) {
    let users: Vec<Collaborator> = rt.collaborators.read().await.values().cloned().collect();
    let _ = socket.emit("users:update", &users);
    let _ = socket.broadcast().emit("users:update", &users);
}
