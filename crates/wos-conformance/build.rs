// Rust guideline compliant 2026-02-21

//! Captures scoped native replay source provenance at compilation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use integrity_canonical::{DigestAlgorithm, canonical_json_bytes, compute_digest};
use serde_json::json;

// This scope is deliberately explicit and reported. It is not a complete
// compiler input graph or binary fingerprint. Used Formspec/FEL sibling sources
// have a separate declared fingerprint below.
const NATIVE_CRATES: &[&str] = &[
    "wos-conformance",
    "wos-runtime",
    "wos-core",
    "wos-formspec-binding",
    "wos-events",
    "wos-lint",
];

fn hash(bytes: &[u8]) -> String {
    compute_digest(bytes, DigestAlgorithm::Sha256)
}

fn collect(
    path: &Path,
    root: &Path,
    files: &mut BTreeMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed={}", path.display());
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            collect(&entry?.path(), root, files)?;
        }
    } else {
        files.insert(
            path.strip_prefix(root)?.to_string_lossy().into_owned(),
            hash(&std::fs::read(path)?),
        );
    }
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .ok_or("missing native workspace root")?;
    let mut files = BTreeMap::new();
    for name in NATIVE_CRATES {
        let crate_root = root.join("crates").join(name);
        collect(&crate_root.join("src"), root, &mut files)?;
        collect(&crate_root.join("Cargo.toml"), root, &mut files)?;
        if crate_root.join("build.rs").exists() {
            collect(&crate_root.join("build.rs"), root, &mut files)?;
        }
    }
    collect(&root.join("Cargo.toml"), root, &mut files)?;
    collect(&root.join("Cargo.lock"), root, &mut files)?;
    // Embedded owner schemas participate in validation behavior, including the
    // reference policy and replay resources loaded through offline resolvers.
    collect(&root.join("schemas"), root, &mut files)?;
    let stack_root = root.parent().ok_or("missing stack root")?;
    let mut external_files = BTreeMap::new();
    let mut external_revisions = BTreeMap::new();
    for relative in [
        "formspec/crates/formspec-core",
        "formspec/crates/formspec-eval",
        "formspec/crates/formspec-lint",
        "fel-core",
    ] {
        let selected = stack_root.join(relative);
        collect(&selected.join("src"), stack_root, &mut external_files)?;
        collect(
            &selected.join("Cargo.toml"),
            stack_root,
            &mut external_files,
        )?;
        if selected.join("build.rs").exists() {
            collect(&selected.join("build.rs"), stack_root, &mut external_files)?;
        }
        external_revisions.insert(relative, git(&selected, &["rev-parse", "HEAD"]));
    }
    for relative in [
        "formspec/schemas",
        "formspec/Cargo.toml",
        "formspec/Cargo.lock",
        "fel-core/Cargo.lock",
    ] {
        let selected = stack_root.join(relative);
        if selected.exists() {
            collect(&selected, stack_root, &mut external_files)?;
        }
    }
    for reference in ["HEAD", "index", "packed-refs"] {
        if let Some(path) = git(root, &["rev-parse", "--git-path", reference]) {
            let path = PathBuf::from(path);
            println!(
                "cargo:rerun-if-changed={}",
                if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                }
                .display()
            );
        }
    }
    if let Some(reference) = git(root, &["symbolic-ref", "-q", "HEAD"]) {
        if let Some(path) = git(root, &["rev-parse", "--git-path", &reference]) {
            let path = PathBuf::from(path);
            println!(
                "cargo:rerun-if-changed={}",
                if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                }
                .display()
            );
        }
    }
    let features: BTreeMap<_, _> = std::env::vars()
        .filter(|(name, _)| name.starts_with("CARGO_FEATURE_"))
        .collect();
    let identity = json!({
        "nativeGitRevision":git(root, &["rev-parse", "HEAD"]),
        "nativeReplaySourceSnapshot":hash(&canonical_json_bytes(&serde_json::to_value(&files)?)?),
        "sourceScope":{"crates":NATIVE_CRATES,"paths":["src/**", "Cargo.toml", "existing build.rs"],"workspaceInputs":["Cargo.toml", "Cargo.lock", "schemas/**"]},
        "lockfileSha256":hash(&std::fs::read(root.join("Cargo.lock"))?),
        "target":std::env::var("TARGET").ok(),"profile":std::env::var("PROFILE").ok(),"features":features,
        "toolchain":null,"completeCompiledGraph":false,
        "externalPathDependencySourceFingerprints":{"sourceSha256":hash(&canonical_json_bytes(&serde_json::to_value(&external_files)?)?),"revisions":external_revisions,"files":external_files}
    });
    println!("cargo:rustc-env=WOS_REPLAY_BUILD_IDENTITY={identity}");
    Ok(())
}
