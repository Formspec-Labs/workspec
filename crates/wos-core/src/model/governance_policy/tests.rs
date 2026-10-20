// Rust guideline compliant 2026-02-21
//! Annotation examples use the actual canonical resource resolver.
use super::*;
#[test]
fn every_rules_schema_annotation_example_validates_at_its_actual_node() {
    let root: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/wos-governance-policy-rules.schema.json"
    ))
    .unwrap();
    fn walk(node: &Value, root: &Value, path: &str, count: &mut usize) {
        if let Some(examples) = node.get("examples").and_then(Value::as_array) {
            let mut wrapper = serde_json::json!({"$schema":"https://json-schema.org/draft/2020-12/schema","$defs":root["$defs"],"$ref":"#/$defs/ExampleTarget"});
            wrapper["$defs"]["ExampleTarget"] = node.clone();
            let validator = jsonschema::options()
                .with_retriever(RulesResources)
                .build(&wrapper)
                .unwrap();
            for example in examples {
                assert!(
                    validator.is_valid(example),
                    "{path}: {example}: {:?}",
                    validator
                        .iter_errors(example)
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                );
                *count += 1;
            }
        }
        match node {
            Value::Object(fields) => {
                for (name, child) in fields {
                    if !matches!(name.as_str(), "examples" | "description" | "x-wos") {
                        walk(child, root, &format!("{path}/{name}"), count);
                    }
                }
            }
            Value::Array(children) => {
                for (index, child) in children.iter().enumerate() {
                    walk(child, root, &format!("{path}/{index}"), count);
                }
            }
            _ => {}
        }
    }
    let mut count = 0;
    walk(&root, &root, "", &mut count);
    assert!(count > 30, "only {count} examples were checked");
}
