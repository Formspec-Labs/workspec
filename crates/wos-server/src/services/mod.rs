//! Domain services. Each service wraps storage + wos-core with
//! business logic and returns studio-shaped view types.

use std::sync::Arc;

use crate::config::ServerConfig;
use crate::storage::StorageHandle;

pub mod bundle_service;

pub use bundle_service::BundleService;

pub struct AppServices {
    pub bundle: BundleService,
}

impl AppServices {
    pub async fn new(cfg: Arc<ServerConfig>, storage: StorageHandle) -> anyhow::Result<Self> {
        let bundle = BundleService::new(cfg.clone(), storage.clone()).await?;
        Ok(Self { bundle })
    }
}
