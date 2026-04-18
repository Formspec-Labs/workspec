//! Thin wrap around `wos-conformance`. Exposes `run_fixture` and
//! `verify_processor_manifest` so CI pipelines and external consumers
//! can exercise the server as a spec conformance target.

use serde::{Deserialize, Serialize};
use wos_conformance::{ConformanceResult, run_fixture};

/// `POST /api/conformance/fixture` request body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureRequest {
    /// The fixture JSON (inline). Paths inside `documents` resolve
    /// against `baseDir`.
    pub fixture: serde_json::Value,
    /// Base directory for resolving relative document paths.
    /// Defaults to the server's `fixtures` dir.
    #[serde(default)]
    pub base_dir: Option<String>,
}

/// `POST /api/conformance/fixture` response body.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureResponse {
    pub passed: bool,
    pub failures: Vec<String>,
    pub transition_count: usize,
    pub provenance_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_used: Option<String>,
}

impl From<ConformanceResult> for FixtureResponse {
    fn from(r: ConformanceResult) -> Self {
        Self {
            passed: r.passed,
            failures: r.failures,
            transition_count: r.transitions.len(),
            provenance_count: r.provenance.len(),
            binding_used: r.binding_used,
        }
    }
}

pub fn run(req: &FixtureRequest, default_base_dir: &str) -> crate::ApiResult<FixtureResponse> {
    let fixture_json = serde_json::to_string(&req.fixture).map_err(|e| {
        crate::ApiError::BadRequest(format!("fixture must be a valid JSON document: {e}"))
    })?;
    let base_dir = req
        .base_dir
        .clone()
        .unwrap_or_else(|| default_base_dir.to_string());
    let result = run_fixture(&fixture_json, &base_dir)
        .map_err(|e| crate::ApiError::BadRequest(format!("conformance run failed: {e}")))?;
    Ok(result.into())
}
