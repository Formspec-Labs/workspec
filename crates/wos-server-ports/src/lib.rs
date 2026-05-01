//! `wos-server-ports` — trait crate for the wos-server adapter boundary.
//!
//! Defines the contracts that concrete adapters (SQLite, Postgres, JWT, Keycloak,
//! Restate, etc.) implement. **No impls live here** — only traits, row types,
//! error types, and associated data structures. A crate that does not list
//! `wos-server-sqlite` in its deps cannot import `SqliteStorage`; Cargo enforces
//! what conventions cannot.
//!
//! Adapter authoring guide: see `crates/wos-server-ports/AUTHORING.md` (WS-081).

pub mod audit;
pub mod auth;
pub mod runtime;
pub mod storage;

pub use audit::{AuditError, AuditResult, AuditSink, ExportEnvelope, NoopAuditSink};
pub use auth::{AuthContext, AuthError, AuthProvider, AuthResult, AuthUser, TokenPair};
pub use runtime::{
    BundleResolverPort, ProvenancePort, RuntimeAdapterError, RuntimeOps, RuntimeResult, SeamAccess,
    TimerCoord,
};
pub use storage::{
    AgentRow, DelegationRow, IdentityFactRow, InboundCloudEventRow, InstanceMutator, InstanceQuery,
    InstanceRow, IntakeRecordRow, KernelRow, LIST_INSTANCES_PAGE_SIZE_MAX,
    ListInstancesPageSizeMax, Page, ProvenanceRow, SessionRow, Storage, StorageError,
    StorageHandle, StorageResult, UserRow,
};
