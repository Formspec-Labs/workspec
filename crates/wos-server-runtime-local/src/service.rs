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
