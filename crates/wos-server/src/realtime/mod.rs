//! Socket.IO layer (placeholder). Fleshed out after the HTTP surface is
//! stable — see plan.md Step 9.

use socketioxide::SocketIo;
use socketioxide::layer::SocketIoLayer;

use crate::AppState;

pub fn build(_state: AppState) -> (SocketIoLayer, SocketIo) {
    let (layer, io) = SocketIo::new_layer();
    io.ns("/", |_socket: socketioxide::extract::SocketRef| async move {
        // Handlers added in Step 9.
    });
    (layer, io)
}
