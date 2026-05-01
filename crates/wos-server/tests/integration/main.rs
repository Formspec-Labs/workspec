//! Single integration-test binary for `wos-server`.
//!
//! Each former `tests/<name>.rs` is now `tests/integration/<name>.rs` and
//! declared as a module below. Consolidating into one binary cuts link time
//! and `target/` footprint by ~30× versus the prior file-per-binary layout.

#![allow(dead_code, unused_imports)]

// Shared helpers (formerly tests/common/mod.rs and tests/http_coverage_shared/*.rs).
mod common;
mod harness;
mod slice_b;

// Test modules (one per former tests/<name>.rs binary).
mod adapter_scaffolds;
mod audit_sink_consistency;
mod auth_jwt;
mod bundle_validation;
mod equity_outcome_predicate;
mod http_adverse_notice;
mod http_api_surface_expansion;
mod http_change_password;
mod http_coverage_backfill;
mod http_coverage_slice_b;
mod http_coverage_slice_c;
mod http_event_submit_drain;
mod http_governance_delegations;
mod http_jwt_logout;
mod http_policy_resolve_get;
mod http_smoke;
mod http_tasks_lifecycle;
mod http_tenant_passthrough;
mod json_util;
mod provenance_chain;
mod provenance_spec_shape;
mod runtime_lifecycle;
mod runtime_store_persistence;
mod session_sweep;
mod signature_affirmations;
mod storage_sqlite;
mod timer_list_pagination;
mod timer_poll_e2e;
mod ws_003_auth_breadth;
mod ws_di_seams;
mod ws_spec_gaps_2;
mod ws_top_roi;
