//! Selects the actual installed governance owner before runtime effects.
use super::{NativeReadOnlyDependency, RuntimeError, WosRuntime};
use crate::governance::{
    CapturedGovernanceEvidence, QualifiedGovernancePolicy, RuntimeGovernanceResultContext,
};
use crate::store::RuntimeRecord;
use wos_core::{KernelDocument, ProvenanceRecord};

impl WosRuntime {
    /// Installs one qualified generic policy without replacing the AI pre-event hook.
    ///
    /// # Errors
    /// Refuses preparation-time changes, reference composition and a foreign kernel.
    pub fn with_governance_policy(
        mut self,
        policy: QualifiedGovernancePolicy,
        evidence: CapturedGovernanceEvidence,
    ) -> Result<Self, RuntimeError> {
        if self.preparation.is_some()
            || self.reference_due_process.is_some()
            || reference_selected(policy.kernel())
        {
            return Err(refused(
                "generic and reference governance cannot be composed",
            ));
        }
        let kernel = self.resolver.resolve_kernel(
            policy
                .kernel()
                .url
                .as_deref()
                .ok_or_else(|| refused("selected workflow URL missing"))?,
            policy
                .kernel()
                .version
                .as_deref()
                .ok_or_else(|| refused("selected workflow version missing"))?,
        )?;
        if !policy.matches_kernel(&kernel) {
            return Err(refused("installed governance kernel differs"));
        }
        self.generic_governance = Some(policy.selected(evidence));
        Ok(self)
    }

    pub(super) fn validate_governance_selection(
        &self,
        kernel: &KernelDocument,
    ) -> Result<(), RuntimeError> {
        if let Some(selected) = &self.generic_governance {
            self.require_native_dependency(NativeReadOnlyDependency::GovernancePolicy)?;
            if self.reference_due_process.is_some()
                || reference_selected(kernel)
                || !selected.policy.matches_kernel(kernel)
            {
                return Err(refused(
                    "selected governance owner differs from actual kernel",
                ));
            }
        } else if self.reference_due_process.is_some() || reference_selected(kernel) {
            let capabilities =
                self.reference_due_process
                    .as_ref()
                    .ok_or(RuntimeError::Governance(
                        wos_core::model::due_process::RefusalCode::UnsupportedProfile,
                    ))?;
            super::governance::admission::validate(capabilities, kernel)
                .map_err(RuntimeError::Governance)?;
        }
        Ok(())
    }

    pub(super) fn validate_governance_maintenance(
        &self,
        prior: &RuntimeRecord,
        proposed: &RuntimeRecord,
    ) -> Result<(), RuntimeError> {
        if self.generic_governance.is_some()
            && prior.process.case_state != proposed.process.case_state
        {
            return Err(refused("maintenance cannot change governed case data"));
        }
        Ok(())
    }

    pub(super) fn check_governance_result(
        &self,
        context: RuntimeGovernanceResultContext<'_>,
    ) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        self.validate_governance_selection(context.kernel)?;
        if reference_selected(context.kernel) {
            return Ok(Vec::new());
        }
        let result = if let Some(selected) = &self.generic_governance {
            selected.check_result(context)?
        } else {
            crate::governance::GovernanceResult::without_required_governance(context)?
        };
        result.validate_coverage(&context)?;
        result.into_provenance(
            &super::process_minter(context.proposed_process)?,
            context.now_iso,
        )
    }

    pub(super) fn validate_governance_output_bindings(
        &self,
        binding: &crate::integration::IntegrationBinding,
    ) -> Result<(), RuntimeError> {
        let Some(selected) = &self.generic_governance else {
            return Ok(());
        };
        let Some(due) = &selected.policy.selection().due_process else {
            return Ok(());
        };
        for target in binding.output_binding.keys() {
            let raw = target.strip_prefix("caseFile.").unwrap_or(target);
            let segments = raw.split('.').collect::<Vec<_>>();
            if segments
                .iter()
                .any(|part| part.is_empty() || part.contains(['[', ']']))
            {
                return Err(refused("unsupported governed output target"));
            }
            for protected in &due.protected_effects {
                let protected_segments =
                    crate::governance::protected_pointer_segments(&protected.path)?;
                if segments
                    .iter()
                    .map(|s| *s)
                    .zip(protected_segments.iter().map(String::as_str))
                    .all(|(a, b)| a == b)
                {
                    return Err(refused(
                        "integration output overlaps a governed protected path",
                    ));
                }
            }
        }
        Ok(())
    }
}
fn reference_selected(kernel: &KernelDocument) -> bool {
    kernel
        .governance
        .as_ref()
        .and_then(|g| g.pointer("/dueProcess/execution"))
        .is_some()
}
fn refused(code: &str) -> RuntimeError {
    RuntimeError::GovernancePolicy(code.into())
}
