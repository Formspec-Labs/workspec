//! `DocumentResolver` bridging `BundleService` into `wos-runtime`.
//!
//! Synchronous per the wos-core trait. Calls the async `BundleService`
//! via `tokio::runtime::Handle::block_on`; safe as long as invocations
//! happen from inside `tokio::task::spawn_blocking` (which is how
//! `AppRuntime` dispatches all runtime calls).

use std::sync::Arc;

use thiserror::Error;
use tokio::runtime::Handle;
use wos_core::GovernanceDocument;
use wos_core::traits::DocumentResolver;
use wos_core::KernelDocument;

use crate::services::bundle_service::BundleService;

pub struct BundleServiceResolver {
    bundle: Arc<BundleService>,
    handle: Handle,
}

impl BundleServiceResolver {
    pub fn new(bundle: Arc<BundleService>, handle: Handle) -> Self {
        Self { bundle, handle }
    }
}

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("kernel `{url}` not loaded in registry")]
    KernelNotFound { url: String },

    #[error("kernel `{url}` loaded with version `{found}` but version `{wanted}` was requested")]
    KernelVersionMismatch {
        url: String,
        found: String,
        wanted: String,
    },

    #[error("governance sidecar for `{url}` is not loaded")]
    GovernanceNotFound { url: String },

    #[error("sidecar for `{url}` is not loaded")]
    SidecarNotFound { url: String },

    #[error("failed to parse kernel `{url}`: {message}")]
    KernelParse { url: String, message: String },

    #[error("failed to parse governance sidecar for `{url}`: {message}")]
    GovernanceParse { url: String, message: String },
}

impl DocumentResolver for BundleServiceResolver {
    type Error = ResolverError;

    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, Self::Error> {
        let bundle = self.bundle.clone();
        let url = url.to_string();
        let wanted = version.to_string();
        self.handle.block_on(async move {
            let row = bundle
                .get(&url)
                .await
                .ok_or_else(|| ResolverError::KernelNotFound { url: url.clone() })?;
            if !wanted.is_empty() && row.version != wanted {
                return Err(ResolverError::KernelVersionMismatch {
                    url,
                    found: row.version,
                    wanted,
                });
            }
            serde_json::from_value::<KernelDocument>(row.document).map_err(|e| {
                ResolverError::KernelParse {
                    url,
                    message: e.to_string(),
                }
            })
        })
    }

    fn resolve_governance(
        &self,
        url: &str,
        _version: &str,
    ) -> Result<GovernanceDocument, Self::Error> {
        let bundle = self.bundle.clone();
        let url = url.to_string();
        self.handle.block_on(async move {
            let bundle_view = bundle
                .full_bundle(&url)
                .await
                .ok_or_else(|| ResolverError::GovernanceNotFound { url: url.clone() })?;
            let gov = bundle_view
                .governance
                .ok_or(ResolverError::GovernanceNotFound { url: url.clone() })?;
            serde_json::from_value::<GovernanceDocument>(gov).map_err(|e| {
                ResolverError::GovernanceParse {
                    url,
                    message: e.to_string(),
                }
            })
        })
    }

    fn resolve_sidecar(
        &self,
        url: &str,
        _anchor_date: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error> {
        let bundle = self.bundle.clone();
        let url = url.to_string();
        self.handle.block_on(async move {
            let bundle_view = bundle
                .full_bundle(&url)
                .await
                .ok_or_else(|| ResolverError::SidecarNotFound { url: url.clone() })?;
            Ok(serde_json::to_value(bundle_view).unwrap_or(serde_json::json!({})))
        })
    }
}
