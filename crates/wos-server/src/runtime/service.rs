//! Phase-1 stub `ExternalService`. Returns the input echo as output — this
//! is enough to let workflows that include `invokeService` actions make
//! progress in test mode. A real implementation dispatches on integration-
//! profile bindings (arazzo, tool, policy, etc.) and is the focus of
//! Phase 9.

use thiserror::Error;
use wos_core::traits::ExternalService;

#[derive(Debug, Default)]
pub struct EchoExternalService;

#[derive(Debug, Error)]
pub enum EchoServiceError {
    #[error("external service error: {0}")]
    Other(String),
}

impl ExternalService for EchoExternalService {
    type Error = EchoServiceError;

    fn invoke(
        &self,
        service_ref: &str,
        input: &serde_json::Value,
        idempotency_key: Option<&str>,
    ) -> Result<serde_json::Value, Self::Error> {
        Ok(serde_json::json!({
            "serviceRef": service_ref,
            "echoed": input,
            "idempotencyKey": idempotency_key,
        }))
    }
}
