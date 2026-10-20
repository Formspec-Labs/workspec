// Rust guideline compliant 2026-02-21
//! Validates maintained test artifacts against only bundled owner resources.
use serde_json::Value;

struct OfflineRetriever;
impl jsonschema::Retrieve for OfflineRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let bytes = match uri.as_str() {
            "https://wos-spec.org/schemas/tooling/1.0" => {
                include_str!("../../../../schemas/wos-tooling.schema.json")
            }
            "https://wos-spec.org/schemas/workflow/1.0"
            | "https://wos-spec.org/schemas/wos-workflow.schema.json" => {
                include_str!("../../../../schemas/wos-workflow.schema.json")
            }
            "https://wos-spec.org/schemas/wos-reference-due-process.schema.json" => {
                include_str!("../../../../schemas/wos-reference-due-process.schema.json")
            }
            "https://wos-spec.org/schemas/wos-reference-replay.schema.json" => {
                include_str!("../../../../schemas/wos-reference-replay.schema.json")
            }
            _ => return Err(format!("test schema resource is unavailable locally: {uri}").into()),
        };
        Ok(serde_json::from_str(bytes)?)
    }
}

pub fn validator(schema: &Value) -> jsonschema::Validator {
    jsonschema::options()
        .with_retriever(OfflineRetriever)
        .build(schema)
        .unwrap()
}
