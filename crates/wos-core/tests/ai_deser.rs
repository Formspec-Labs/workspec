// Rust guideline compliant 2026-02-21

//! Native example admission preserves authored workflow values without conversion.
use serde_json::Value;
use std::{fs, path::PathBuf};
use wos_core::ai_policy::load_native_ai_policy;

#[test]
fn maintained_ai_examples_admit_from_the_exact_native_workflow() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in [
        "autonomy-caps-agent-config.json",
        "autonomy-caps-ai.json",
        "benefits-adjudication-ai.json",
        "benefits-drift-monitor.json",
        "deontic-enforcement-ai.json",
        "document-extractor-config.json",
    ] {
        let source: Value =
            serde_json::from_slice(&fs::read(root.join("fixtures/ai").join(name)).unwrap())
                .unwrap();
        let original = source.clone();
        let policy = load_native_ai_policy(&source)
            .unwrap_or_else(|e| panic!("{name}: {e}"))
            .expect("native AI example");
        assert_eq!(policy.source(), &original, "{name}");
        assert!(!policy.agents().is_empty(), "{name}");
        assert_eq!(
            source, original,
            "native admission cannot rewrite its source"
        );
    }
}
