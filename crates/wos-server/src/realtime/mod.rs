//! Socket.IO realtime layer.
//!
//! Mirrors the event contract in `studio/src/services/SocketIORealtimePort.ts`:
//!
//! Client → Server
//! * `user:join`     `{ id?, name? }`   — register collaborator
//! * `cursor:move`   `{ x, y }`         — broadcast cursor position
//! * `kernel:update` `{ url, kernel }`  — persist + broadcast kernel edit.
//!   **`WOS_AUTH=jwt`:** handshake must include `auth: { token: "<access_jwt>" }`
//!   (same access JWT as HTTP). Each `kernel:update` re-runs
//!   [`AuthProvider::verify`](crate::auth::AuthProvider::verify) on that token
//!   and requires **Supervisor** from the current user row (so logout, expiry,
//!   revocation, and role changes apply without relying on connect-time cache).
//!   **`WOS_AUTH=mock`:** updates allowed without a token (local studio).
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
use socketioxide::extract::{Data, SocketRef, State as SocketState, TryData};
use socketioxide::layer::SocketIoLayer;
use tokio::sync::RwLock;

use crate::AppState;
use crate::config::AuthKind;
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
    /// JWT mode only: `sid ->` access JWT presented at connect when
    /// [`AuthProvider::verify`] succeeded. Each `kernel:update` re-verifies this
    /// token (see module docs). Missing entry or `None` means no usable token.
    pub socket_access_token: RwLock<std::collections::HashMap<String, Option<String>>>,
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

/// Socket.IO client `auth` payload from `studio` (`auth: { token }`).
#[derive(Debug, Clone, Deserialize)]
struct HandshakeAuth {
    #[serde(default)]
    token: Option<String>,
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
    io.ns(
        "/",
        move |socket: SocketRef,
              TryData(handshake): TryData<HandshakeAuth>,
              SocketState(rt): SocketState<Arc<RealtimeState>>| {
            let state = state.clone();
            let token = match handshake.as_ref() {
                Ok(h) => h.token.clone(),
                Err(_) => None,
            };
            async move { on_connect(socket, state, rt, token).await }
        },
    );
}

async fn on_connect(
    socket: SocketRef,
    state: AppState,
    rt: Arc<RealtimeState>,
    token: Option<String>,
) {
    if matches!(state.cfg.auth, AuthKind::Jwt) {
        // WS-009: reject malformed/missing/expired handshake tokens at connect
        // time so idle malicious connections do not hold server resources.
        // Pre-WS-009 behavior cached `None` and let the first `kernel:update`
        // surface the failure; that wasted a connection slot per bad client.
        let valid_token = match &token {
            Some(t) if state.auth.verify(t).await.is_ok() => Some(t.clone()),
            _ => None,
        };
        let Some(valid_token) = valid_token else {
            let _ = socket.disconnect();
            return;
        };
        rt.socket_access_token
            .write()
            .await
            .insert(socket.id.to_string(), Some(valid_token));
    }
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
        move |s: SocketRef,
              Data::<KernelUpdate>(body),
              SocketState::<Arc<RealtimeState>>(rt)| {
            let state = state_for_kernel.clone();
            async move {
                let allowed = match state.cfg.auth {
                    AuthKind::Mock => true,
                    AuthKind::Jwt => {
                        let token = rt
                            .socket_access_token
                            .read()
                            .await
                            .get(&s.id.to_string())
                            .and_then(|v| v.clone());
                        match token {
                            Some(t) => match state.auth.verify(&t).await {
                                Ok(ctx) => {
                                    // Last non-typed-role check post-WS-083 sweep:
                                    // `kernel:update` is a Socket.IO event, not a
                                    // `FromRequestParts` extractor, so the
                                    // `RequireRole<Supervisor>` axum extractor does
                                    // not reach this site. Use `Supervisor::NAME`
                                    // (via the `Role` trait) so role-typo bugs are
                                    // caught at compile time.
                                    ctx.user.role.eq_ignore_ascii_case(
                                        <crate::auth::Supervisor as crate::auth::Role>::NAME,
                                    )
                                }
                                Err(_) => false,
                            },
                            None => false,
                        }
                    }
                };
                if !allowed {
                    let _ = s.emit(
                        "kernel:update-rejected",
                        &KernelRejected {
                            reason: "unauthorized".into(),
                            issues: None,
                        },
                    );
                    return;
                }
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
        rt.socket_access_token.write().await.remove(&s.id.to_string());
        broadcast_users(&s, &rt).await;
    });
}

async fn broadcast_users(socket: &SocketRef, rt: &Arc<RealtimeState>) {
    let users: Vec<Collaborator> = rt.collaborators.read().await.values().cloned().collect();
    let _ = socket.emit("users:update", &users);
    let _ = socket.broadcast().emit("users:update", &users);
}
