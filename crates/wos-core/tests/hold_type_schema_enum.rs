//! Keep standard [`wos_core::HoldType`] wire tokens aligned with
//! `schemas/wos-workflow.schema.json` → `$defs` → `HoldPolicy` → `properties.holdType` enum arm.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use wos_core::HoldType;

/// Canonical literals (must match `HoldType::from_wire` and the workflow schema enum).
const STANDARD_HOLD_TYPE_TOKENS: &[&str] = &[
    "pending-applicant-response",
    "pending-external-verification",
    "pending-legal-review",
    "pending-legislation",
    "pending-related-case",
    "voluntary-hold",
    "legal-hold",
];

fn workspace_root() -> PathBuf {
    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root is two levels above crates/wos-core")
        .to_path_buf();

    let cwd = std::env::current_dir().ok();
    for candidate in [Some(manifest_root), cwd].into_iter().flatten() {
        for ancestor in candidate.ancestors() {
            if ancestor.join("fixtures").is_dir()
                && ancestor.join("schemas/wos-workflow.schema.json").is_file()
            {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("could not resolve workspace root with fixtures/ and schemas/");
}

#[test]
fn standard_hold_types_match_workflow_schema_enum() {
    let schema_path = workspace_root().join("schemas/wos-workflow.schema.json");
    let raw = fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display()));
    let root: Value = serde_json::from_str(&raw).expect("workflow schema JSON");

    let arr = root
        .pointer("/$defs/HoldPolicy/properties/holdType/oneOf/0/enum")
        .and_then(Value::as_array)
        .expect("HoldPolicy.holdType standard enum array missing");

    let schema_tokens: HashSet<String> = arr
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();

    let expected: HashSet<String> = STANDARD_HOLD_TYPE_TOKENS
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    assert_eq!(schema_tokens, expected);

    for token in STANDARD_HOLD_TYPE_TOKENS {
        let h: HoldType = serde_json::from_str(&format!("\"{token}\""))
            .unwrap_or_else(|e| panic!("deserialize HoldType from {token:?}: {e}"));
        assert_eq!(h.as_str(), *token);
        assert!(
            !matches!(h, HoldType::Vendor(_)),
            "standard token {token} must not deserialize as vendor"
        );
    }

    let pattern = root
        .pointer("/$defs/HoldPolicy/properties/holdType/oneOf/1/pattern")
        .and_then(Value::as_str)
        .expect("vendor holdType pattern missing");
    assert_eq!(pattern, "^x-[a-z][a-z0-9-]*$");
}
