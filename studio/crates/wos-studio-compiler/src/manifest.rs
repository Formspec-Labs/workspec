// Rust guideline compliant 2026-05-02

//! Compile manifest — the reproducibility primitive.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The compile manifest per `SA-MUST-cmp-050..062`. Records every input
/// the compile consumed plus version / pin information so an external
/// party with a workspace export bundle and the recorded compiler
/// version can reproduce the artifact byte-for-byte.
///
/// The `manifestHash` field (`SA-MUST-cmp-073`) is a sha256 over the
/// JCS-canonicalized manifest minus `manifestHash` and `compiledAt`
/// itself — it binds the ApprovalPackage to a specific compile. Use
/// [`CompileManifest::compute_hash`] to populate it after all other
/// fields are set.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileManifest {
    pub compiler_version: String,
    pub schema_version: String,
    pub wos_version_pin: Option<String>,
    pub workspace_id: String,
    pub workflow_intent_id: String,
    pub workflow_intent_version: String,

    /// Sorted by id for stable comparison.
    pub source_versions_consumed: Vec<String>,
    /// Sorted by id for stable comparison.
    pub policy_objects_consumed: Vec<String>,
    /// Sorted by id for stable comparison.
    pub mappings_consumed: Vec<String>,
    /// Sorted by id for stable comparison.
    pub bindings_consumed: Vec<String>,
    /// Sorted by id for stable comparison.
    pub scenarios_consumed: Vec<String>,

    /// Embedded blocks the artifact contains. `SA-MUST-cmp-006`.
    pub embedded_blocks_emitted: Vec<String>,

    /// Compile-time stamp; NOT part of artifact body and NOT included
    /// in `manifest_hash` (excluding it makes hashes stable across
    /// repeated identical compiles per `SA-MUST-cmp-001`).
    pub compiled_at: Option<String>,

    /// Reviewer / agent who triggered the compile.
    pub compiled_by: Option<String>,

    /// SHA256 over the JCS-canonicalized manifest minus `manifestHash`
    /// and `compiledAt`. Set by [`Self::compute_hash`] in phase 8.
    /// Empty until then.
    pub manifest_hash: String,

    /// Free-form extension surface for future fields.
    #[serde(flatten)]
    pub extensions: IndexMap<String, Value>,
}

impl CompileManifest {
    /// Construct a manifest with sane defaults; callers populate
    /// id/version fields after.
    pub fn empty(workspace_id: String, workflow_intent_id: String) -> Self {
        Self {
            compiler_version: crate::COMPILER_VERSION.to_string(),
            schema_version: crate::SCHEMA_VERSION.to_string(),
            wos_version_pin: None,
            workspace_id,
            workflow_intent_id,
            workflow_intent_version: "0.1.0".to_string(),
            source_versions_consumed: Vec::new(),
            policy_objects_consumed: Vec::new(),
            mappings_consumed: Vec::new(),
            bindings_consumed: Vec::new(),
            scenarios_consumed: Vec::new(),
            embedded_blocks_emitted: Vec::new(),
            compiled_at: None,
            compiled_by: None,
            manifest_hash: String::new(),
            extensions: IndexMap::new(),
        }
    }

    /// Compute the manifest's content hash and store it in
    /// `self.manifest_hash`. Idempotent — overwrites any prior value.
    /// Excludes `manifest_hash` itself + `compiled_at` from the hash
    /// input so two compiles of identical input at different timestamps
    /// produce the same hash.
    pub fn compute_hash(&mut self) {
        self.manifest_hash = String::new();
        let saved_compiled_at = self.compiled_at.take();
        let canonical = serde_json_canonicalizer::to_string(self)
            .unwrap_or_else(|_| String::new());
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        let digest = hasher.finalize();
        self.manifest_hash = format!("sha256:{:x}", digest);
        self.compiled_at = saved_compiled_at;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_hash_is_deterministic_across_compiled_at() {
        let mut a = CompileManifest::empty("ws-1".into(), "wf-1".into());
        a.workflow_intent_version = "0.1.0".into();
        a.compiled_at = Some("2026-05-02T12:00:00Z".into());
        a.compute_hash();

        let mut b = CompileManifest::empty("ws-1".into(), "wf-1".into());
        b.workflow_intent_version = "0.1.0".into();
        b.compiled_at = Some("2099-12-31T23:59:59Z".into());
        b.compute_hash();

        assert_eq!(
            a.manifest_hash, b.manifest_hash,
            "compiled_at MUST NOT influence manifest_hash"
        );
        assert!(a.manifest_hash.starts_with("sha256:"));
    }

    #[test]
    fn compute_hash_changes_on_input_change() {
        let mut a = CompileManifest::empty("ws-1".into(), "wf-1".into());
        a.compute_hash();
        let mut b = CompileManifest::empty("ws-1".into(), "wf-1".into());
        b.policy_objects_consumed.push("pol-x".into());
        b.compute_hash();
        assert_ne!(a.manifest_hash, b.manifest_hash);
    }
}
