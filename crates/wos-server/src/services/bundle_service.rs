use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::ServerConfig;
use crate::storage::{KernelRow, StorageError, StorageHandle};

/// Studio-facing kernel summary (`KernelSummary` in `WosBackend.ts`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelSummary {
    pub url: String,
    pub title: String,
    pub version: String,
    pub status: String,
    pub impact_level: String,
}

impl From<&KernelRow> for KernelSummary {
    fn from(r: &KernelRow) -> Self {
        Self {
            url: r.url.clone(),
            title: r.title.clone(),
            version: r.version.clone(),
            status: r.status.clone(),
            impact_level: r.impact_level.clone(),
        }
    }
}

/// Full document bundle: kernel + any sidecars (governance, ai, etc.).
/// Mirrors `WosDocumentBundle` in the studio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WosDocumentBundle {
    pub kernel: serde_json::Value,
    #[serde(flatten)]
    pub sidecars: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy)]
enum Sidecar {
    Governance,
    Ai,
    CaseFile,
    Cdm,
    Forms,
    Presentation,
    Integrations,
    BusinessCalendar,
    Ontology,
    PolicyParameters,
    ProvenanceIndex,
    Qa,
    Reporting,
}

impl Sidecar {
    const ALL: &'static [(Sidecar, &'static str, &'static str)] = &[
        (Sidecar::Governance, "governance", "governance"),
        (Sidecar::Ai, "ai", "ai"),
        (Sidecar::CaseFile, "case-file", "caseFile"),
        (Sidecar::Cdm, "cdm", "cdm"),
        (Sidecar::Forms, "forms", "forms"),
        (Sidecar::Presentation, "presentation", "presentation"),
        (Sidecar::Integrations, "integrations", "integrations"),
        (
            Sidecar::BusinessCalendar,
            "business-calendar",
            "businessCalendar",
        ),
        (Sidecar::Ontology, "ontology", "ontology"),
        (
            Sidecar::PolicyParameters,
            "policy-parameters",
            "policyParameters",
        ),
        (
            Sidecar::ProvenanceIndex,
            "provenance-index",
            "provenanceIndex",
        ),
        (Sidecar::Qa, "qa", "qa"),
        (Sidecar::Reporting, "reporting", "reporting"),
    ];
}

/// Loads kernels from the fixtures dir at boot, persists them into the
/// storage registry, and serves read-paths from an in-memory cache.
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

    /// Load from storage + seed fresh kernels from the fixtures directory.
    pub async fn hydrate(&self) -> anyhow::Result<()> {
        // Always scan fixtures and upsert — kernels can be overwritten by the
        // filesystem at boot so dev workflows stay fast.
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

        let row = KernelRow {
            url,
            title,
            version,
            status,
            impact_level,
            document: doc,
            updated_at: Utc::now(),
        };
        self.storage.upsert_kernel(&row).await?;
        Ok(())
    }

    pub async fn list(&self) -> Vec<KernelSummary> {
        let cache = self.cache.read().await;
        let mut v: Vec<_> = cache.values().map(Into::into).collect();
        v.sort_by(|a: &KernelSummary, b: &KernelSummary| a.url.cmp(&b.url));
        v
    }

    pub async fn primary_kernel(&self) -> Option<KernelRow> {
        let url = self.primary_url.read().await.clone()?;
        self.cache.read().await.get(&url).cloned()
    }

    pub async fn get(&self, url: &str) -> Option<KernelRow> {
        self.cache.read().await.get(url).cloned()
    }

    pub async fn replace(&self, url: &str, document: serde_json::Value) -> Result<KernelRow, StorageError> {
        let existing = self
            .storage
            .get_kernel(url)
            .await?
            .ok_or(StorageError::NotFound)?;
        let title = document
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(&existing.title)
            .to_string();
        let version = document
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or(&existing.version)
            .to_string();
        let status = document
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or(&existing.status)
            .to_string();
        let impact_level = document
            .get("governance")
            .and_then(|g| g.get("impactLevel"))
            .and_then(|v| v.as_str())
            .or_else(|| document.get("impactLevel").and_then(|v| v.as_str()))
            .unwrap_or(&existing.impact_level)
            .to_string();

        let row = KernelRow {
            url: url.to_string(),
            title,
            version,
            status,
            impact_level,
            document,
            updated_at: Utc::now(),
        };
        self.storage.upsert_kernel(&row).await?;
        self.cache.write().await.insert(url.to_string(), row.clone());
        Ok(row)
    }

    /// Build the full bundle (kernel + sidecars) for an URL.
    pub async fn full_bundle(&self, url: &str) -> Option<WosDocumentBundle> {
        let row = self.get(url).await?;
        let mut bundle = WosDocumentBundle {
            kernel: row.document,
            sidecars: serde_json::Map::new(),
        };
        // Sidecars are mirrored from the fixtures dir: `<fixtures>/<kind>/<slug>.json`.
        let slug = url_slug(url);
        for (_, dir, field) in Sidecar::ALL.iter() {
            let path: PathBuf = self
                .cfg
                .fixtures_dir
                .join(dir)
                .join(format!("{slug}.json"));
            if let Ok(bytes) = tokio::fs::read(&path).await {
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    bundle.sidecars.insert((*field).to_string(), v);
                }
            }
        }
        Some(bundle)
    }
}

fn url_slug(url: &str) -> String {
    // Tolerates `urn:wos:workflow:<slug>:<version>` and plain slugs.
    url.rsplit(':').nth(1).map(|s| s.to_string()).unwrap_or_else(|| {
        url.split('/')
            .last()
            .unwrap_or(url)
            .trim_end_matches(".json")
            .to_string()
    })
}
