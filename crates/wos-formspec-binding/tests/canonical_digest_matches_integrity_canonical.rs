// Integration test for fs-7md4: the in-binding canonical-digest path MUST
// agree byte-for-byte with `integrity_canonical::build_signed_payload` over a
// real bundle response.
//
// Closes the regression seam left when the binding's `compute_formspec_signed_
// payload_digest` shim was deleted (commit removing the 14-line `lib.rs:508
// shim`). The shim's only behaviour was a thin re-export of `build_signed_
// payload`; if a future refactor reintroduces a parallel digest path, this
// test fails loudly the moment the bundle response and integrity-canonical
// disagree.

use std::path::PathBuf;

use integrity_canonical::{DigestAlgorithm, build_signed_payload};

/// Resolve bundle 001's `formspec-response.json` from this crate's manifest
/// dir. Bundle 001 lives in the formspec sibling repo:
/// `formspec/tests/fixtures/cross-stack/001-standalone-formspec-verified/`.
fn bundle_001_response_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent (crates/)")
        .parent()
        .expect("crates dir has a parent (work-spec/)")
        .parent()
        .expect("work-spec dir has a parent (stack root)")
        .join("formspec")
        .join("tests")
        .join("fixtures")
        .join("cross-stack")
        .join("001-standalone-formspec-verified")
        .join("formspec-response.json")
}

#[test]
fn canonical_digest_matches_integrity_canonical_recomputation_for_bundle_001() {
    let path = bundle_001_response_path();
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("read bundle 001 response at {path:?}: {error}"));
    let response: serde_json::Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parse bundle 001 response at {path:?}: {error}"));

    let in_file_digest = response
        .get("authoredSignatures")
        .and_then(|s| s.get(0))
        .and_then(|s| s.get("signedPayload"))
        .and_then(|sp| sp.get("digest"))
        .and_then(|d| d.as_str())
        .expect("bundle 001 response.authoredSignatures[0].signedPayload.digest is present");

    let in_file_algorithm = response
        .get("authoredSignatures")
        .and_then(|s| s.get(0))
        .and_then(|s| s.get("signedPayload"))
        .and_then(|sp| sp.get("digestAlgorithm"))
        .and_then(|a| a.as_str())
        .expect("bundle 001 response.authoredSignatures[0].signedPayload.digestAlgorithm is present");
    assert_eq!(
        in_file_algorithm, "sha-256",
        "bundle 001 fixture pins sha-256; regenerating with a different algorithm requires \
         updating this test"
    );

    let recomputed = build_signed_payload(&response, DigestAlgorithm::Sha256)
        .expect("integrity-canonical build_signed_payload succeeds for bundle 001 response");

    assert_eq!(
        recomputed.digest, in_file_digest,
        "integrity-canonical recomputed digest must equal the in-file signedPayload.digest; \
         drift between wos-formspec-binding's canonical path and integrity-canonical is the \
         regression this test guards (fs-7md4)"
    );
}
