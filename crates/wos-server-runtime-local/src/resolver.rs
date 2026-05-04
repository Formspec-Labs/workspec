use std::sync::Arc;

use thiserror::Error;
use tokio::runtime::Handle;
use wos_core::GovernanceDocument;
use wos_core::KernelDocument;
use wos_core::traits::DocumentResolver;
use wos_server_ports::runtime::BundleResolverPort;

pub struct BundleServiceResolver {
    bundle: Arc<dyn BundleResolverPort>,
    handle: Handle,
}

impl BundleServiceResolver {
    pub fn new(bundle: Arc<dyn BundleResolverPort>, handle: Handle) -> Self {
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
            let kernel_json = bundle
                .resolve_kernel_bundle(&url)
                .await
                .map_err(|_| ResolverError::KernelNotFound { url: url.clone() })?;
            let parsed = serde_json::from_value::<KernelDocument>(kernel_json).map_err(|e| {
                ResolverError::KernelParse {
                    url: url.clone(),
                    message: e.to_string(),
                }
            })?;
            let found_version = parsed.version.clone().unwrap_or_default();
            if !wanted.is_empty() && found_version != wanted {
                return Err(ResolverError::KernelVersionMismatch {
                    url,
                    found: found_version,
                    wanted,
                });
            }
            Ok(parsed)
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
            let gov_json = bundle
                .resolve_governance_bundle(&url)
                .await
                .map_err(|_| ResolverError::GovernanceNotFound { url: url.clone() })?;
            serde_json::from_value::<GovernanceDocument>(gov_json).map_err(|e| {
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
            bundle
                .resolve_sidecar_bundle(&url)
                .await
                .map_err(|_| ResolverError::SidecarNotFound { url: url.clone() })
        })
    }
}
