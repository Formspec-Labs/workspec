//! `ProjectRegistry` — stub skeleton for in-memory WOS project storage.
//!
//! This module holds the UUID-keyed map of open projects. The full implementation
//! (backed by `wos-authoring::WosProject`) lands in Task 3. For Tasks 1–2 the
//! registry is an empty shell: it exists so the tool handler signature
//! `(args: Value) -> Result<Value, ToolError>` is stable.

/// Registry of open WOS projects, keyed by project UUID strings.
///
/// Task 3 will fill this with real `WosProject` entries backed by
/// `wos-authoring`. For now it is an empty placeholder that stabilises
/// the tool-handler signature across Tasks 1–2.
#[derive(Default)]
pub struct ProjectRegistry {
    // Populated in Task 3 when wos-authoring::WosProject is available.
}

impl ProjectRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }
}
