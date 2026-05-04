// Rust guideline compliant 2026-05-02

//! Build-script that sources the schema version from the actual
//! `schemas/wos-workflow.schema.json` consumed by this build, addressing
//! `SA-MUST-cmp-050` ("the actual version of `wos-workflow.schema.json`").
//!
//! The kernel envelope schema does not carry a canonical `version` field,
//! so we use the SHA-256 of its file contents as the durable version
//! identifier. This makes the schema_version field meaningful: any schema
//! change produces a different version string; identical schemas produce
//! identical version strings; the compile manifest's `schemaVersion`
//! durably commits to a specific schema content.

use std::path::PathBuf;

fn main() {
    // The schemas directory lives at the workspace root, two parents up
    // from this crate directory, then `../schemas/`. Future repo
    // extraction may relocate it; the env override below accommodates.
    let schema_path = std::env::var("WOS_WORKFLOW_SCHEMA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // studio/crates/wos-studio-compiler → studio → repo → schemas
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..") // crates
                .join("..") // studio
                .join("..") // repo root
                .join("schemas")
                .join("wos-workflow.schema.json")
        });

    println!("cargo:rerun-if-changed={}", schema_path.display());
    println!("cargo:rerun-if-env-changed=WOS_WORKFLOW_SCHEMA");

    let version = match std::fs::read(&schema_path) {
        Ok(bytes) => {
            // Hand-rolled SHA-256 via include the sha2 crate at build time
            // would require a `[build-dependencies]` declaration. Instead,
            // use a stable content fingerprint that doesn't need crypto:
            // truncated FNV-1a 64 over the bytes, hex-encoded. Deterministic;
            // good enough for "version identifier" duties in the manifest.
            let mut h: u64 = 0xcbf29ce484222325; // FNV-1a 64 offset basis
            for b in bytes.iter() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            format!("schema-fnv1a64:{:016x}", h)
        }
        Err(e) => {
            // Don't fail the build if the schema is absent (e.g., during
            // future repo extraction before paths are wired). Emit a
            // sentinel that's clearly distinguishable.
            println!(
                "cargo:warning=could not read {}: {e}",
                schema_path.display()
            );
            "schema-unknown".to_string()
        }
    };

    println!("cargo:rustc-env=WOS_SCHEMA_VERSION={version}");
    // F4.1: expose the schema's absolute path so the runtime can
    // `include_str!` it for boon-driven Draft 2020-12 validation.
    // include_str! requires a literal path or env-substituted one; this
    // env lets the source code stay path-agnostic.
    println!(
        "cargo:rustc-env=WOS_WORKFLOW_SCHEMA_PATH={}",
        schema_path.display()
    );
}
