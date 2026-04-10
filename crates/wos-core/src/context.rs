// Rust guideline compliant 2026-02-21

//! FEL evaluation context builder (Kernel S7.2).
//!
//! Constructs the evaluation context from case state, event data, and
//! instance metadata. Layers enrich the context through seams (S7.3).

use std::collections::HashMap;

/// Evaluation context for FEL expression evaluation.
///
/// Provides the variables available to guard expressions, assertion gates,
/// deontic constraints, and other FEL expressions in WOS documents.
#[derive(Debug, Clone, Default)]
pub struct EvalContext {
    /// Case file data (Kernel S7.2: `caseFile`).
    pub case_file: HashMap<String, serde_json::Value>,

    /// Triggering event data (Kernel S7.2: `event`).
    pub event: Option<serde_json::Value>,

    /// Current task data (Kernel S7.2: `task`).
    pub task: Option<serde_json::Value>,

    /// Workflow instance metadata (Kernel S7.2: `instance`).
    pub instance: HashMap<String, serde_json::Value>,

    /// Temporal parameters (Layer 1 enrichment: `parameters`).
    pub parameters: HashMap<String, serde_json::Value>,

    /// Agent operational state (Layer 2 enrichment: `agent`).
    pub agent: Option<serde_json::Value>,

    /// Agent output being evaluated (Layer 2 enrichment: `output`).
    pub output: Option<serde_json::Value>,

    /// Implementation-defined environment (Kernel S7.2: `env`).
    pub env: HashMap<String, serde_json::Value>,
}

impl EvalContext {
    /// Build an evaluation context from case state and optional event data.
    ///
    /// Populates the `caseFile` and `event` namespaces. Additional
    /// namespaces (`instance`, `parameters`, `agent`, `output`) can be
    /// set after construction.
    pub fn from_case_state(
        case_state: &std::collections::HashMap<String, serde_json::Value>,
        event_data: Option<&serde_json::Value>,
    ) -> Self {
        let case_file = case_state.clone();

        let event = event_data.cloned();

        Self {
            case_file,
            event,
            task: None,
            instance: HashMap::new(),
            parameters: HashMap::new(),
            agent: None,
            output: None,
            env: HashMap::new(),
        }
    }

    /// Build a `fel_core::MapEnvironment` from this context.
    ///
    /// Flattens the typed context into the flat namespace that FEL expects.
    pub fn to_fel_environment(&self) -> fel_core::MapEnvironment {
        use fel_core::types::FelValue;

        let mut fields = HashMap::new();

        // Case file as object + dotted paths for both access patterns.
        let case_pairs: Vec<(String, FelValue)> = self
            .case_file
            .iter()
            .map(|(k, v)| (k.clone(), json_to_fel_value(v)))
            .collect();
        fields.insert("caseFile".to_string(), FelValue::Object(case_pairs.clone()));
        for (k, v) in &case_pairs {
            fields.insert(format!("caseFile.{k}"), v.clone());
        }

        // Event data as object + dotted paths.
        if let Some(event_val) = &self.event {
            let event_pairs: Vec<(String, FelValue)> = event_val
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .map(|(k, v)| (k.clone(), json_to_fel_value(v)))
                        .collect()
                })
                .unwrap_or_default();
            fields.insert("event".to_string(), FelValue::Object(event_pairs.clone()));
            for (k, v) in &event_pairs {
                fields.insert(format!("event.{k}"), v.clone());
            }
        }

        // Instance metadata as object + dotted paths.
        if !self.instance.is_empty() {
            let instance_pairs: Vec<(String, FelValue)> = self
                .instance
                .iter()
                .map(|(k, v)| (k.clone(), json_to_fel_value(v)))
                .collect();
            fields.insert(
                "instance".to_string(),
                FelValue::Object(instance_pairs.clone()),
            );
            for (k, v) in &instance_pairs {
                fields.insert(format!("instance.{k}"), v.clone());
            }
        }

        // Parameters as dotted paths.
        for (key, value) in &self.parameters {
            fields.insert(
                format!("parameters.{key}"),
                json_to_fel_value(value),
            );
        }

        fel_core::MapEnvironment::with_fields(fields)
    }
}

/// Convert a JSON value to a FEL value.
///
/// Delegates to `fel_core::json_to_fel` which correctly handles all
/// JSON types including nested objects and arrays.
fn json_to_fel_value(value: &serde_json::Value) -> fel_core::FelValue {
    fel_core::json_to_fel(value)
}
