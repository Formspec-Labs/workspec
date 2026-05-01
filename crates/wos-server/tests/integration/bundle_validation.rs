//! Kernel validation: the bundle service's `validate_kernel` must reject
//! obviously malformed documents and accept the project's own fixtures.

use std::path::Path;

use wos_server::services::bundle_service::validate_kernel;

#[test]
fn empty_object_is_rejected() {
    let result = validate_kernel(&serde_json::json!({}));
    assert!(!result.is_valid, "empty object must fail validation");
    assert!(!result.issues.is_empty());
}

#[test]
fn fixtures_validate_cleanly() {
    // Walk every kernel in the repo's fixtures dir and confirm the lint
    // pass surfaces no error-severity issues. Warnings are tolerated — the
    // linter's Warning/Info tiers are advisory, not gating.
    let kernels = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("kernel");
    if !kernels.exists() {
        // Running outside the repo checkout; skip gracefully.
        return;
    }
    let mut checked = 0;
    for entry in std::fs::read_dir(&kernels).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let doc: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => continue, // skip non-JSON fixtures
        };
        // Skip non-kernel JSON (sidecars, etc.). Published fixtures use
        // `$wosWorkflow` at the root; tests and inline docs may use `$wosWorkflow`.
        let is_kernel_doc = doc.get("$wosWorkflow").is_some() || doc.get("$wosWorkflow").is_some();
        if !is_kernel_doc {
            continue;
        }
        // `validate_kernel` → `wos_lint::lint_document` (see `bundle_service.rs`).
        // If the linter later splits strict kernel-only vs workflow-root gates,
        // revisit this filter or relocate non-kernel JSON out of `fixtures/kernel/`.
        let result = validate_kernel(&doc);
        let errors: Vec<_> = result
            .issues
            .iter()
            .filter(|i| i.severity == "error")
            .collect();
        assert!(
            errors.is_empty(),
            "{:?}: errors = {:?}",
            path,
            errors.iter().map(|i| &i.message).collect::<Vec<_>>()
        );
        checked += 1;
    }
    assert!(checked > 0, "expected to validate at least one fixture");
}
