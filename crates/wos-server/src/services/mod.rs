//! Domain services. Each service wraps storage + wos-core with
//! business logic and returns studio-shaped view types.

use std::sync::Arc;

use crate::config::ServerConfig;
use crate::storage::StorageHandle;

pub mod applicant_service;
pub mod assurance_service;
pub mod bundle_service;
pub mod calendar_service;
pub mod conformance_service;
pub mod dashboard_service;
pub mod deontic_service;
pub mod eval_service;
pub mod governance_service;
pub mod instance_service;
pub mod integration_service;
pub mod lint_service;
pub mod notifications_service;
pub mod provenance_service;
pub mod semantic_service;
pub mod timer_task;

pub use applicant_service::ApplicantService;
pub use bundle_service::BundleService;
pub use calendar_service::CalendarService;
pub use dashboard_service::DashboardService;
pub use eval_service::EvalService;
pub use governance_service::GovernanceService;
pub use instance_service::InstanceService;
pub use notifications_service::NotificationsService;
pub use provenance_service::ProvenanceService;

pub struct AppServices {
    pub bundle: Arc<BundleService>,
    pub instance: Arc<InstanceService>,
    pub provenance: Arc<ProvenanceService>,
    pub eval: Arc<EvalService>,
    pub governance: Arc<GovernanceService>,
    pub dashboard: Arc<DashboardService>,
    pub applicant: Arc<ApplicantService>,
    pub calendar: Arc<CalendarService>,
    pub notifications: Arc<NotificationsService>,
}

impl AppServices {
    pub async fn new(cfg: Arc<ServerConfig>, storage: StorageHandle) -> anyhow::Result<Self> {
        let bundle = Arc::new(BundleService::new(cfg.clone(), storage.clone()).await?);
        let provenance = Arc::new(ProvenanceService::new(storage.clone()));
        let instance = Arc::new(InstanceService::new(storage.clone(), bundle.clone()));
        let eval = Arc::new(EvalService::new(storage.clone(), bundle.clone()));
        let governance = Arc::new(GovernanceService::new(storage.clone(), bundle.clone()));
        let dashboard = Arc::new(DashboardService::new(storage.clone()));
        let applicant = Arc::new(ApplicantService::new(
            storage.clone(),
            bundle.clone(),
            provenance.clone(),
        ));
        let calendar = Arc::new(CalendarService::new(bundle.clone()));
        let notifications = Arc::new(NotificationsService::new(bundle.clone()));
        Ok(Self {
            bundle,
            instance,
            provenance,
            eval,
            governance,
            dashboard,
            applicant,
            calendar,
            notifications,
        })
    }
}
