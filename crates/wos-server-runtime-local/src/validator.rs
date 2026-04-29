// Rust guideline compliant 2026-02-21

use thiserror::Error;
use wos_core::traits::{ContractValidator, ValidationResult};

const RIGHTS_IMPACTING: &str = "rights-impacting";
const SAFETY_IMPACTING: &str = "safety-impacting";

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

pub struct PolicyLayeredValidator<V> {
    inner: V,
}

impl<V> PolicyLayeredValidator<V> {
    pub fn new(inner: V) -> Self {
        Self { inner }
    }
}

impl<V: ContractValidator> ContractValidator for PolicyLayeredValidator<V> {
    type Error = V::Error;

    fn validate(
        &self,
        contract_ref: &str,
        data: &serde_json::Value,
    ) -> Result<ValidationResult, V::Error> {
        let mut result = self.inner.validate(contract_ref, data)?;

        let impact = data
            .get("impactLevel")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if impact == RIGHTS_IMPACTING || impact == SAFETY_IMPACTING {
            let ledger = data
                .get("respondentLedgerRef")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if ledger.is_empty() {
                result.valid = false;
                result.errors.push(format!(
                    "Runtime §15.7: {impact} submit requires respondentLedgerRef evidence"
                ));
            }
        }

        Ok(result)
    }
}
