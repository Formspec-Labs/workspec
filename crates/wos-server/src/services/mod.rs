//! Domain services. Each service wraps storage + wos-core with
//! business logic and returns studio-shaped view types.

use std::sync::Arc;

use crate::config::ServerConfig;
use crate::storage::StorageHandle;

pub mod applicant_service;
pub mod bundle_service;
pub mod dashboard_service;
pub mod governance_service;
pub mod instance_service;
pub mod provenance_service;

pub use applicant_service::ApplicantService;
pub use bundle_service::BundleService;
pub use dashboard_service::DashboardService;
pub use governance_service::GovernanceService;
pub use instance_service::InstanceService;
pub use provenance_service::ProvenanceService;

pub struct AppServices {
    pub bundle: Arc<BundleService>,
    pub instance: Arc<InstanceService>,
    pub provenance: Arc<ProvenanceService>,
    pub governance: Arc<GovernanceService>,
    pub dashboard: Arc<DashboardService>,
    pub applicant: Arc<ApplicantService>,
}

impl AppServices {
    pub async fn new(cfg: Arc<ServerConfig>, storage: StorageHandle) -> anyhow::Result<Self> {
        let bundle = Arc::new(BundleService::new(cfg.clone(), storage.clone()).await?);
        let provenance = Arc::new(ProvenanceService::new(storage.clone()));
        let instance = Arc::new(InstanceService::new(storage.clone(), bundle.clone()));
        let governance = Arc::new(GovernanceService::new(storage.clone(), bundle.clone()));
        let dashboard = Arc::new(DashboardService::new(storage.clone()));
        let applicant = Arc::new(ApplicantService::new(
            storage.clone(),
            bundle.clone(),
            provenance.clone(),
        ));
        Ok(Self {
            bundle,
            instance,
            provenance,
            governance,
            dashboard,
            applicant,
        })
    }
}
