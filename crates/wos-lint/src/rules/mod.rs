// Rust guideline compliant 2026-02-21

//! Lint rule modules organized by verification tier.

pub mod continuous_mode;
pub mod fel_analysis;
pub mod registry;
pub mod schema_doc;
pub mod tier1;
pub mod tier2;

pub use registry::{Graduation, RuleMetadata, Tier, all_lint_rules};
