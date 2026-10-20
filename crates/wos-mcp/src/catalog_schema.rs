//! Local canonical resources make discovery schemas usable by independent clients.

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

static OWNERS: LazyLock<BTreeMap<String, Value>> = LazyLock::new(|| {
    [
        include_str!("../../../schemas/wos-workflow.schema.json"),
        include_str!("../../../schemas/wos-tooling.schema.json"),
        include_str!("../../../schemas/wos-reference-due-process.schema.json"),
        include_str!("../../../schemas/wos-reference-replay.schema.json"),
    ]
    .into_iter()
    .map(|text| {
        let schema: Value = serde_json::from_str(text).expect("canonical schema JSON");
        (
            schema["$id"].as_str().expect("canonical schema ID").into(),
            schema,
        )
    })
    .collect()
});

/// Retain reachable owner fragments at their original IDs and JSON Pointers.
///
/// This is a closed catalog presentation step, not a schema validator or a
/// network resolver. Unsupported reference forms fail catalog construction.
pub(super) fn portable(schema: &Value) -> Value {
    closure(schema).expect("catalog references use distributed canonical fragments")
}

fn closure(schema: &Value) -> Result<Value, String> {
    let mut pending = vec![];
    references(schema, "https://wos-spec.org/mcp/catalog", &mut pending)?;
    let mut seen = BTreeSet::new();
    let mut resources = BTreeMap::<String, Value>::new();
    while let Some(reference) = pending.pop() {
        if !seen.insert(reference.clone()) {
            continue;
        }
        let uri = jsonschema::uri::from_str(&reference).map_err(|e| e.to_string())?;
        let id = uri.strip_fragment().as_str().to_string();
        let owner = OWNERS
            .get(&id)
            .ok_or_else(|| format!("unavailable owner: {id}"))?;
        let pointer = uri
            .fragment()
            .ok_or("owner reference requires a fragment")?
            .decode()
            .to_string()
            .map_err(|_| "fragment is not UTF-8")?;
        if !pointer.starts_with('/') {
            return Err(format!(
                "only canonical JSON Pointer fragments are supported: {reference}"
            ));
        }
        owner
            .pointer(&pointer)
            .ok_or_else(|| format!("unknown fragment: {reference}"))?;
        // A reference may select an array member. Retain the complete canonical
        // array so its JSON Schema keyword remains an array with no null holes.
        let pointer = containing_array_or_pointer(owner, &pointer)?;
        let fragment = owner
            .pointer(&pointer)
            .ok_or_else(|| format!("unknown fragment: {reference}"))?;
        references(fragment, &id, &mut pending)?;
        let resource = resources
            .entry(id.clone())
            .or_insert_with(|| json!({"$id":id,"$schema":owner["$schema"]}));
        // Array-member selections have widened to their first array ancestor;
        // the remaining parents are canonical objects.
        let tokens: Vec<_> = pointer[1..]
            .split('/')
            .map(|s| s.replace("~1", "/").replace("~0", "~"))
            .collect();
        let mut destination = resource;
        for token in &tokens[..tokens.len() - 1] {
            destination = destination
                .as_object_mut()
                .ok_or("non-object canonical pointer")?
                .entry(token.clone())
                .or_insert_with(|| json!({}));
        }
        destination
            .as_object_mut()
            .ok_or("non-object canonical pointer parent")?
            .insert(
                tokens.last().expect("nonempty pointer").clone(),
                fragment.clone(),
            );
    }
    let mut result = schema.clone();
    if !resources.is_empty() {
        let definitions = result
            .as_object_mut()
            .ok_or("catalog schema is an object")?
            .entry("$defs")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .ok_or("$defs is an object")?;
        for (index, resource) in resources.into_values().enumerate() {
            let key = format!("wosCanonical{index}");
            if definitions.insert(key, resource).is_some() {
                return Err("canonical resource key collision".into());
            }
        }
    }
    Ok(result)
}

fn containing_array_or_pointer(owner: &Value, pointer: &str) -> Result<String, String> {
    let mut prefix = String::new();
    let mut current = owner;
    for token in pointer[1..].split('/') {
        if current.is_array() {
            return Ok(prefix);
        }
        prefix.push('/');
        prefix.push_str(token);
        current = owner
            .pointer(&prefix)
            .ok_or_else(|| format!("unknown canonical pointer: {pointer}"))?;
    }
    Ok(prefix)
}

fn references(value: &Value, base: &str, pending: &mut Vec<String>) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            // Current canonical owners have one root ID and ordinary $ref.
            // Refuse a future schema feature rather than misrepresent its scope.
            for keyword in [
                "$id",
                "$dynamicRef",
                "$recursiveRef",
                "$anchor",
                "$dynamicAnchor",
            ] {
                if object.contains_key(keyword) {
                    return Err(format!("unsupported nested schema keyword: {keyword}"));
                }
            }
            if let Some(reference) = object.get("$ref") {
                let base = jsonschema::uri::from_str(base).map_err(|e| e.to_string())?;
                pending.push(
                    jsonschema::uri::resolve_against(
                        &base.borrow(),
                        reference.as_str().ok_or("$ref is a string")?,
                    )
                    .map_err(|e| e.to_string())?
                    .as_str()
                    .into(),
                );
            }
            for child in object.values() {
                references(child, base, pending)?;
            }
        }
        Value::Array(array) => {
            for child in array {
                references(child, base, pending)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RefuseRetrieval;
    impl jsonschema::Retrieve for RefuseRetrieval {
        fn retrieve(
            &self,
            uri: &jsonschema::Uri<String>,
        ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
            Err(format!("unexpected schema retrieval: {uri}").into())
        }
    }

    #[test]
    fn retains_recursive_owner_constraints_without_external_retrieval() {
        let schema = portable(
            &json!({"type":"object","properties":{"state":{"$ref":"https://wos-spec.org/schemas/wos-workflow.schema.json#/$defs/State"}}}),
        );
        let validator = jsonschema::options()
            .with_retriever(RefuseRetrieval)
            .build(&schema)
            .unwrap();
        assert!(validator.is_valid(&json!({"state":{"type":"compound","initialState":"a","states":{"a":{"type":"atomic"}}}})));
        assert!(!validator.is_valid(&json!({"state":{"type":"invented"}})));
        assert!(schema.to_string().contains("#/$defs/State"));
    }

    #[test]
    fn resolves_encoded_pointers_and_cross_owner_relative_references() {
        for reference in [
            "https://wos-spec.org/schemas/wos-workflow.schema.json#/%24defs/State",
            "https://wos-spec.org/schemas/wos-reference-due-process.schema.json#/$defs/DueProcessExecution",
        ] {
            let schema = portable(&json!({"$ref":reference}));
            let validator = jsonschema::options()
                .with_retriever(RefuseRetrieval)
                .build(&schema)
                .unwrap();
            assert!(!validator.is_valid(&json!("invalid native object")));
        }
    }

    #[test]
    fn array_member_references_preserve_complete_canonical_arrays() {
        let pointer = "/$defs/Actor/properties/type/oneOf/0";
        let schema = portable(
            &json!({"$ref":format!("https://wos-spec.org/schemas/wos-workflow.schema.json#{pointer}")}),
        );
        let owner = &OWNERS["https://wos-spec.org/schemas/wos-workflow.schema.json"];
        let array_pointer = "/$defs/Actor/properties/type/oneOf";
        assert_eq!(
            schema["$defs"]["wosCanonical0"]
                .pointer(array_pointer)
                .unwrap(),
            owner.pointer(array_pointer).unwrap()
        );
        let validator = jsonschema::options()
            .with_retriever(RefuseRetrieval)
            .build(&schema)
            .unwrap();
        assert!(validator.is_valid(&json!("human")));
        assert!(!validator.is_valid(&json!("invented")));
    }

    #[test]
    fn every_advertised_schema_compiles_without_external_resources() {
        for tool in crate::catalog::descriptors()["tools"].as_array().unwrap() {
            for key in ["inputSchema", "outputSchema"] {
                jsonschema::options()
                    .with_retriever(RefuseRetrieval)
                    .build(&tool[key])
                    .unwrap_or_else(|error| panic!("{} {key}: {error}", tool["name"]));
            }
        }
    }

    #[test]
    fn refuses_unavailable_or_unsupported_references() {
        for schema in [
            json!({"$ref":"https://example.invalid/schema#/a"}),
            json!({"$ref":"https://wos-spec.org/schemas/wos-workflow.schema.json#unknown"}),
            json!({"$ref":"https://wos-spec.org/schemas/wos-workflow.schema.json#/$defs/Actor/properties/type/oneOf/99"}),
            json!({"$dynamicRef":"#x"}),
        ] {
            assert!(closure(&schema).is_err());
        }
    }
}
