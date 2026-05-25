// Rust guideline compliant 2026-05-15
//! G-7 WOS consumer: walks the shared cross-stack corpus via
//! `integrity-bundle-fixtures` (fixture bytes at `formspec/tests/fixtures/cross-stack/`).

use std::path::{Path, PathBuf};

use integrity_bundle_fixtures::{
    FixtureBundle, all_manifest_schema_paths, discover_bundles, validate_manifest_schema,
};

const EXPECTED_BUNDLE_IDS: [&str; 8] = ["001", "002", "003", "004", "005", "006", "007", "008"];

fn cross_stack_root() -> PathBuf {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../formspec/tests/fixtures/cross-stack");
    assert!(
        root.join("manifest.schema.json").is_file(),
        "cross-stack fixture root missing at {} (repo layout may have moved)",
        root.display()
    );
    root
}

fn bundle_is_byte_populated(bundle: &FixtureBundle) -> bool {
    bundle.dir.join("formspec-response.json").is_file()
}

#[test]
fn given_cross_stack_fixture_root_when_discovering_bundles_then_expected_ids_are_present() {
    let bundles = discover_bundles(cross_stack_root()).expect("discover cross-stack bundles");
    assert_eq!(
        bundles.len(),
        EXPECTED_BUNDLE_IDS.len(),
        "expected exactly {} cross-stack fixture bundles",
        EXPECTED_BUNDLE_IDS.len()
    );
    let ids: Vec<_> = bundles.iter().map(|bundle| bundle.id.as_str()).collect();
    assert_eq!(ids, EXPECTED_BUNDLE_IDS);
}

#[test]
fn given_each_cross_stack_manifest_when_validating_schema_then_validation_succeeds() {
    let root = cross_stack_root();
    let manifest_paths = all_manifest_schema_paths(root.to_str().unwrap()).expect("manifest paths");
    assert_eq!(
        manifest_paths.len(),
        EXPECTED_BUNDLE_IDS.len(),
        "expected {} manifest.toml files",
        EXPECTED_BUNDLE_IDS.len()
    );
    for manifest_path in manifest_paths {
        validate_manifest_schema(&manifest_path).unwrap_or_else(|error| {
            panic!(
                "manifest {:?} failed schema validation: {error}",
                manifest_path
            );
        });
    }
}

#[test]
fn given_byte_populated_bundle_when_wos_provenance_required_then_cbor_exists_on_disk() {
    let bundles = discover_bundles(cross_stack_root()).expect("discover cross-stack bundles");
    for bundle in &bundles {
        if !bundle_is_byte_populated(bundle) {
            continue;
        }
        let required = &bundle.manifest.required_files;
        if required.wos_provenance {
            let path = bundle.dir.join("wos-provenance.cbor");
            assert!(
                path.is_file(),
                "bundle {} requires wos-provenance.cbor at {}",
                bundle.id,
                path.display()
            );
        }
        if let Some(wos) = bundle.manifest.expected_outcomes.wos.as_ref() {
            if wos.present == Some(true) {
                assert!(
                    required.wos_provenance,
                    "bundle {} declares wos present but wos_provenance is not required",
                    bundle.id
                );
            }
        }
    }
}

#[test]
fn given_skeleton_bundle_when_no_bytes_then_consumer_skips_artifact_verify() {
    let bundles = discover_bundles(cross_stack_root()).expect("discover cross-stack bundles");
    for bundle in &bundles {
        if bundle_is_byte_populated(bundle) {
            continue;
        }
        assert!(
            !bundle.dir.join("wos-provenance.cbor").is_file(),
            "skeleton bundle {} must not ship wos-provenance.cbor until bytes land",
            bundle.id
        );
    }
}
