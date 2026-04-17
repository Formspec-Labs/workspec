//! Instance projections — build the server's HTTP responses from stored
//! `wos_core::instance::CaseInstance` rows.

use std::sync::Arc;

use wos_core::instance::CaseInstance;

use crate::domain::{InstanceResponse, TaskListItem};
use crate::error::{ApiError, ApiResult};
use crate::storage::{InstanceRow, StorageHandle};

use super::bundle_service::BundleService;

pub struct InstanceService {
    storage: StorageHandle,
    bundle: Arc<BundleService>,
}

impl InstanceService {
    pub fn new(storage: StorageHandle, bundle: Arc<BundleService>) -> Self {
        Self { storage, bundle }
    }

    pub fn storage(&self) -> &StorageHandle {
        &self.storage
    }

    /// Deserialize the stored JSON blob into a `wos_core::CaseInstance`.
    pub fn parse(row: &InstanceRow) -> ApiResult<CaseInstance> {
        serde_json::from_value(row.instance_json.clone())
            .map_err(|e| ApiError::ServiceUnavailable(format!("instance rehydration failed: {e}")))
    }

    /// Wrap a parsed instance in the server's `InstanceResponse` envelope.
    pub async fn to_response(&self, row: &InstanceRow) -> ApiResult<InstanceResponse> {
        let instance = Self::parse(row)?;
        let definition_title = self
            .bundle
            .get(&row.definition_url)
            .await
            .map(|k| k.title);
        Ok(InstanceResponse {
            instance,
            impact_level: row.impact_level.clone(),
            definition_title,
        })
    }

    /// Flatten active tasks across an instance into `TaskListItem`s.
    pub async fn tasks_for(&self, row: &InstanceRow) -> ApiResult<Vec<TaskListItem>> {
        let instance = Self::parse(row)?;
        let definition_title = self
            .bundle
            .get(&row.definition_url)
            .await
            .map(|k| k.title);
        Ok(instance
            .active_tasks
            .iter()
            .map(|t| TaskListItem {
                task: t.clone(),
                instance_id: row.instance_id.clone(),
                definition_url: row.definition_url.clone(),
                definition_title: definition_title.clone(),
                configuration: instance.configuration.clone(),
                case_state: instance.case_state.clone(),
            })
            .collect())
    }
}
