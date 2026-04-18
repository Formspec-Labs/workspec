//! Advanced governance L3 — SMT verification, equity guardrail evaluation,
//! constraint-zone adaptive-action enumeration. External-dependency
//! features (SMT solver, triplestore) are traited out via
//! [`crate::adapters`]; default impls return well-shaped "inconclusive"
//! or windowed aggregate responses so clients see spec-correct shapes
//! regardless of the adapter wired in.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::{ApiError, ApiResult};
use crate::services::bundle_service::BundleService;
use crate::services::json_util::lookup_dotted;
use crate::storage::{InstanceQuery, StorageHandle};

// ── Verification (SMT stub) ────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyRequest {
    pub workflow_url: String,
    #[serde(default)]
    pub constraint_subset: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResponse {
    pub solver: SolverInfo,
    pub results: Vec<VerifyResult>,
    pub summary: VerifySummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverInfo {
    pub name: String,
    pub version: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResult {
    pub constraint_ref: String,
    /// `proven-safe` | `proven-unsafe` | `inconclusive`.
    pub result: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifySummary {
    pub total_constraints: u64,
    pub proven_safe: u64,
    pub proven_unsafe: u64,
    pub inconclusive: u64,
}

pub async fn verify(
    bundle: &Arc<BundleService>,
    req: &VerifyRequest,
) -> ApiResult<VerifyResponse> {
    let bundle_view = bundle
        .full_bundle(&req.workflow_url)
        .await
        .ok_or(ApiError::NotFound)?;
    let advanced = bundle_view
        .advanced
        .as_ref()
        .and_then(|v| v.get("verifiableConstraints"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let refs: Vec<String> = if let Some(subset) = &req.constraint_subset {
        subset.clone()
    } else {
        advanced
            .iter()
            .filter_map(|c| {
                c.get("id")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
            .collect()
    };
    let results: Vec<VerifyResult> = refs
        .iter()
        .map(|r| VerifyResult {
            constraint_ref: r.clone(),
            result: "inconclusive".into(),
            note: "SMT solver not configured; set WOS_SMT=z3 and rebuild with \
                   --features solver-z3 for real proofs"
                .into(),
        })
        .collect();
    let total = results.len() as u64;
    Ok(VerifyResponse {
        solver: SolverInfo {
            name: "noop".into(),
            version: "0.0.0".into(),
            note: "stub solver — every constraint reported inconclusive".into(),
        },
        results,
        summary: VerifySummary {
            total_constraints: total,
            proven_safe: 0,
            proven_unsafe: 0,
            inconclusive: total,
        },
    })
}

// ── Equity evaluation ──────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityEvaluateRequest {
    pub workflow_url: String,
    /// JSONPath-like dotted path into `caseState`, e.g. `applicant.zip`.
    pub group_by_path: String,
    /// Boolean outcome predicate; defaults to `status == "completed"`.
    #[serde(default)]
    pub outcome_predicate: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityReport {
    pub workflow_url: String,
    pub group_by_path: String,
    pub groups: Vec<GroupOutcome>,
    pub disparity: f64,
    pub alert: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupOutcome {
    pub group: String,
    pub total: u64,
    pub positive: u64,
    pub rate: f64,
}

pub async fn evaluate_equity(
    storage: &StorageHandle,
    req: &EquityEvaluateRequest,
) -> ApiResult<EquityReport> {
    let page = storage
        .list_instances(InstanceQuery {
            definition_url: Some(vec![req.workflow_url.clone()]),
            page: 1,
            page_size: 10_000,
            ..Default::default()
        })
        .await?;

    let mut by_group: std::collections::HashMap<String, (u64, u64)> =
        std::collections::HashMap::new();
    for row in &page.items {
        let case_state = row.case_state();
        let group = lookup_dotted(&case_state, &req.group_by_path).unwrap_or_else(|| "_".into());
        let positive = match &req.outcome_predicate {
            Some(_) => false, // expression eval stubbed — treat as false for now
            None => row.status == "completed",
        };
        let entry = by_group.entry(group).or_insert((0, 0));
        entry.0 += 1;
        if positive {
            entry.1 += 1;
        }
    }
    let groups: Vec<GroupOutcome> = by_group
        .into_iter()
        .map(|(group, (total, positive))| GroupOutcome {
            group,
            total,
            positive,
            rate: if total == 0 {
                0.0
            } else {
                positive as f64 / total as f64
            },
        })
        .collect();
    let (min, max) = groups
        .iter()
        .fold((f64::INFINITY, 0.0_f64), |(lo, hi), g| (lo.min(g.rate), hi.max(g.rate)));
    let disparity = if min.is_finite() { max - min } else { 0.0 };
    Ok(EquityReport {
        workflow_url: req.workflow_url.clone(),
        group_by_path: req.group_by_path.clone(),
        groups,
        disparity,
        alert: disparity > 0.2,
    })
}

// ── Constraint zones (DCR-style adaptive case management) ─────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintZoneView {
    pub id: String,
    pub description: Option<String>,
    pub activities: Vec<serde_json::Value>,
    pub relations: Vec<serde_json::Value>,
}

pub async fn list_zones(
    bundle: &Arc<BundleService>,
    workflow_url: &str,
) -> ApiResult<Vec<ConstraintZoneView>> {
    let bundle_view = bundle
        .full_bundle(workflow_url)
        .await
        .ok_or(ApiError::NotFound)?;
    let zones = bundle_view
        .advanced
        .as_ref()
        .and_then(|v| v.get("constraintZones"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(zones
        .into_iter()
        .filter_map(|z| {
            Some(ConstraintZoneView {
                id: z.get("id")?.as_str()?.to_string(),
                description: z
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                activities: z
                    .get("activities")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default(),
                relations: z
                    .get("relations")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default(),
            })
        })
        .collect())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidActionsResponse {
    pub zone_id: String,
    pub instance_id: String,
    pub valid_actions: Vec<serde_json::Value>,
    pub note: String,
}

pub async fn valid_actions_in_zone(
    bundle: &Arc<BundleService>,
    instance_id: &str,
    zone_id: &str,
    workflow_url: &str,
) -> ApiResult<ValidActionsResponse> {
    let zones = list_zones(bundle, workflow_url).await?;
    let zone = zones
        .into_iter()
        .find(|z| z.id == zone_id)
        .ok_or(ApiError::NotFound)?;
    // Stub: return the zone's declared activities as "valid next actions."
    // Real evaluation computes the DCR marking against stored provenance
    // and filters against executed / pending / included predicates.
    Ok(ValidActionsResponse {
        zone_id: zone.id,
        instance_id: instance_id.to_string(),
        valid_actions: zone.activities,
        note: "zone evaluation is stubbed — all declared activities listed".into(),
    })
}
