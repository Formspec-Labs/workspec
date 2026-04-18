// Rust guideline compliant 2026-02-21

//! Intent-driven authoring API for WOS Kernel Documents.
//!
//! # Layer overview
//!
//! ```text
//! WosProject (facade, Task 6)
//!      |
//! RawWosProject + IWosProjectCore  (this task)
//!      |
//! KernelDocument (wos-core)
//! ```
//!
//! Consumers use `RawWosProject` + `IWosProjectCore` directly today; the
//! `WosProject` facade (Task 6) will wrap them with the 28 intent-driven
//! helper methods that `wos-mcp` tool handlers call.

pub mod command;
pub mod diagnostics;
pub mod project;
pub mod raw;

pub use command::Command;
pub use diagnostics::AuthoringDiagnostic;
pub use project::WosProject;
pub use raw::{IWosProjectCore, RawWosProject};
