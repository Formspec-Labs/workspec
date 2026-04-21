//! `ProjectRegistry` — in-memory registry of open WOS projects.
//!
//! Keyed by `Uuid` strings. Enforces a maximum of 20 simultaneously open
//! projects to bound memory use during a single MCP session.

use std::collections::HashMap;

use uuid::Uuid;
use wos_authoring::WosProject;

use crate::errors::ToolError;

/// Maximum number of simultaneously open projects per server session.
const MAX_PROJECTS: usize = 20;

/// In-memory registry of open WOS projects, keyed by UUID.
///
/// The registry is the sole owner of all live `WosProject` instances.
/// Tool handlers receive a `&mut ProjectRegistry` and call the lookup
/// helpers to get mutable access; they never store or transfer ownership.
pub struct ProjectRegistry {
    entries: HashMap<Uuid, WosProject>,
}

impl ProjectRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Insert a new project, assign a UUID, and return that UUID.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::TooManyProjects` when the registry already holds
    /// `MAX_PROJECTS` entries. The caller's project is NOT inserted.
    pub fn insert(&mut self, project: WosProject) -> Result<Uuid, ToolError> {
        if self.entries.len() >= MAX_PROJECTS {
            return Err(ToolError::TooManyProjects);
        }
        let id = Uuid::new_v4();
        self.entries.insert(id, project);
        Ok(id)
    }

    /// Look up a project by UUID string.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::ProjectNotFound` when the id is unknown or not a
    /// valid UUID.
    pub fn get(&self, project_id: &str) -> Result<&WosProject, ToolError> {
        let uuid = parse_uuid(project_id)?;
        self.entries
            .get(&uuid)
            .ok_or_else(|| ToolError::ProjectNotFound(project_id.to_string()))
    }

    /// Look up a project mutably by UUID string.
    ///
    /// # Errors
    ///
    /// Returns `ToolError::ProjectNotFound` when the id is unknown or not a
    /// valid UUID.
    pub fn get_mut(&mut self, project_id: &str) -> Result<&mut WosProject, ToolError> {
        let uuid = parse_uuid(project_id)?;
        self.entries
            .get_mut(&uuid)
            .ok_or_else(|| ToolError::ProjectNotFound(project_id.to_string()))
    }

    /// Remove a project from the registry. No-op if the id is unknown.
    pub fn close(&mut self, project_id: &str) {
        if let Ok(uuid) = parse_uuid(project_id) {
            self.entries.remove(&uuid);
        }
    }

    /// List all open project UUIDs.
    pub fn list(&self) -> Vec<Uuid> {
        self.entries.keys().copied().collect()
    }
}

impl Default for ProjectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a UUID string into a `Uuid`, mapping parse errors to `ProjectNotFound`.
///
/// Using `ProjectNotFound` (not `InvalidArguments`) here keeps the error
/// message coherent for callers: an unparseable id means the project cannot
/// be located, which is the same observable outcome as a valid-but-unknown id.
fn parse_uuid(s: &str) -> Result<Uuid, ToolError> {
    Uuid::parse_str(s).map_err(|_| ToolError::ProjectNotFound(s.to_string()))
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_project() -> WosProject {
        WosProject::new_kernel()
    }

    #[test]
    fn insert_and_retrieve() {
        let mut reg = ProjectRegistry::new();
        let project = make_project();
        let id = reg.insert(project).expect("insert must succeed");
        let id_str = id.to_string();

        assert!(reg.get(&id_str).is_ok());
        assert!(reg.get_mut(&id_str).is_ok());
    }

    #[test]
    fn close_removes_project() {
        let mut reg = ProjectRegistry::new();
        let id = reg.insert(make_project()).unwrap();
        reg.close(&id.to_string());
        assert!(matches!(
            reg.get(&id.to_string()),
            Err(ToolError::ProjectNotFound(_))
        ));
    }

    #[test]
    fn too_many_projects_returns_error() {
        let mut reg = ProjectRegistry::new();
        for _ in 0..20 {
            reg.insert(make_project())
                .expect("must succeed for first 20");
        }
        let err = reg.insert(make_project()).unwrap_err();
        assert!(matches!(err, ToolError::TooManyProjects));
    }

    #[test]
    fn unknown_project_id_returns_not_found() {
        let reg = ProjectRegistry::new();
        let err = reg.get("00000000-0000-0000-0000-000000000000").unwrap_err();
        assert!(matches!(err, ToolError::ProjectNotFound(_)));
    }

    #[test]
    fn invalid_uuid_string_returns_not_found() {
        let reg = ProjectRegistry::new();
        let err = reg.get("not-a-uuid").unwrap_err();
        assert!(matches!(err, ToolError::ProjectNotFound(_)));
    }

    #[test]
    fn list_returns_all_ids() {
        let mut reg = ProjectRegistry::new();
        let id1 = reg.insert(make_project()).unwrap();
        let id2 = reg.insert(make_project()).unwrap();

        let listed = reg.list();
        assert_eq!(listed.len(), 2);
        assert!(listed.contains(&id1));
        assert!(listed.contains(&id2));
    }
}
