use std::sync::Arc;

use thiserror::Error;
use tokio::runtime::Handle;
use wos_core::GovernanceDocument;
use wos_core::KernelDocument;
use wos_core::traits::DocumentResolver;
use wos_runtime::RuntimeError;
use wos_server_ports::runtime::BundleResolverPort;

#[derive(Clone)]
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
                .resolve_kernel_bundle(&url, &wanted)
                .await
                .map_err(|_| ResolverError::KernelNotFound { url: url.clone() })?;
            let parsed = serde_json::from_value::<KernelDocument>(kernel_json).map_err(|e| {
                ResolverError::KernelParse {
                    url: url.clone(),
                    message: e.to_string(),
                }
            })?;
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

/// Wraps [`BundleServiceResolver`] so [`DocumentResolver::Error`] is
/// [`RuntimeError`], preserving typed kernel URL / version mismatch signals for
/// HTTP mapping (see `wos-server` `From<RuntimeError> for ApiError`).
#[derive(Clone)]
pub struct RuntimeKernelResolver(pub BundleServiceResolver);

impl RuntimeKernelResolver {
    pub fn new(bundle: Arc<dyn BundleResolverPort>, handle: Handle) -> Self {
        Self(BundleServiceResolver::new(bundle, handle))
    }
}

fn map_bundle_resolver_error(e: ResolverError) -> RuntimeError {
    match e {
        ResolverError::KernelNotFound { url } => RuntimeError::KernelWorkflowNotFound { url },
        ResolverError::KernelVersionMismatch {
            url,
            found,
            wanted,
        } => RuntimeError::KernelDefinitionVersionMismatch {
            url,
            loaded_version: found,
            requested_version: wanted,
        },
        other => RuntimeError::Resolver(other.to_string()),
    }
}

impl DocumentResolver for RuntimeKernelResolver {
    type Error = RuntimeError;

    fn resolve_kernel(&self, url: &str, version: &str) -> Result<KernelDocument, Self::Error> {
        self.0
            .resolve_kernel(url, version)
            .map_err(map_bundle_resolver_error)
    }

    fn resolve_governance(
        &self,
        url: &str,
        version: &str,
    ) -> Result<GovernanceDocument, Self::Error> {
        self.0
            .resolve_governance(url, version)
            .map_err(map_bundle_resolver_error)
    }

    fn resolve_sidecar(
        &self,
        url: &str,
        anchor_date: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error> {
        self.0
            .resolve_sidecar(url, anchor_date)
            .map_err(map_bundle_resolver_error)
    }
}
