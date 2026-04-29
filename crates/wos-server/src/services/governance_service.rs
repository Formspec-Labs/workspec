//! Project stored kernel documents into studio-facing governance views.

use std::sync::Arc;

use crate::domain::{
    AdverseDecisionNoticeView, AgentCapabilityView, AgentView, CalendarEventView,
    DelegationEntryView, DeonticConstraintView, EquityCategoryView, EquityConfigView,
    EquityDisparityMethodView, EquityRemediationTriggerView, EquityReportingScheduleView,
    OverrideAuthorityView, PipelineAssertionView, PipelineStageView, PipelineView,
    PolicyVersionView, QualityControlsView, ResolvedPolicyView, ReviewSamplingView,
    SeparationOfDutiesView, ServiceHealthView, SolverView, VerificationCounterexampleView,
    VerificationReportView, VerificationResultView, VerificationSummaryView,
};
use crate::error::{ApiError, ApiResult};
use crate::storage::StorageHandle;

use super::bundle_service::BundleService;
use super::notifications_service::{find_template, interpolate};

pub struct GovernanceService {
    storage: StorageHandle,
    bundle: Arc<BundleService>,
}

impl GovernanceService {
    pub fn new(storage: StorageHandle, bundle: Arc<BundleService>) -> Self {
        Self { storage, bundle }
    }

    pub async fn agents(&self, workflow_url: &str) -> Vec<AgentView> {
        let bundle = match self.bundle.full_bundle(workflow_url).await {
            Some(b) => b,
            None => return Vec::new(),
        };
        let ai = match bundle.ai.as_ref() {
            Some(v) => v,
            None => return Vec::new(),
        };
        ai.get("agents")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().map(map_agent).collect())
            .unwrap_or_default()
    }

    pub async fn deontic_constraints(&self, workflow_url: &str) -> Vec<DeonticConstraintView> {
        let Some(bundle) = self.bundle.full_bundle(workflow_url).await else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if let Some(gov) = &bundle.governance {
            if let Some(arr) = gov.get("deontic").and_then(|v| v.as_array()) {
                for v in arr {
                    out.push(map_deontic(v));
                }
            }
        }
        if let Some(ai) = &bundle.ai {
            if let Some(arr) = ai.get("deontic").and_then(|v| v.as_array()) {
                for v in arr {
                    out.push(map_deontic(v));
                }
            }
        }
        out
    }

    pub async fn quality_controls(&self, workflow_url: &str) -> Option<QualityControlsView> {
        let bundle = self.bundle.full_bundle(workflow_url).await?;
        let gov = bundle.governance.as_ref()?;
        let qc = gov.get("qualityControls")?;
        Some(QualityControlsView {
            review_sampling: qc.get("reviewSampling").map(|v| ReviewSamplingView {
                rate: v.get("rate").and_then(|x| x.as_f64()).unwrap_or(0.0),
                method: s(v, "method"),
                scope: s(v, "scope"),
            }),
            separation_of_duties: qc
                .get("separationOfDuties")
                .map(|v| SeparationOfDutiesView {
                    scope: s(v, "scope").unwrap_or_default(),
                    exclude_roles: v
                        .get("excludeRoles")
                        .and_then(|a| a.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect()
                        }),
                }),
            override_authority: qc.get("overrideAuthority").map(|v| OverrideAuthorityView {
                require_structured_rationale: v
                    .get("requireStructuredRationale")
                    .and_then(|x| x.as_bool()),
                require_authority_verification: v
                    .get("requireAuthorityVerification")
                    .and_then(|x| x.as_bool()),
                require_supporting_evidence: v
                    .get("requireSupportingEvidence")
                    .and_then(|x| x.as_bool()),
            }),
        })
    }

    pub async fn pipelines(&self, workflow_url: &str) -> Vec<PipelineView> {
        let Some(bundle) = self.bundle.full_bundle(workflow_url).await else {
            return Vec::new();
        };
        let Some(gov) = &bundle.governance else { return Vec::new() };
        gov.get("pipelines")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().map(map_pipeline).collect())
            .unwrap_or_default()
    }

    pub async fn verification_report(&self, workflow_url: &str) -> Option<VerificationReportView> {
        let bundle = self.bundle.full_bundle(workflow_url).await?;
        let report = bundle.verification_report.as_ref()?;
        let solver_v = report.get("solver")?;
        Some(VerificationReportView {
            solver: SolverView {
                name: s(solver_v, "name").unwrap_or_default(),
                version: s(solver_v, "version").unwrap_or_default(),
                timeout: s(solver_v, "timeout"),
            },
            results: report
                .get("results")
                .and_then(|a| a.as_array())
                .map(|a| a.iter().map(map_verification_result).collect())
                .unwrap_or_default(),
            summary: report.get("summary").map(|v| VerificationSummaryView {
                total_constraints: u64_opt(v, "totalConstraints"),
                proven_safe: u64_opt(v, "provenSafe"),
                proven_unsafe: u64_opt(v, "provenUnsafe"),
                inconclusive: u64_opt(v, "inconclusive"),
                total_solver_time_ms: u64_opt(v, "totalSolverTimeMs"),
            }),
        })
    }

    pub async fn equity_config(&self, workflow_url: &str) -> Option<EquityConfigView> {
        let bundle = self.bundle.full_bundle(workflow_url).await?;
        let eq = bundle.equity.as_ref()?;
        Some(EquityConfigView {
            protected_categories: eq
                .get("protectedCategories")
                .and_then(|a| a.as_array())
                .map(|a| a.iter().map(map_equity_category).collect())
                .unwrap_or_default(),
            disparity_methods: eq
                .get("disparityMethods")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .map(|v| EquityDisparityMethodView {
                            id: s(v, "id").unwrap_or_default(),
                            method: s(v, "method").unwrap_or_default(),
                            description: s(v, "description"),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            reporting_schedule: eq
                .get("reportingSchedule")
                .map(|v| EquityReportingScheduleView {
                    frequency: s(v, "frequency"),
                    recipient_roles: v
                        .get("recipientRoles")
                        .and_then(|a| a.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()),
                }),
            remediation_triggers: eq
                .get("remediationTriggers")
                .and_then(|a| a.as_array())
                .map(|a| {
                    a.iter()
                        .map(|v| EquityRemediationTriggerView {
                            condition: s(v, "condition").unwrap_or_default(),
                            action: s(v, "action").unwrap_or_default(),
                            notify_roles: v
                                .get("notifyRoles")
                                .and_then(|a| a.as_array())
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|x| x.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default(),
                            description: s(v, "description"),
                        })
                        .collect()
                }),
        })
    }

    pub async fn delegations(&self, workflow_url: &str) -> ApiResult<Vec<DelegationEntryView>> {
        let rows = self.storage.list_delegations(workflow_url).await?;
        Ok(rows
            .into_iter()
            .map(|r| DelegationEntryView {
                id: r.id,
                delegator: r.delegator,
                delegate: r.delegate,
                scope: r.scope,
                authority: r.authority,
                legal_instrument: r.legal_instrument,
                start_date: r.start_date.to_rfc3339(),
                end_date: r.end_date.map(|t| t.to_rfc3339()),
                status: r.status,
            })
            .collect())
    }

    pub async fn revoke_delegation(
        &self,
        workflow_url: &str,
        id: &str,
    ) -> crate::error::ApiResult<()> {
        self.storage.revoke_delegation(workflow_url, id).await?;
        Ok(())
    }

    /// `POST /api/governance/:url/delegations` — create or update a
    /// delegation record. Writes to the `delegations` table (append or
    /// upsert on existing id). Provenance of the delegation itself is
    /// captured by the caller as a separate `RecordDelegation`-shaped
    /// provenance entry in the target instance's chain.
    pub async fn create_delegation(
        &self,
        workflow_url: &str,
        entry: &crate::domain::DelegationEntryView,
    ) -> crate::error::ApiResult<()> {
        let row = crate::storage::DelegationRow {
            id: entry.id.clone(),
            workflow_url: workflow_url.to_string(),
            delegator: entry.delegator.clone(),
            delegate: entry.delegate.clone(),
            scope: entry.scope.clone(),
            authority: entry.authority.clone(),
            legal_instrument: entry.legal_instrument.clone(),
            start_date: chrono::DateTime::parse_from_rfc3339(&entry.start_date)
                .map_err(|e| {
                    crate::error::ApiError::BadRequest(format!(
                        "invalid startDate: {e}"
                    ))
                })?
                .with_timezone(&chrono::Utc),
            end_date: entry
                .end_date
                .as_deref()
                .map(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .map(|t| t.with_timezone(&chrono::Utc))
                })
                .transpose()
                .map_err(|e| {
                    crate::error::ApiError::BadRequest(format!("invalid endDate: {e}"))
                })?,
            status: entry.status.clone(),
        };
        self.storage.upsert_delegation(&row).await?;
        Ok(())
    }

    pub async fn policy_versions(&self, workflow_url: &str) -> Vec<PolicyVersionView> {
        let Some(bundle) = self.bundle.full_bundle(workflow_url).await else {
            return Vec::new();
        };
        let Some(pp) = bundle.policy_parameters.as_ref() else {
            return Vec::new();
        };
        let versions = pp.get("versions").and_then(|a| a.as_array());
        let today = chrono::Utc::now();
        versions
            .map(|a| {
                a.iter()
                    .map(|v| {
                        let effective = s(v, "effectiveDate").unwrap_or_default();
                        let expiry = s(v, "expiryDate");
                        let params = v
                            .get("parameters")
                            .and_then(|p| p.as_object())
                            .map(|m| m.len() as u64)
                            .unwrap_or(0);
                        let status = classify_version(&effective, expiry.as_deref(), &today);
                        PolicyVersionView {
                            id: s(v, "id").unwrap_or_default(),
                            label: s(v, "label").unwrap_or_default(),
                            effective_date: effective,
                            expiry_date: expiry,
                            parameter_count: params,
                            status,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Resolve the `policy-parameters` version active at `as_of`. WS-034 backs
    /// both `POST /governance/{url}/policy-resolve` (legacy body form) and
    /// `GET /policy/{url}/resolve?asOf=` (decided 2026-04-25). A version is
    /// "active" when `effectiveDate <= as_of < (expiryDate or +inf)` — a
    /// half-open interval: `effectiveDate` is inclusive, `expiryDate` is
    /// exclusive. When multiple versions cover `as_of`, returns the one with
    /// the latest `effectiveDate` (versions are sorted descending by parsed
    /// `effectiveDate` before the walk, so JSON-array order is irrelevant).
    /// Returns `None` if the workflow has no `policy-parameters` sidecar or
    /// the requested instant falls in a gap (before the earliest effective
    /// date, or inside an unbounded gap between versions). Versions whose
    /// `effectiveDate` fails RFC3339 parsing are treated as missing and a
    /// `tracing::warn!` is emitted so ops can find them.
    pub async fn resolve_policy(
        &self,
        workflow_url: &str,
        as_of: &chrono::DateTime<chrono::Utc>,
    ) -> Option<ResolvedPolicyView> {
        let bundle = self.bundle.full_bundle(workflow_url).await?;
        let pp = bundle.policy_parameters.as_ref()?;
        let versions = pp.get("versions").and_then(|a| a.as_array())?;
        let parse = |s: &str| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|x| x.with_timezone(&chrono::Utc))
        };

        // Pair each version with its parsed effectiveDate; drop entries with
        // missing or unparseable effectiveDate (with a warn for ops). Sort
        // descending by effectiveDate so the first interval-matching entry
        // wins regardless of JSON-array order.
        let mut sorted: Vec<(chrono::DateTime<chrono::Utc>, &serde_json::Value)> = versions
            .iter()
            .filter_map(|v| match s(v, "effectiveDate").as_deref().and_then(parse) {
                Some(t) => Some((t, v)),
                None => {
                    tracing::warn!(
                        workflow_url = %workflow_url,
                        version_id = %s(v, "id").unwrap_or_default(),
                        "policy-parameters version has missing or unparseable effectiveDate; skipping",
                    );
                    None
                }
            })
            .collect();
        sorted.sort_by(|a, b| b.0.cmp(&a.0));

        let mut best: Option<&serde_json::Value> = None;
        for (eff, v) in &sorted {
            if eff > as_of {
                continue;
            }
            if let Some(exp_str) = s(v, "expiryDate").as_deref() {
                if let Some(exp) = parse(exp_str) {
                    if &exp <= as_of {
                        continue;
                    }
                }
            }
            best = Some(v);
            break;
        }

        best.map(|v| {
            let params = v
                .get("parameters")
                .cloned()
                .unwrap_or(serde_json::Value::Object(Default::default()));
            ResolvedPolicyView {
                id: s(v, "id").unwrap_or_default(),
                label: s(v, "label").unwrap_or_default(),
                effective_date: s(v, "effectiveDate").unwrap_or_default(),
                expiry_date: s(v, "expiryDate"),
                parameters: params,
                resolved_as_of: as_of.to_rfc3339(),
            }
        })
    }

    pub async fn calendar_events(&self, workflow_url: &str) -> Vec<CalendarEventView> {
        let Some(bundle) = self.bundle.full_bundle(workflow_url).await else {
            return Vec::new();
        };
        let Some(bc) = bundle.business_calendar.as_ref() else {
            return Vec::new();
        };
        bc.get("events")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .map(|v| CalendarEventView {
                        id: s(v, "id").unwrap_or_default(),
                        name: s(v, "name").unwrap_or_default(),
                        date: s(v, "date").unwrap_or_default(),
                        event_type: s(v, "type").unwrap_or_else(|| "agency".into()),
                        impacts_deadlines: v
                            .get("impactsDeadlines")
                            .and_then(|x| x.as_bool())
                            .unwrap_or(false),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub async fn health(&self) -> Vec<ServiceHealthView> {
        // Placeholder: projects the first bundle's integrationProfile entries.
        // Real implementation probes registered processors in a later step.
        let mut out = Vec::new();
        for summary in self.bundle.list().await {
            let Some(bundle) = self.bundle.full_bundle(&summary.url).await else {
                continue;
            };
            let Some(ip) = bundle.integration_profile.as_ref() else {
                continue;
            };
            if let Some(procs) = ip.get("processors").and_then(|a| a.as_array()) {
                for (i, p) in procs.iter().enumerate() {
                    out.push(ServiceHealthView {
                        id: s(p, "id").unwrap_or_else(|| format!("svc-{i}")),
                        name: s(p, "name").unwrap_or_else(|| format!("processor-{i}")),
                        status: "healthy".into(),
                        latency: "—".into(),
                        error_rate: "0%".into(),
                        last_check: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }
        }
        out
    }

    pub async fn render_adverse_notice(
        &self,
        workflow_url: &str,
        template_id: &str,
        context: &serde_json::Value,
    ) -> ApiResult<AdverseDecisionNoticeView> {
        let bundle = self
            .bundle
            .full_bundle(workflow_url)
            .await
            .ok_or(ApiError::NotFound)?;

        let templates = bundle.notification_templates.ok_or_else(|| {
            ApiError::BadRequest("no notification-template sidecar attached".into())
        })?;
        let tmpl = find_template(&templates, template_id).ok_or(ApiError::NotFound)?;

        let body_template = tmpl
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::BadRequest("template has no `body` field".into()))?;

        let due = bundle.due_process.as_ref();
        let grace_period = due
            .and_then(|d| d.get("gracePeriod"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let appeal_window = due
            .and_then(|d| d.get("appealWindow"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let right_to_contest = due
            .and_then(|d| d.get("rightToContest"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let mut merged_context = context.clone();
        if let Some(obj) = merged_context.as_object_mut() {
            if let Some(gp) = &grace_period {
                obj.insert("gracePeriod".into(), serde_json::Value::String(gp.clone()));
            }
            if let Some(aw) = &appeal_window {
                obj.insert("appealWindow".into(), serde_json::Value::String(aw.clone()));
            }
            if let Some(rtc) = &right_to_contest {
                obj.insert("rightToContest".into(), serde_json::Value::String(rtc.clone()));
            }
        }

        let body = interpolate(body_template, &merged_context);
        let subject = tmpl
            .get("subject")
            .and_then(|v| v.as_str())
            .map(|s| interpolate(s, &merged_context));
        let channels = tmpl
            .get("channels")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(AdverseDecisionNoticeView {
            template_id: template_id.into(),
            body,
            subject,
            grace_period,
            appeal_window,
            right_to_contest,
            channels,
        })
    }
}

fn classify_version(
    effective: &str,
    expiry: Option<&str>,
    now: &chrono::DateTime<chrono::Utc>,
) -> String {
    let parse = |s: &str| chrono::DateTime::parse_from_rfc3339(s).ok().map(|x| x.with_timezone(&chrono::Utc));
    let eff = parse(effective);
    let exp = expiry.and_then(parse);
    match (eff, exp) {
        (Some(eff), _) if eff > *now => "upcoming".into(),
        (_, Some(exp)) if exp < *now => "archived".into(),
        _ => "active".into(),
    }
}

fn map_agent(v: &serde_json::Value) -> AgentView {
    let capabilities = v
        .get("capabilities")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .map(|c| AgentCapabilityView {
                    name: s(c, "name").unwrap_or_default(),
                    autonomy: s(c, "autonomy").unwrap_or_else(|| "recommend".into()),
                })
                .collect()
        })
        .unwrap_or_default();
    AgentView {
        id: s(v, "id").unwrap_or_default(),
        name: s(v, "name").unwrap_or_default(),
        agent_type: s(v, "type").unwrap_or_else(|| "agent".into()),
        version: s(v, "version").unwrap_or_default(),
        status: s(v, "status").unwrap_or_else(|| "active".into()),
        capabilities,
        confidence_floor: v.get("confidenceFloor").and_then(|x| x.as_f64()),
    }
}

fn map_deontic(v: &serde_json::Value) -> DeonticConstraintView {
    DeonticConstraintView {
        kind: s(v, "kind").unwrap_or_else(|| "obligation".into()),
        id: s(v, "id").unwrap_or_default(),
        summary: s(v, "summary").unwrap_or_default(),
        detail: s(v, "detail"),
        on_violation: s(v, "onViolation"),
        bypassable: v.get("bypassable").and_then(|x| x.as_bool()),
    }
}

fn map_pipeline(v: &serde_json::Value) -> PipelineView {
    PipelineView {
        id: s(v, "id").unwrap_or_default(),
        description: s(v, "description"),
        stages: v
            .get("stages")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().map(map_stage).collect())
            .unwrap_or_default(),
    }
}

fn map_stage(v: &serde_json::Value) -> PipelineStageView {
    PipelineStageView {
        id: s(v, "id").unwrap_or_default(),
        stage_type: s(v, "type").unwrap_or_else(|| "transform".into()),
        contract_ref: s(v, "contractRef"),
        assertions: v
            .get("assertions")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().map(map_assertion).collect()),
        rejection_policy: s(v, "rejectionPolicy"),
        description: s(v, "description"),
    }
}

fn map_assertion(v: &serde_json::Value) -> PipelineAssertionView {
    PipelineAssertionView {
        assertion_type: s(v, "type").unwrap_or_default(),
        expression: s(v, "expression"),
        fields: v
            .get("fields")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()),
        description: s(v, "description"),
        rejection_policy: s(v, "rejectionPolicy"),
    }
}

fn map_verification_result(v: &serde_json::Value) -> VerificationResultView {
    VerificationResultView {
        constraint_ref: s(v, "constraintRef").unwrap_or_default(),
        result: s(v, "result").unwrap_or_else(|| "inconclusive".into()),
        solver_time_ms: v.get("solverTimeMs").and_then(|x| x.as_u64()),
        notes: s(v, "notes"),
        counterexample: v.get("counterexample").map(|c| VerificationCounterexampleView {
            inputs: c.get("inputs").cloned(),
            explanation: s(c, "explanation"),
        }),
    }
}

fn map_equity_category(v: &serde_json::Value) -> EquityCategoryView {
    EquityCategoryView {
        id: s(v, "id").unwrap_or_default(),
        group_by_path: s(v, "groupByPath").unwrap_or_default(),
        description: s(v, "description"),
        groups: v
            .get("groups")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default(),
    }
}

fn s(v: &serde_json::Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string())
}

fn u64_opt(v: &serde_json::Value, k: &str) -> Option<u64> {
    v.get(k).and_then(|x| x.as_u64())
}
