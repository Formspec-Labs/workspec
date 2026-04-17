use std::sync::Arc;

use crate::domain::{
    ActiveTaskView, CaseInstanceView, DelegationShortView, GovernanceStateView, HoldView,
    TaskListItemView, TimerView,
};
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

    pub fn map_row(row: &InstanceRow) -> CaseInstanceView {
        let configuration = row.configuration();
        let active_tasks = row
            .active_tasks()
            .as_array()
            .map(|a| a.iter().map(map_task).collect())
            .unwrap_or_default();
        let timers = row
            .timers()
            .as_array()
            .map(|a| a.iter().map(map_timer).collect())
            .unwrap_or_default();
        let governance_state = row.governance_state().and_then(map_governance_state);

        CaseInstanceView {
            instance_id: row.instance_id.clone(),
            definition_url: row.definition_url.clone(),
            definition_version: row.definition_version.clone(),
            status: row.status.clone(),
            configuration,
            case_state: row.case_state(),
            active_tasks,
            timers,
            governance_state,
            impact_level: row.impact_level.clone(),
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
        }
    }

    pub async fn map_row_to_tasks(&self, row: &InstanceRow) -> Vec<TaskListItemView> {
        let definition_title = self
            .bundle
            .get(&row.definition_url)
            .await
            .map(|k| k.title)
            .unwrap_or_else(|| row.definition_url.clone());
        let configuration = row.configuration();
        let case_state = row.case_state();

        let Some(active_tasks) = row.active_tasks().as_array() else {
            return Vec::new();
        };
        active_tasks
            .iter()
            .map(|t| {
                let view = map_task(t);
                TaskListItemView {
                    task_id: view.task_id.clone(),
                    instance_id: row.instance_id.clone(),
                    task_ref: view.task_ref,
                    status: view.status,
                    assigned_actor: view.assigned_actor,
                    deadline: view.deadline,
                    impact_level: view.impact_level.or_else(|| Some(row.impact_level.clone())),
                    configuration: configuration.clone(),
                    case_state: case_state.clone(),
                    definition_title: definition_title.clone(),
                    definition_url: row.definition_url.clone(),
                    created_at: view.created_at,
                }
            })
            .collect()
    }
}

fn map_task(v: &serde_json::Value) -> ActiveTaskView {
    ActiveTaskView {
        task_id: s(v, "taskId").unwrap_or_default(),
        task_ref: s(v, "taskRef").unwrap_or_default(),
        status: s(v, "status").unwrap_or_else(|| "created".into()),
        assigned_actor: s(v, "assignedActor"),
        contract_ref: s(v, "contractRef"),
        binding: s(v, "binding"),
        deadline: s(v, "deadline"),
        impact_level: s(v, "impactLevel"),
        created_at: s(v, "createdAt").unwrap_or_default(),
        updated_at: s(v, "updatedAt").unwrap_or_default(),
    }
}

fn map_timer(v: &serde_json::Value) -> TimerView {
    TimerView {
        timer_id: s(v, "timerId").unwrap_or_default(),
        deadline: s(v, "deadline").unwrap_or_default(),
        event: s(v, "event").unwrap_or_default(),
        scope_state: s(v, "scopeState"),
    }
}

fn map_governance_state(v: &serde_json::Value) -> Option<GovernanceStateView> {
    Some(GovernanceStateView {
        active_delegations: v
            .get("activeDelegations")
            .and_then(|x| x.as_array())
            .map(|a| a.iter().map(map_delegation_short).collect())
            .unwrap_or_default(),
        active_holds: v
            .get("activeHolds")
            .and_then(|x| x.as_array())
            .map(|a| a.iter().map(map_hold).collect())
            .unwrap_or_default(),
        review_state: v
            .get("reviewState")
            .cloned()
            .unwrap_or(serde_json::json!({})),
    })
}

fn map_delegation_short(v: &serde_json::Value) -> DelegationShortView {
    DelegationShortView {
        delegator_id: s(v, "delegatorId").unwrap_or_default(),
        delegate_id: s(v, "delegateId").unwrap_or_default(),
        scope: s(v, "scope").unwrap_or_default(),
        authority: s(v, "authority"),
        granted_at: s(v, "grantedAt").unwrap_or_default(),
        expires_at: s(v, "expiresAt"),
    }
}

fn map_hold(v: &serde_json::Value) -> HoldView {
    HoldView {
        hold_type: s(v, "holdType").unwrap_or_default(),
        started_at: s(v, "startedAt").unwrap_or_default(),
        expected_end: s(v, "expectedEnd"),
        resume_trigger: s(v, "resumeTrigger").unwrap_or_default(),
        hold_state: s(v, "holdState"),
    }
}

fn s(v: &serde_json::Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string())
}
