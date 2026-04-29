//! `wos-server-ports` — trait crate for the wos-server adapter boundary.
//!
//! Defines the contracts that concrete adapters (SQLite, Postgres, JWT, Keycloak,
//! Restate, etc.) implement. **No impls live here** — only traits, row types,
//! error types, and associated data structures. A crate that does not list
//! `wos-server-sqlite` in its deps cannot import `SqliteStorage`; Cargo enforces
//! what conventions cannot.
//!
//! Adapter authoring guide: see `crates/wos-server-ports/AUTHORING.md` (WS-081).

pub mod auth;
pub mod storage;

pub use auth::{AuthContext, AuthError, AuthProvider, AuthResult, AuthUser, TokenPair};
pub use storage::{
    AgentRow, DelegationRow, IdentityFactRow, InboundCloudEventRow, InstanceMutator, InstanceQuery,
    InstanceRow, IntakeRecordRow, KernelRow, ListInstancesPageSizeMax, Page, ProvenanceRow,
    SessionRow, Storage, StorageError, StorageHandle, StorageResult, UserRow,
    LIST_INSTANCES_PAGE_SIZE_MAX,
};
