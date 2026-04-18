//! Permissive `ContractValidator` — accepts any payload. Real
//! `FormspecProcessor` validation lands when the binding crate exposes
//! its processor API.

use thiserror::Error;
use wos_core::traits::{ContractValidator, ValidationResult};

#[derive(Debug, Default)]
pub struct PermissiveValidator;

#[derive(Debug, Error)]
pub enum ValidatorError {
    #[error("validator error: {0}")]
    Other(String),
}

impl ContractValidator for PermissiveValidator {
    type Error = ValidatorError;

    fn validate(
        &self,
        _contract_ref: &str,
        _data: &serde_json::Value,
    ) -> Result<ValidationResult, Self::Error> {
        Ok(ValidationResult {
            valid: true,
            errors: Vec::new(),
        })
    }
}
