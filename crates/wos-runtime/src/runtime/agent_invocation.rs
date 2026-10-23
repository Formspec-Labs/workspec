// Rust guideline compliant 2026-02-21

//! Agent capability invocation and precondition enforcement (AI §3.3.1, ADR 0064).
//!
//! Manages native capability selection, FEL precondition evaluation, input rights/contracts,
//! provenance emission, and `AgentInvoker` dispatch.

use wos_core::agent::{AgentContext, AgentTask, InvokerKind};
use wos_core::evaluate_and_emit_capability_preconditions;
use wos_core::model::ai::AgentDeclaration;

use super::{RuntimeError, WosRuntime, process_minter};
use crate::durable::{AgentInvocationDecision, AgentRuntime};
use crate::store::RuntimeRecord;

impl WosRuntime {
    /// Programmatic agent capability invocation on an existing workflow process.
    pub fn invoke_agent(
        &mut self,
        process_id: &str,
        agent_id: &str,
        capability_id: &str,
        task: AgentTask,
    ) -> Result<AgentInvocationDecision, RuntimeError> {
        let mut record = self.store.load_record(process_id)?;
        let kernel = self
            .resolver
            .resolve_kernel(&record.process.definition_url, &record.process.definition_version)?;
        let decision =
            self.invoke_agent_for_record(&mut record, &kernel, agent_id, capability_id, task)?;
        self.store.save_record(record)?;
        Ok(decision)
    }

    /// Invokes an agent capability against a runtime record, evaluating preconditions
    /// and emitting `capabilityInvocation` provenance records.
    pub(crate) fn invoke_agent_for_record(
        &mut self,
        record: &mut RuntimeRecord,
        kernel: &wos_core::model::kernel::KernelDocument,
        agent_id: &str,
        capability_id: &str,
        task: AgentTask,
    ) -> Result<AgentInvocationDecision, RuntimeError> {
        let minter = process_minter(&record.process)?;

        // 1. Native capability selection: locate the agent declaration and capability.
        let mut target_decl: Option<AgentDeclaration> = None;
        for agent_val in &kernel.agents {
            if let Ok(decl) = serde_json::from_value::<AgentDeclaration>(agent_val.clone()) {
                if decl.id == agent_id {
                    target_decl = Some(decl);
                    break;
                }
            }
        }
        let decl = target_decl.ok_or_else(|| {
            RuntimeError::UnsupportedAction(format!(
                "agent '{}' not declared in workflow definition",
                agent_id
            ))
        })?;

        let capability = decl
            .capabilities
            .iter()
            .find(|c| c.id == capability_id)
            .ok_or_else(|| {
                RuntimeError::UnsupportedAction(format!(
                    "capability '{}' not declared for agent '{}'",
                    capability_id, agent_id
                ))
            })?
            .clone();

        // 2. Evaluate capability preconditions
        let task_inputs_val = serde_json::to_value(&task.inputs).ok();
        let (eval, prov_record) = evaluate_and_emit_capability_preconditions(
            &minter,
            &capability,
            agent_id,
            &record.process.case_state,
            None,
            task_inputs_val.as_ref(),
        );

        let prov_id = prov_record.id.clone();
        record.provenance_log.push(prov_record);

        if !eval.passed {
            // Blocked by precondition: do NOT invoke AgentInvoker
            return Ok(AgentInvocationDecision {
                invocation_blocked: true,
                outcome: Some("preconditionNotSatisfied".to_string()),
                result: None,
                provenance_record_id: prov_id,
            });
        }

        // 3. Preconditions passed: validate input contract if declared
        if let Some(ref contract_ref) = capability.input_contract_ref {
            if let Some(ref inputs) = task_inputs_val {
                let val_res = self.validator.validate(contract_ref, inputs)?;
                if !val_res.valid {
                    return Err(RuntimeError::Evaluator(format!(
                        "input contract violation for capability '{}': {:?}",
                        capability_id, val_res.errors
                    )));
                }
            }
        }

        // 4. Look up registered invoker
        let invoker_spec = decl.invoker.as_ref().ok_or_else(|| {
            RuntimeError::UnsupportedAction(format!(
                "agent '{}' has no invoker spec configured",
                agent_id
            ))
        })?;

        let invoker = self.agent_invokers.lookup(invoker_spec).ok_or_else(|| {
            RuntimeError::UnsupportedAction(format!(
                "no agent invoker registered for kind {:?}",
                InvokerKind::from(invoker_spec)
            ))
        })?;

        let invocation_index = record
            .provenance_log
            .iter()
            .filter(|p| {
                p.record_kind == wos_core::ProvenanceKind::CapabilityInvocation
                    && p.actor_id.as_deref() == Some(agent_id)
            })
            .count() as u32;

        let ctx = AgentContext {
            process_id: &record.process.process_id,
            invocation_index,
            case_state: &record.process.case_state,
        };

        let result = invoker
            .invoke(&decl, &task, &ctx)
            .map_err(|e| RuntimeError::Evaluator(e.to_string()))?;

        Ok(AgentInvocationDecision {
            invocation_blocked: false,
            outcome: Some("completed".to_string()),
            result: Some(result),
            provenance_record_id: prov_id,
        })
    }
}

impl AgentRuntime for WosRuntime {
    fn invoke_agent(
        &mut self,
        process_id: &str,
        agent_id: &str,
        capability_id: &str,
        task: AgentTask,
    ) -> Result<AgentInvocationDecision, RuntimeError> {
        WosRuntime::invoke_agent(self, process_id, agent_id, capability_id, task)
    }
}
