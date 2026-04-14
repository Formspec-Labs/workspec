// Rust guideline compliant 2026-02-21

//! Formspec binding adapter for `wos-runtime`.

use wos_core::instance::{ActiveTask, ValidationOutcome};
use wos_runtime::binding::{
    BindingError, CaseMutationBundle, ContractBindingAdapter, PreparedTask, SubmissionValidation,
};

/// Formspec processor abstraction used by the binding adapter.
pub trait FormspecProcessor {
    /// Validate a full Formspec response envelope.
    fn validate_envelope(
        &self,
        response: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, BindingError>;

    /// Validate `response.data` against the pinned Definition.
    fn validate_definition(
        &self,
        definition_url: &str,
        definition_version: &str,
        data: &serde_json::Value,
    ) -> Result<Option<Vec<serde_json::Value>>, BindingError>;

    /// Compute prefill data for a task.
    fn compute_prefill(
        &self,
        mapping_ref: Option<&str>,
        case_state: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, BindingError>;

    /// Compute a case mutation from a completed response.
    fn map_response(
        &self,
        mapping_ref: &str,
        response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError>;
}

/// Formspec-backed binding adapter.
#[derive(Debug, Clone)]
pub struct FormspecBinding<P> {
    processor: P,
}

impl<P> FormspecBinding<P> {
    /// Create a binding adapter from a Formspec processor.
    pub fn new(processor: P) -> Self {
        Self { processor }
    }
}

impl<P> ContractBindingAdapter for FormspecBinding<P>
where
    P: FormspecProcessor + Send + Sync,
{
    fn binding(&self) -> &'static str {
        "formspec"
    }

    fn prepare_task(
        &self,
        task: &ActiveTask,
        case_state: &serde_json::Value,
    ) -> Result<PreparedTask, BindingError> {
        Ok(PreparedTask {
            prefill_data: self
                .processor
                .compute_prefill(task.prefill_mapping_ref.as_deref(), case_state)?,
        })
    }

    fn validate_submission(
        &self,
        task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<SubmissionValidation, BindingError> {
        let mut errors = validate_required_envelope_fields(response)?;
        errors.extend(self.processor.validate_envelope(response)?);

        let response_definition_url = response
            .get("definitionUrl")
            .and_then(serde_json::Value::as_str);
        let response_definition_version = response
            .get("definitionVersion")
            .and_then(serde_json::Value::as_str);
        let pin_match = response_definition_url == task.definition_url.as_deref()
            && response_definition_version == task.definition_version.as_deref();

        let mut validation_results = None;
        let definition_valid = if errors.is_empty() && pin_match {
            let data = response
                .get("data")
                .ok_or_else(|| BindingError::InvalidInput("response.data missing".to_string()))?;
            validation_results = self.processor.validate_definition(
                task.definition_url.as_deref().unwrap_or_default(),
                task.definition_version.as_deref().unwrap_or_default(),
                data,
            )?;
            validation_results
                .as_ref()
                .is_none_or(std::vec::Vec::is_empty)
        } else {
            false
        };

        if !pin_match {
            errors.push(serde_json::json!({
                "code": "pinMismatch",
                "message": "response pin does not match task pin",
            }));
        }

        Ok(SubmissionValidation {
            validation_outcome: ValidationOutcome {
                envelope_valid: errors
                    .iter()
                    .all(|error| error.get("code") != Some(&serde_json::json!("invalidEnvelope"))),
                pin_match,
                definition_valid,
                errors,
                validation_results,
            },
        })
    }

    fn compute_case_mutation(
        &self,
        task: &ActiveTask,
        response: &serde_json::Value,
    ) -> Result<Option<CaseMutationBundle>, BindingError> {
        let Some(mapping_ref) = task.response_mapping_ref.as_deref() else {
            return Ok(None);
        };
        self.processor.map_response(mapping_ref, response)
    }
}

fn validate_required_envelope_fields(
    response: &serde_json::Value,
) -> Result<Vec<serde_json::Value>, BindingError> {
    let Some(object) = response.as_object() else {
        return Ok(vec![serde_json::json!({
            "code": "invalidEnvelope",
            "message": "response must be a JSON object",
        })]);
    };

    let mut errors = Vec::new();
    for required in ["status", "definitionUrl", "definitionVersion", "data"] {
        if !object.contains_key(required) {
            errors.push(serde_json::json!({
                "code": "invalidEnvelope",
                "message": format!("missing required property '{required}'"),
            }));
        }
    }

    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Default)]
    struct StubProcessor;

    impl FormspecProcessor for StubProcessor {
        fn validate_envelope(
            &self,
            response: &serde_json::Value,
        ) -> Result<Vec<serde_json::Value>, BindingError> {
            if response
                .get("meta")
                .and_then(|meta| meta.get("rejectEnvelope"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                return Ok(vec![serde_json::json!({
                    "code": "invalidEnvelope",
                    "message": "processor rejected envelope",
                })]);
            }
            Ok(Vec::new())
        }

        fn validate_definition(
            &self,
            _definition_url: &str,
            _definition_version: &str,
            data: &serde_json::Value,
        ) -> Result<Option<Vec<serde_json::Value>>, BindingError> {
            let valid = data
                .get("approved")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            Ok(if valid {
                None
            } else {
                Some(vec![serde_json::json!({
                    "code": "definitionInvalid",
                    "message": "approved must be true",
                })])
            })
        }

        fn compute_prefill(
            &self,
            mapping_ref: Option<&str>,
            case_state: &serde_json::Value,
        ) -> Result<Option<serde_json::Value>, BindingError> {
            Ok(mapping_ref.map(|mapping_ref| {
                serde_json::json!({
                    "mappingRef": mapping_ref,
                    "caseState": case_state,
                })
            }))
        }

        fn map_response(
            &self,
            mapping_ref: &str,
            response: &serde_json::Value,
        ) -> Result<Option<CaseMutationBundle>, BindingError> {
            let mut field_updates = serde_json::Map::new();
            field_updates.insert(
                "mappingRef".to_string(),
                serde_json::Value::String(mapping_ref.to_string()),
            );
            field_updates.insert("decision".to_string(), response["data"]["approved"].clone());
            Ok(Some(CaseMutationBundle { field_updates }))
        }
    }

    fn formspec_task() -> ActiveTask {
        ActiveTask {
            task_id: "task-1".to_string(),
            task_ref: "review".to_string(),
            status: wos_core::instance::ActiveTaskStatus::Assigned,
            assigned_actor: Some("reviewer".to_string()),
            contract_ref: Some("reviewForm".to_string()),
            binding: Some("formspec".to_string()),
            definition_url: Some("urn:formspec:review".to_string()),
            definition_version: Some("1.0.0".to_string()),
            prefill_mapping_ref: Some("urn:mapping:prefill".to_string()),
            response_mapping_ref: Some("urn:mapping:response".to_string()),
            deadline: None,
            impact_level: None,
            context: None,
            last_validation_outcome: None,
            created_at: "2024-03-09T00:00:00Z".to_string(),
            updated_at: "2024-03-09T00:00:00Z".to_string(),
            extensions: Default::default(),
        }
    }

    #[test]
    fn prepare_task_returns_prefill_only() {
        let adapter = FormspecBinding::new(StubProcessor);
        let prepared = adapter
            .prepare_task(&formspec_task(), &serde_json::json!({ "seed": 1 }))
            .unwrap();
        assert_eq!(
            prepared.prefill_data,
            Some(serde_json::json!({
                "mappingRef": "urn:mapping:prefill",
                "caseState": { "seed": 1 }
            }))
        );
    }

    #[test]
    fn registers_as_formspec_binding() {
        let mut registry = wos_runtime::binding::BindingRegistry::new();
        registry.register(FormspecBinding::new(StubProcessor));

        let adapter = registry
            .get("formspec")
            .expect("formspec adapter should register");
        assert_eq!(adapter.binding(), "formspec");
    }

    #[test]
    fn validate_submission_reports_pin_mismatch() {
        let adapter = FormspecBinding::new(StubProcessor);
        let validation = adapter
            .validate_submission(
                &formspec_task(),
                &serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:formspec:other",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": true }
                }),
            )
            .unwrap();

        assert!(!validation.validation_outcome.pin_match);
        assert!(!validation.validation_outcome.definition_valid);
    }

    #[test]
    fn validate_submission_returns_definition_results() {
        let adapter = FormspecBinding::new(StubProcessor);
        let validation = adapter
            .validate_submission(
                &formspec_task(),
                &serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:formspec:review",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": false }
                }),
            )
            .unwrap();

        assert!(validation.validation_outcome.envelope_valid);
        assert!(validation.validation_outcome.pin_match);
        assert!(!validation.validation_outcome.definition_valid);
        assert_eq!(
            validation.validation_outcome.validation_results,
            Some(vec![serde_json::json!({
                "code": "definitionInvalid",
                "message": "approved must be true",
            })])
        );
    }

    #[test]
    fn compute_case_mutation_is_side_effect_free() {
        let adapter = FormspecBinding::new(StubProcessor);
        let first = adapter
            .compute_case_mutation(
                &formspec_task(),
                &serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:formspec:review",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": true }
                }),
            )
            .unwrap()
            .unwrap();
        let second = adapter
            .compute_case_mutation(
                &formspec_task(),
                &serde_json::json!({
                    "status": "completed",
                    "definitionUrl": "urn:formspec:review",
                    "definitionVersion": "1.0.0",
                    "data": { "approved": true }
                }),
            )
            .unwrap()
            .unwrap();

        assert_eq!(first.field_updates, second.field_updates);
    }
}
