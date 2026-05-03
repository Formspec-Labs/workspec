// Rust guideline compliant 2026-05-01

//! Standalone HTTP server that exposes the `WosInstance` Restate virtual object.
//!
//! Used by CI and local Phase 4 smoke tests: start this process, register its
//! listen address with Restate Server (`POST /deployments`), then run ingress
//! integration tests with `WOS_RESTATE_IT_URL` set to the cluster ingress base.

use std::net::SocketAddr;

use restate_sdk::prelude::HttpServer;
use wos_server_runtime_restate::restate_virtual::wos_instance_endpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = std::env::var("WOS_RESTATE_WORKER_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:9080".to_string())
        .parse()?;

    eprintln!("wos-restate-worker listening on {addr}");
    HttpServer::new(wos_instance_endpoint())
        .listen_and_serve(addr)
        .await;
    Ok(())
}
