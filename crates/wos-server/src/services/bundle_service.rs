use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;
use wos_server_ports::runtime::{BundleResolverPort, RuntimeAdapterError};

use crate::config::ServerConfig;
use crate::domain::{BundleView, KernelSummaryView, ValidationIssueView, ValidationResultView};
use crate::storage::{KernelRow, StorageError, StorageHandle};

/// Known sidecars: `(fixture subdirectory, BundleView field name)`.
const SIDECARS: &[(&str, &str)] = &[
    ("governance", "governance"),
    ("due-process", "dueProcess"),
    ("assertion-gate", "assertionGates"),
    ("ai", "ai"),
    ("policy-parameters", "policyParameters"),
    ("notification-template", "notificationTemplates"),
    ("business-calendar", "businessCalendar"),
    ("advanced", "advanced"),
    ("equity", "equity"),
    ("drift-monitor", "driftMonitor"),
    ("agent-config", "agentConfigs"),
    ("verification-report", "verificationReport"),
    ("correspondence-metadata", "correspondenceMetadata"),
    ("semantic-profile", "semanticProfile"),
    ("integration-profile", "integrationProfile"),
    ("lifecycle-detail", "lifecycleDetail"),
    ("case-instance", "caseInstances"),
];

/// Kernel registry + bundle projection. [`Self::full_bundle`] joins the
/// kernel from SQLite with sidecar JSON files under
/// `{fixtures_dir}/{governance|semantic-profile|…}/{url_slug}.json}` only —
/// sidecars are not read from the database. See [`url_slug`] for how
/// fixture filenames are derived from workflow URLs.
pub struct BundleService {
    cfg: Arc<ServerConfig>,
    storage: StorageHandle,
    cache: RwLock<std::collections::HashMap<String, KernelRow>>,
    primary_url: RwLock<Option<String>>,
}

impl BundleService {
    pub async fn new(cfg: Arc<ServerConfig>, storage: StorageHandle) -> anyhow::Result<Self> {
        let svc = Self {
            cfg,
            storage,
            cache: RwLock::new(Default::default()),
            primary_url: RwLock::new(None),
        };
        svc.hydrate().await?;
        Ok(svc)
    }

    pub async fn hydrate(&self) -> anyhow::Result<()> {
        let kernels_dir = self.cfg.fixtures_dir.join("kernel");
        if kernels_dir.exists() {
            let mut rd = tokio::fs::read_dir(&kernels_dir).await?;
            while let Some(entry) = rd.next_entry().await? {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                if let Err(e) = self.ingest_fixture(&path).await {
                    tracing::warn!(?path, error = %e, "failed to ingest kernel fixture");
                }
            }
        }

        let rows = self.storage.list_kernels().await?;
        let mut cache = self.cache.write().await;
        let mut primary = self.primary_url.write().await;
        cache.clear();
        for r in rows {
            if primary.is_none() {
                *primary = Some(r.url.clone());
            }
            cache.insert(r.url.clone(), r);
        }
        Ok(())
    }

    async fn ingest_fixture(&self, path: &Path) -> anyhow::Result<()> {
        let bytes = tokio::fs::read(path).await?;
        let doc: serde_json::Value = serde_json::from_slice(&bytes)?;
        let row = kernel_row_from_doc(doc)?;
        self.storage.upsert_kernel(&row).await?;
        Ok(())
    }

    pub async fn list(&self) -> Vec<KernelSummaryView> {
        let cache = self.cache.read().await;
        let mut v: Vec<KernelSummaryView> = cache
            .values()
            .map(|r| KernelSummaryView {
                url: r.url.clone(),
                title: r.title.clone(),
                version: r.version.clone(),
                status: r.status.clone(),
                impact_level: r.impact_level.clone(),
            })
            .collect();
        v.sort_by(|a, b| a.url.cmp(&b.url));
        v
    }

    pub async fn primary_kernel(&self) -> Option<KernelRow> {
        let url = self.primary_url.read().await.clone()?;
        self.cache.read().await.get(&url).cloned()
    }

    pub async fn get(&self, url: &str) -> Option<KernelRow> {
        self.cache.read().await.get(url).cloned()
    }

    pub async fn replace(
        &self,
        url: &str,
        document: serde_json::Value,
    ) -> Result<KernelRow, StorageError> {
        let existing = self
            .storage
            .get_kernel(url)
            .await?
            .ok_or(StorageError::NotFound)?;
        let mut row =
            kernel_row_from_doc(document).map_err(|e| StorageError::Other(e.to_string()))?;
        // Keep the URL the PUT was addressed to — the client is the authority
        // on the path; spare them from surprising us if the body's `url`
        // differs from the path slug.
        row.url = url.to_string();
        // Fall back to existing metadata on any missing top-level field.
        if row.title.is_empty() {
            row.title = existing.title;
        }
        if row.version == "0.0.0" {
            row.version = existing.version;
        }
        self.storage.upsert_kernel(&row).await?;
        self.cache
            .write()
            .await
            .insert(url.to_string(), row.clone());
        Ok(row)
    }

    /// Kernel from the in-memory cache plus optional sidecars from the
    /// fixture tree on disk (not from persistent sidecar storage).
    pub async fn full_bundle(&self, url: &str) -> Option<BundleView> {
        let row = self.get(url).await?;
        let mut bundle = BundleView {
            kernel: row.document,
            governance: None,
            due_process: None,
            assertion_gates: None,
            ai: None,
            policy_parameters: None,
            notification_templates: None,
            business_calendar: None,
            advanced: None,
            equity: None,
            drift_monitor: None,
            agent_configs: None,
            verification_report: None,
            correspondence_metadata: None,
            semantic_profile: None,
            integration_profile: None,
            lifecycle_detail: None,
            case_instances: None,
        };
        let slug = url_slug(url);
        for (subdir, field) in SIDECARS.iter() {
            let path: PathBuf = self
                .cfg
                .fixtures_dir
                .join(subdir)
                .join(format!("{slug}.json"));
            if let Ok(bytes) = tokio::fs::read(&path).await {
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    assign_sidecar(&mut bundle, field, v);
                }
            }
        }
        Some(bundle)
    }
}

#[async_trait::async_trait]
impl BundleResolverPort for BundleService {
    async fn resolve_kernel_bundle(
        &self,
        workflow_url: &str,
    ) -> Result<serde_json::Value, RuntimeAdapterError> {
        self.get(workflow_url)
            .await
            .map(|row| row.document)
            .ok_or_else(|| {
                RuntimeAdapterError::Message(format!("kernel `{workflow_url}` not found"))
            })
    }

    async fn resolve_governance_bundle(
        &self,
        workflow_url: &str,
    ) -> Result<serde_json::Value, RuntimeAdapterError> {
        self.full_bundle(workflow_url)
            .await
            .and_then(|bundle| bundle.governance)
            .ok_or_else(|| {
                RuntimeAdapterError::Message(format!(
                    "governance sidecar for `{workflow_url}` not found"
                ))
            })
    }

    async fn resolve_sidecar_bundle(
        &self,
        workflow_url: &str,
    ) -> Result<serde_json::Value, RuntimeAdapterError> {
        let bundle = self.full_bundle(workflow_url).await.ok_or_else(|| {
            RuntimeAdapterError::Message(format!("bundle for `{workflow_url}` not found"))
        })?;
        serde_json::to_value(bundle).map_err(|e| {
            RuntimeAdapterError::Message(format!(
                "failed to serialise sidecar bundle for `{workflow_url}`: {e}"
            ))
        })
    }
}

fn assign_sidecar(bundle: &mut BundleView, field: &str, v: serde_json::Value) {
    match field {
        "governance" => bundle.governance = Some(v),
        "dueProcess" => bundle.due_process = Some(v),
        "assertionGates" => bundle.assertion_gates = Some(v),
        "ai" => bundle.ai = Some(v),
        "policyParameters" => bundle.policy_parameters = Some(v),
        "notificationTemplates" => bundle.notification_templates = Some(v),
        "businessCalendar" => bundle.business_calendar = Some(v),
        "advanced" => bundle.advanced = Some(v),
        "equity" => bundle.equity = Some(v),
        "driftMonitor" => bundle.drift_monitor = Some(v),
        "agentConfigs" => bundle.agent_configs = Some(v),
        "verificationReport" => bundle.verification_report = Some(v),
        "correspondenceMetadata" => bundle.correspondence_metadata = Some(v),
        "semanticProfile" => bundle.semantic_profile = Some(v),
        "integrationProfile" => bundle.integration_profile = Some(v),
        "lifecycleDetail" => bundle.lifecycle_detail = Some(v),
        "caseInstances" => bundle.case_instances = Some(v),
        _ => {}
    }
}

fn kernel_row_from_doc(doc: serde_json::Value) -> anyhow::Result<KernelRow> {
    let url = doc
        .get("url")
        .and_then(|v| v.as_str())
        .or_else(|| doc.get("id").and_then(|v| v.as_str()))
        .ok_or_else(|| anyhow::anyhow!("kernel missing `url`"))?
        .to_string();
    let title = doc
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(&url)
        .to_string();
    let version = doc
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();
    let status = doc
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("draft")
        .to_string();
    let impact_level = doc
        .get("governance")
        .and_then(|g| g.get("impactLevel"))
        .and_then(|v| v.as_str())
        .or_else(|| doc.get("impactLevel").and_then(|v| v.as_str()))
        .unwrap_or("operational")
        .to_string();
    Ok(KernelRow {
        url,
        title,
        version,
        status,
        impact_level,
        document: doc,
        updated_at: Utc::now(),
    })
}

/// Lint a kernel JSON via `wos-lint` and map diagnostics to the studio's
/// `WosValidationResult` shape.
pub fn validate_kernel(doc: &serde_json::Value) -> ValidationResultView {
    let json = match serde_json::to_string(doc) {
        Ok(s) => s,
        Err(e) => {
            return ValidationResultView {
                is_valid: false,
                issues: vec![ValidationIssueView {
                    severity: "error".into(),
                    category: "structure".into(),
                    message: format!("serialisation error: {e}"),
                    target_id: None,
                }],
            };
        }
    };
    match wos_lint::lint_document(&json) {
        Ok(diags) => {
            let issues: Vec<ValidationIssueView> = diags
                .iter()
                .map(|d| ValidationIssueView {
                    severity: severity_to_str(d.severity).into(),
                    category: rule_id_to_category(d.rule_id).into(),
                    message: d.message.clone(),
                    target_id: Some(d.path.clone()).filter(|s| !s.is_empty()),
                })
                .collect();
            let is_valid = !issues.iter().any(|i| i.severity == "error");
            ValidationResultView { is_valid, issues }
        }
        Err(e) => ValidationResultView {
            is_valid: false,
            issues: vec![ValidationIssueView {
                severity: "error".into(),
                category: "structure".into(),
                message: e.to_string(),
                target_id: None,
            }],
        },
    }
}

fn severity_to_str(s: wos_lint::LintSeverity) -> &'static str {
    match s {
        wos_lint::LintSeverity::Error => "error",
        wos_lint::LintSeverity::Warning => "warning",
        wos_lint::LintSeverity::Info => "info",
    }
}

fn rule_id_to_category(rule_id: &str) -> &'static str {
    // Studio categories: "structure" | "policy" | "soundness" | "satisfiability".
    // Map by LINT-MATRIX rule-id prefix. "K-" and "G-" single-doc checks are
    // structural; runtime/policy rules land in "policy".
    match rule_id.chars().next() {
        Some('K') | Some('G') | Some('T') | Some('B') => "structure",
        Some('P') => "policy",
        Some('S') => "soundness",
        Some('F') => "satisfiability",
        _ => "structure",
    }
}

/// Slug for fixture filenames: second segment from the right when split by
/// `:`, otherwise the last `/` path segment (trimming `.json`). Example:
/// `urn:wos:workflow:demo:1.0.0` → `demo` → `fixtures/governance/demo.json`.
fn url_slug(url: &str) -> String {
    url.rsplit(':')
        .nth(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            url.rsplit('/')
                .next()
                .unwrap_or(url)
                .trim_end_matches(".json")
                .to_string()
        })
}
