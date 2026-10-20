// Rust guideline compliant 2026-02-21

//! Validates governance against actual native transitions before runtime effects.
//!
//! A selected policy consumes complete retained records. Human evidence remains
//! an attributable judgment. Resource authority and actual delivery qualification
//! remain owning-host responsibilities; this module never invents either.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use wos_core::eval::ObservedTransition;
use wos_core::instance::{PendingEvent, WorkflowProcess};
use wos_core::{GovernanceDocument, KernelDocument, ProvenanceKind, ProvenanceRecord};
use wos_events::governance::{
    AdverseNoticeProposal, CounterfactualEvidence, CounterfactualPolarity, DecisionEvidence,
    ReasoningEvidence, RuleReference,
};

use crate::runtime::RuntimeError;
use crate::store::{RuntimeRecord, TaskArtifactKind};

/// Actual immutable evaluation inputs, before effects or native persistence.
///
/// Every transition is supplied, including nested and continuous transitions.
/// `prior` is the complete record loaded before timer/event materialization.
#[derive(Clone, Copy)]
pub struct RuntimeGovernanceResultContext<'a> {
    /// Actual selected kernel.
    pub kernel: &'a KernelDocument,
    /// Complete retained process, artifacts and provenance before this drain.
    pub prior: &'a RuntimeRecord,
    /// Exact event consumed by the single evaluator.
    pub event: &'a PendingEvent,
    /// Evaluated process before side-effect realization.
    pub proposed_process: &'a WorkflowProcess,
    /// All transitions actually observed in this drain.
    pub transitions: &'a [ObservedTransition],
    /// Actual new Facts and other native records produced in this drain.
    pub provenance: &'a [ProvenanceRecord],
    /// Original command clock, retained on replay.
    pub now_ms: u64,
    /// RFC 3339 rendering of the retained clock.
    pub now_iso: &'a str,
}

/// Exact authored transition selected by a finite governance binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceTransition {
    /// Original global source state identifier.
    pub from: String,
    /// Original global target state identifier.
    pub to: String,
    /// Original authored or actual native event label.
    pub event: String,
}

/// Human decision evidence selected from an accepted native task artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HumanDecisionBinding {
    /// Actual transition, not a state-name approximation.
    pub transition: GovernanceTransition,
    /// Task declaration whose accepted artifact supplies the judgment.
    pub task_ref: String,
    /// JSON pointer to structured DecisionEvidence in the accepted response.
    pub evidence_pointer: String,
    /// Optional pinned owning Mapping applied to the unchanged accepted Response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_mapping: Option<GovernanceEvidenceMapping>,
    /// Exact permitted rule references from the selected source policy.
    pub rules: Vec<RuleReference>,
    /// Complete selected criterion identifiers.
    pub criteria: Vec<String>,
    /// Snapshot paths this finite policy permits as decision evidence.
    pub case_field_paths: Vec<String>,
    /// Controllable criterion/factor identifiers permitted for positive evidence.
    pub positive_factors: Vec<String>,
    /// Irrelevant factors the actual human review must explicitly exclude.
    pub negative_factors: Vec<String>,
}

/// Immutable owning Mapping selected by the workflow's decision policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceEvidenceMapping {
    /// Existing selected Formspec Mapping reference.
    #[serde(rename = "ref")]
    pub reference: String,
    /// Exact raw-file SHA-256 from the existing selected processor.
    pub sha256: String,
}

/// Read-only normalized evidence from an exact original accepted artifact.
///
/// The current binding owner applies the selected maintained Mapping outside
/// effect locks. The original signed Response remains unchanged and retained.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturedGovernanceDecision {
    /// Exact original accepted artifact identifier.
    pub artifact_id: String,
    /// Canonical digest of the complete unchanged original Response.
    pub response_sha256: String,
    /// Existing selected Mapping reference.
    pub mapping_ref: String,
    /// Exact selected Mapping's raw-file digest.
    pub mapping_sha256: String,
    /// Maintained Mapping output; validated as closed DecisionEvidence.
    pub evidence: Value,
}

/// A business-field value that cannot become adverse without its original notice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectedGovernanceEffect {
    /// Exact JSON pointer into the actual case file.
    pub path: String,
    /// Finite selected values representing effective adverse treatment.
    pub adverse_values: Vec<Value>,
}

/// Finite before-effective notice capability selected by the workflow owner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DueProcessBinding {
    /// Owning delivery callback binding, independently qualified by the host.
    pub notice_binding_id: String,
    /// Actual transitions allowed to make the original proposal effective.
    pub effective_transitions: Vec<GovernanceTransition>,
    /// Complete selected adverse business writes, checked also on task Mapping.
    pub protected_effects: Vec<ProtectedGovernanceEffect>,
}

/// A qualified historical receipt captured by the existing notice owner.
///
/// This is protected host input, not an HTTP-body assertion. The host retains
/// its exact owning admission and rechecks it with the complete prior fence.
/// Runtime checks the actual retained callback, content and proposal association;
/// this structure does not itself grant delivery or resource authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturedGovernanceNotice {
    /// Actual original process and case association.
    pub process_id: String,
    /// Original durable case ledger.
    pub case_ledger_id: String,
    /// Exact selected original callback binding.
    pub binding_id: String,
    /// Original immutable intent identifier.
    pub intent_id: String,
    /// Original immutable delivery attempt.
    pub attempt_id: String,
    /// Retained CallbackReceived provenance identifier.
    pub callback_record_ref: String,
    /// Digest of that exact native callback record's data.
    pub callback_data_sha256: String,
    /// Digest of the persisted, qualified original clock decision.
    pub date_decision_sha256: String,
    /// Original-profile logical delivery time, not current physical/firing time.
    pub governed_sent_at: String,
    /// Actual exact rendered body covered by the recorded intent.
    pub human_readable: String,
    /// Exact deterministic proposal content digest, absent for a request notice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_content_sha256: Option<String>,
}

/// Immutable, bounded per-preparation facts supplied by existing current owners.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturedGovernanceEvidence {
    /// Actual retained receipts; an empty list never means delivery occurred.
    pub notices: Vec<CapturedGovernanceNotice>,
    /// Normalized decisions remain bound to their exact original artifacts.
    #[serde(default)]
    pub decisions: Vec<CapturedGovernanceDecision>,
}

/// Finite no-response rule executed from actual time and complete retained artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimeoutDecisionBinding {
    /// Actual unconditional native timeout transition.
    pub transition: GovernanceTransition,
    /// Original retained native timer identifier.
    pub timer_id: String,
    /// Selected whole-calendar-day response period, such as `P90D`.
    pub response_period: String,
    /// Actual qualified original request-notice binding.
    pub request_notice_binding_id: String,
    /// Complete correction-task declarations relevant to this no-response rule.
    pub correction_task_refs: Vec<String>,
    /// Explicit selected demo rule; this does not assert a legal source.
    pub rule: RuleReference,
    /// Actual maintained late-evidence/reconsideration route described by the rule.
    pub reconsideration_route: String,
    /// Factors excluded by this finite rule's restricted input dependencies.
    pub excluded_factors: Vec<String>,
}

/// Actual filing and independent human review for the selected demo appeal rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppealBinding {
    /// Explicit selected policy label; this does not assert a legal source.
    pub policy_label: String,
    /// Authored name of the independent reconsideration body.
    pub review_body: String,
    /// Authored filing instructions included in the original notice.
    pub filing_instructions: String,
    /// Actual transition consuming a real applicant filing task completion.
    pub filed_transition: GovernanceTransition,
    /// Authored applicant filing task.
    pub filed_task_ref: String,
    /// Pointer to the original proposal Facts reference in its accepted artifact.
    pub proposal_reference_pointer: String,
    /// Authored independently assigned human review task.
    pub review_task_ref: String,
    /// Workflow-local declared human selected separately from the original reviewer.
    pub reviewer_actor: String,
    /// Exact actual review transitions, also covered by human decision bindings.
    pub review_transitions: Vec<GovernanceTransition>,
}

/// Closed startup selection for the initial attributable human-decision slice.
///
/// Only supported actual transition bindings are admitted. System timeout,
/// notice effectiveness and independent-appeal bindings must be added through
/// their reviewed finite owners before a workflow requiring them is admitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernancePolicySelection {
    /// Owning selection version.
    pub version: u8,
    /// Exact workflow bytes parsed by the host's pinned document loader.
    pub kernel: Value,
    /// Exact independently selected Delivery document.
    pub delivery: Value,
    /// Complete human determination coverage for the admitted selection.
    pub human_decisions: Vec<HumanDecisionBinding>,
    /// Explicit supported before-effective capability; omitted policies cannot promise it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_process: Option<DueProcessBinding>,
    /// Explicit actual no-response rules; no synthetic human evidence is created.
    #[serde(default)]
    pub timeout_decisions: Vec<TimeoutDecisionBinding>,
    /// Actual independently assigned administrative reconsideration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appeal: Option<AppealBinding>,
}

/// Immutable selected governance that cannot be constructed without validation.
#[derive(Debug, Clone)]
pub struct QualifiedGovernancePolicy {
    selection: GovernancePolicySelection,
    kernel: KernelDocument,
    fingerprint: String,
}

impl QualifiedGovernancePolicy {
    /// Qualifies exact selected documents and every supported decision binding.
    ///
    /// # Errors
    /// Refuses malformed policy, unsupported promises, foreign Delivery targets,
    /// duplicate bindings and incomplete determination coverage. Required notice,
    /// timeout and appeal obligations must select their supported finite owners.
    pub fn qualify(selection: GovernancePolicySelection) -> Result<Self, RuntimeError> {
        require(
            selection.version == 1,
            "unsupported governance selection version",
        )?;
        let kernel: KernelDocument = serde_json::from_value(selection.kernel.clone())
            .map_err(|_| refused("invalid selected kernel"))?;
        let raw = selection
            .kernel
            .get("governance")
            .filter(|value| !value.is_null())
            .ok_or_else(|| refused("governance missing"))?;
        require(
            raw.as_object().is_some_and(|fields| {
                fields
                    .keys()
                    .all(|key| matches!(key.as_str(), "audit" | "taskCatalog" | "dueProcess"))
            }),
            "governance declaration requires an unavailable capability",
        )?;
        let audit = raw.get("audit");
        if let Some(tags) = audit.and_then(|value| value.pointer("/reasoningTier/requiredForTags"))
        {
            require(
                tags == &serde_json::json!(["determination"]),
                "reasoning tag selection unsupported",
            )?;
        }
        if let Some(tags) =
            audit.and_then(|value| value.pointer("/counterfactualTier/requiredForTags"))
        {
            require(
                tags == &serde_json::json!(["adverse-decision"]),
                "counterfactual tag selection unsupported",
            )?;
        }
        require(
            audit
                .and_then(|value| value.pointer("/reasoningTier/requireDecisionRequirements"))
                .is_none_or(|value| value == &Value::Bool(false)),
            "decision requirements capability not installed",
        )?;
        let governance: GovernanceDocument = serde_json::from_value(raw.clone())
            .map_err(|_| refused("invalid canonical governance"))?;
        require(
            selection
                .delivery
                .get("targetWorkflow")
                .and_then(Value::as_str)
                == kernel.url.as_deref(),
            "Delivery target differs",
        )?;
        let requires_due_process = matches!(
            selection.kernel.get("impactLevel").and_then(Value::as_str),
            Some("rights-impacting" | "safety-impacting")
        ) || governance.due_process.is_some();
        require(
            !requires_due_process || selection.due_process.is_some(),
            "required before-effective governance capability missing",
        )?;
        require(
            selection.due_process.is_some()
                || (selection.appeal.is_none() && selection.timeout_decisions.is_empty()),
            "timeout or appeal capability requires before-effective selection",
        )?;
        require(
            !selection.human_decisions.is_empty() || !selection.timeout_decisions.is_empty(),
            "decision bindings missing",
        )?;
        let mut seen = BTreeSet::new();
        for binding in &selection.human_decisions {
            require(
                seen.insert(
                    serde_json::to_string(&binding.transition)
                        .map_err(|_| refused("transition cannot serialize"))?,
                ),
                "duplicate decision binding",
            )?;
            require(
                !binding.task_ref.trim().is_empty()
                    && binding.evidence_pointer.starts_with('/')
                    && !binding.rules.is_empty()
                    && !binding.criteria.is_empty(),
                "decision source incomplete",
            )?;
            require_unique_nonblank(&binding.criteria)?;
            require_unique_nonblank(&binding.positive_factors)?;
            require_unique_nonblank(&binding.negative_factors)?;
            let rules: Vec<_> = binding
                .rules
                .iter()
                .map(|rule| rule.rule_id.clone())
                .collect();
            require_unique_nonblank(&rules)?;
            require(
                binding
                    .case_field_paths
                    .iter()
                    .all(|path| path.starts_with('/')),
                "evidence paths must be JSON pointers",
            )?;
        }
        let mut declared = Vec::new();
        let mut state_ids = BTreeSet::new();
        for (id, state) in &kernel.lifecycle.states {
            collect_required_transitions(id, state, &mut declared, &mut state_ids)?;
        }
        let covered = selection
            .human_decisions
            .iter()
            .map(|binding| &binding.transition)
            .chain(
                selection
                    .timeout_decisions
                    .iter()
                    .map(|binding| &binding.transition),
            )
            .collect::<Vec<_>>();
        require(
            declared.len() == covered.len()
                && declared.iter().all(|transition| {
                    covered
                        .iter()
                        .filter(|bound| ***bound == *transition)
                        .count()
                        == 1
                }),
            "declared determination coverage differs",
        )?;
        for binding in &selection.human_decisions {
            if let Some(mapping) = &binding.evidence_mapping {
                require(
                    !mapping.reference.trim().is_empty()
                        && mapping
                            .sha256
                            .strip_prefix("sha256:")
                            .is_some_and(|digest| {
                                digest.len() == 64
                                    && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                            }),
                    "selected decision Mapping pin invalid",
                )?;
            }
        }
        qualify_due_process(&selection, &kernel)?;
        let fingerprint = stack_common_hash::canonical_json_hash(
            &serde_json::to_value(&selection).map_err(|_| refused("selection cannot serialize"))?,
        )
        .map_err(|_| refused("selection cannot canonicalize"))?;
        Ok(Self {
            selection,
            kernel,
            fingerprint,
        })
    }

    /// Returns exact owned inputs for original command capture and replay checks.
    #[must_use]
    pub fn selection(&self) -> &GovernancePolicySelection {
        &self.selection
    }

    /// Returns the immutable selected-policy fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub(crate) fn selected(self, evidence: CapturedGovernanceEvidence) -> SelectedGovernancePolicy {
        SelectedGovernancePolicy {
            policy: self,
            evidence,
        }
    }

    pub(crate) fn matches_kernel(&self, kernel: &KernelDocument) -> bool {
        serde_json::to_value(kernel).ok() == serde_json::to_value(&self.kernel).ok()
    }

    pub(crate) fn kernel(&self) -> &KernelDocument {
        &self.kernel
    }

    fn proposal_notice(
        &self,
        context: &RuntimeGovernanceResultContext<'_>,
        decision: &DecisionAudit,
    ) -> Result<AdverseNoticeProposal, RuntimeError> {
        let policy = self
            .selection
            .kernel
            .pointer("/governance/dueProcess/adverseDecisionPolicy")
            .ok_or_else(|| refused("adverse notice policy missing"))?;
        let template_key = policy["noticeTemplateKey"]
            .as_str()
            .ok_or_else(|| refused("adverse notice template missing"))?;
        let template = self
            .selection
            .delivery
            .pointer("/notifications/templates")
            .and_then(|templates| templates.get(template_key))
            .ok_or_else(|| refused("canonical adverse template missing"))?;
        let facts = context
            .provenance
            .iter()
            .find(|record| record.id == decision.reasoning.facts_record_ref)
            .ok_or_else(|| refused("original proposal Facts missing"))?;
        let counterfactual = decision
            .counterfactual
            .as_ref()
            .ok_or_else(|| refused("adverse explanation missing"))?;
        let appeal = self
            .selection
            .appeal
            .as_ref()
            .ok_or_else(|| refused("appeal selection missing"))?;
        let period = appeal_period(&self.selection)?;
        let mut variables = BTreeMap::from([
            (
                "caseId".into(),
                context.prior.process.case_ledger_id.clone(),
            ),
            (
                "determination".into(),
                facts.to_state.clone().unwrap_or_default(),
            ),
            ("reason".into(), decision.reasoning.reason.clone()),
            (
                "snapshotSha256".into(),
                decision.reasoning.snapshot_sha256.clone(),
            ),
            (
                "appealWindow".into(),
                format!(
                    "{} calendar days from qualifying original delivery ({})",
                    calendar_period_ms(period)? / 86_400_000,
                    appeal.policy_label
                ),
            ),
            ("appealBody".into(), appeal.review_body.clone()),
        ]);
        variables.insert(
            "appealInstructions".into(),
            appeal.filing_instructions.clone(),
        );
        variables.insert(
            "positiveCounterfactuals".into(),
            counterfactual
                .positive_counterfactuals
                .iter()
                .map(|item| format!("{}: {}", item.factor, item.effect))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        variables.insert(
            "negativeCounterfactuals".into(),
            counterfactual
                .negative_counterfactuals
                .iter()
                .map(|item| format!("{}: {}", item.factor, item.effect))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let mut parts = Vec::new();
        if let Some(subject) = template["subject"].as_str() {
            parts.push(render_governance_text(subject, &variables)?);
        }
        let sections = template["sections"]
            .as_array()
            .ok_or_else(|| refused("notice sections missing"))?;
        for section in sections {
            let text = section["content"]
                .as_str()
                .ok_or_else(|| refused("notice section content missing"))?;
            let title = section["title"]
                .as_str()
                .or_else(|| section["id"].as_str())
                .unwrap_or("");
            parts.push(format!(
                "{}\n{}",
                title,
                render_governance_text(text, &variables)?
            ));
        }
        let human_readable = parts.join("\n\n");
        // The actual individualized explanation cannot disappear behind a generic template.
        require(
            human_readable.contains(&decision.reasoning.reason)
                && counterfactual
                    .positive_counterfactuals
                    .iter()
                    .all(|item| human_readable.contains(&item.effect))
                && counterfactual
                    .negative_counterfactuals
                    .iter()
                    .all(|item| human_readable.contains(&item.effect)),
            "notice omits the actual individualized explanation",
        )?;
        let machine_readable = serde_json::json!({"kind":"adverseDecisionNotice",
            "proposalFactsRef":facts.id,"snapshot":facts.case_file_snapshot,
            "reasoning":decision.reasoning,"counterfactual":counterfactual,
            "appeal":{"policy":appeal.policy_label,"window":period,
                "anchor":"qualifyingOriginalDelivery","continuationOfServices":false}});
        let content_sha256 = stack_common_hash::canonical_json_hash(&serde_json::json!({
            "machineReadable":machine_readable,"humanReadable":human_readable}))
        .map_err(|_| refused("notice content cannot canonicalize"))?;
        Ok(AdverseNoticeProposal {
            facts_record_ref: facts.id.clone(),
            snapshot_sha256: decision.reasoning.snapshot_sha256.clone(),
            template_key: template_key.into(),
            template_sha256: stack_common_hash::canonical_json_hash(template)
                .map_err(|_| refused("template cannot canonicalize"))?,
            machine_readable,
            human_readable,
            content_sha256,
        })
    }

    fn check_effectiveness(
        &self,
        context: &RuntimeGovernanceResultContext<'_>,
        due: &DueProcessBinding,
        evidence: &CapturedGovernanceEvidence,
    ) -> Result<(), RuntimeError> {
        let changed = due.protected_effects.iter().any(|field| {
            // Creation has no retained prior decision, even when initial data or
            // pure initial actions already supplied an adverse value.
            let before = if context.prior.provenance_log.is_empty() {
                None
            } else {
                context.prior.process.case_state.pointer(&field.path)
            };
            let after = context.proposed_process.case_state.pointer(&field.path);
            before != after && after.is_some_and(|value| field.adverse_values.contains(value))
        });
        if !changed {
            return Ok(());
        }
        require(
            required_facts(context)?.iter().all(|facts| {
                !facts
                    .transition_tags
                    .iter()
                    .any(|tag| tag == "adverse-decision")
            }),
            "a new adverse proposal cannot become effective in the same evaluation",
        )?;
        require(
            context.transitions.iter().any(|transition| {
                due.effective_transitions.iter().any(|selected| {
                    selected.from == transition.from
                        && selected.to == transition.to
                        && selected.event == transition.event
                })
            }),
            "adverse business write is outside its selected effective transition",
        )?;
        // The complete ordered retained decision history identifies the live proposal.
        // A later determination supersedes the old one; a receipt cannot adopt an old denial.
        let facts = context
            .prior
            .provenance_log
            .iter()
            .rev()
            .find(|record| {
                record.record_kind == ProvenanceKind::StateTransition
                    && record
                        .transition_tags
                        .iter()
                        .any(|tag| tag == "determination")
            })
            .ok_or_else(|| refused("effective decision has no retained original determination"))?;
        require(
            facts
                .transition_tags
                .iter()
                .any(|tag| tag == "adverse-decision"),
            "effective receipt refers to a superseded adverse determination",
        )?;
        let mut reasons = context.prior.provenance_log.iter().filter_map(|record| {
            if record.record_kind != ProvenanceKind::ReasoningTierRecorded {
                return None;
            }
            let reason: ReasoningEvidence = serde_json::from_value(record.data.clone()?).ok()?;
            (reason.facts_record_ref == facts.id).then_some(reason)
        });
        let reason = reasons
            .next()
            .ok_or_else(|| refused("original proposal Reasoning missing"))?;
        require(
            reasons.next().is_none(),
            "original proposal Reasoning ambiguous",
        )?;
        let proposal = reason
            .adverse_notice
            .ok_or_else(|| refused("original deterministic proposal missing"))?;
        let expected_snapshot = facts
            .case_file_snapshot
            .as_ref()
            .ok_or_else(|| refused("original proposal snapshot missing"))?;
        require(
            proposal.facts_record_ref == facts.id
                && proposal.snapshot_sha256 == expected_snapshot.sha256
                && stack_common_hash::canonical_json_hash(
                    &serde_json::json!({"machineReadable":proposal.machine_readable,
                "humanReadable":proposal.human_readable}),
                )
                .ok()
                .as_ref()
                    == Some(&proposal.content_sha256),
            "original proposal content differs",
        )?;
        let receipts = evidence
            .notices
            .iter()
            .filter(|receipt| {
                receipt.binding_id == due.notice_binding_id
                    && receipt.proposal_content_sha256.as_deref()
                        == Some(proposal.content_sha256.as_str())
                    && receipt.human_readable == proposal.human_readable
            })
            .collect::<Vec<_>>();
        require(
            receipts.len() == 1,
            "original proposal has no unique qualified actual receipt",
        )?;
        let sent_at = validate_captured_notice(context, receipts[0])?;
        let grace = self
            .selection
            .kernel
            .pointer("/governance/dueProcess/adverseDecisionPolicy/noticeGracePeriod")
            .and_then(Value::as_str)
            .map(wos_core::eval::parse_iso_duration_to_ms)
            .transpose()
            .map_err(|_| refused("notice grace period invalid"))?
            .unwrap_or(0);
        require(
            context.now_ms
                >= sent_at
                    .checked_add(grace)
                    .ok_or_else(|| refused("notice grace deadline overflow"))?,
            "original qualified notice grace period has not elapsed",
        )
    }

    fn check_appeals(
        &self,
        context: &RuntimeGovernanceResultContext<'_>,
        evidence: &CapturedGovernanceEvidence,
        decisions: &[DecisionAudit],
    ) -> Result<Vec<AppealAudit>, RuntimeError> {
        let Some(binding) = &self.selection.appeal else {
            return Ok(Vec::new());
        };
        let mut filed = Vec::new();
        if context
            .transitions
            .iter()
            .any(|actual| transition_matches_observed(&binding.filed_transition, actual))
        {
            let artifact = accepted_event_artifact(context, &binding.filed_task_ref)?;
            let proposal_ref = artifact
                .response
                .pointer(&binding.proposal_reference_pointer)
                .and_then(Value::as_str)
                .ok_or_else(|| refused("appeal filing lacks the original proposal reference"))?;
            let (facts, proposal) = retained_proposal(context.prior, proposal_ref)?;
            let due = self
                .selection
                .due_process
                .as_ref()
                .ok_or_else(|| refused("appeal due-process selection missing"))?;
            let receipts = evidence
                .notices
                .iter()
                .filter(|receipt| {
                    receipt.binding_id == due.notice_binding_id
                        && receipt.proposal_content_sha256.as_deref()
                            == Some(proposal.content_sha256.as_str())
                        && receipt.human_readable == proposal.human_readable
                })
                .collect::<Vec<_>>();
            require(
                receipts.len() == 1,
                "appeal original notice receipt missing or ambiguous",
            )?;
            let anchor = validate_captured_notice(context, receipts[0])?;
            let deadline = anchor
                .checked_add(calendar_period_ms(appeal_period(&self.selection)?)?)
                .ok_or_else(|| refused("appeal deadline overflow"))?;
            let accepted = governance_timestamp(&artifact.recorded_at)?;
            require(
                accepted >= anchor && accepted <= deadline,
                "appeal filing is outside its original delivery window",
            )?;
            require(
                !context.prior.provenance_log.iter().any(|record| {
                    record.record_kind == ProvenanceKind::AppealFiled
                        && record
                            .data
                            .as_ref()
                            .is_some_and(|data| data["originalProposalFactsRef"] == facts.id)
                }),
                "original proposal already has a retained filing",
            )?;
            filed.push(AppealAudit {actor:artifact.actor_id.clone(),event:context.event.event.clone(),
                data:serde_json::json!({"originalProposalFactsRef":facts.id,"filingArtifactRef":artifact.artifact_id,
                    "originalNoticeCallbackRef":receipts[0].callback_record_ref,"originalNoticeContentSha256":proposal.content_sha256,
                    "governedFiledAt":artifact.recorded_at,"windowRule":appeal_period(&self.selection)?,"windowDeadline":governance_time(deadline)?,
                    "reviewTaskRef":binding.review_task_ref,"reviewerActor":binding.reviewer_actor,
                    "policy":binding.policy_label})});
        }
        for decision in decisions.iter().filter(|decision| {
            binding.review_transitions.iter().any(|transition| {
                context
                    .provenance
                    .iter()
                    .find(|record| record.id == decision.reasoning.facts_record_ref)
                    .is_some_and(|facts| transition_matches_facts(transition, facts))
            })
        }) {
            let artifact = accepted_event_artifact(context, &binding.review_task_ref)?;
            let proposal_ref = artifact
                .response
                .pointer(&binding.proposal_reference_pointer)
                .and_then(Value::as_str)
                .ok_or_else(|| refused("appeal review lacks original proposal reference"))?;
            let (original, _) = retained_proposal(context.prior, proposal_ref)?;
            require(
                original.actor_id.as_ref() != Some(&artifact.actor_id),
                "original decision maker cannot review the appeal",
            )?;
            let assigned = context
                .prior
                .process
                .actor_bindings
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter(|actor| actor.declared_actor == binding.reviewer_actor)
                .collect::<Vec<_>>();
            require(
                assigned.len() == 1
                    && assigned[0].assigned_actor == artifact.actor_id
                    && decision.actor.as_ref() == Some(&artifact.actor_id),
                "appeal review is not the independently assigned human",
            )?;
            let filings = context
                .prior
                .provenance_log
                .iter()
                .filter(|record| {
                    record.record_kind == ProvenanceKind::AppealFiled
                        && record.data.as_ref().is_some_and(|data| {
                            data["originalProposalFactsRef"] == original.id
                                && data["reviewTaskRef"] == binding.review_task_ref
                                && data["reviewerActor"] == binding.reviewer_actor
                        })
                })
                .collect::<Vec<_>>();
            require(
                filings.len() == 1,
                "appeal review lacks one retained valid filing",
            )?;
            let data = filings[0].data.as_ref().expect("filing data checked");
            let filing_ref = data["filingArtifactRef"]
                .as_str()
                .ok_or_else(|| refused("original filing artifact missing"))?;
            let filing = context
                .prior
                .artifacts
                .get(filing_ref)
                .ok_or_else(|| refused("original filed artifact is not retained"))?;
            require(
                filing.kind == TaskArtifactKind::Accepted
                    && filing.actor_id != artifact.actor_id
                    && filing.artifact_id == filing_ref
                    && filing.actor_id == filings[0].actor_id.as_deref().unwrap_or("")
                    && data["governedFiledAt"] == filing.recorded_at
                    && governance_timestamp(&filing.recorded_at)?
                        <= governance_timestamp(data["windowDeadline"].as_str().unwrap_or(""))?,
                "retained appeal filing association differs",
            )?;
        }
        Ok(filed)
    }

    fn check_result(
        &self,
        context: RuntimeGovernanceResultContext<'_>,
        evidence: &CapturedGovernanceEvidence,
    ) -> Result<GovernanceResult, RuntimeError> {
        require(
            serde_json::to_value(context.kernel).ok() == serde_json::to_value(&self.kernel).ok(),
            "runtime kernel differs from selected policy",
        )?;
        require(
            context.prior.process.process_id == context.proposed_process.process_id
                && context.prior.process.case_ledger_id == context.proposed_process.case_ledger_id,
            "governance process association differs",
        )?;
        let mut decisions = Vec::new();
        for facts in required_facts(&context)? {
            let bindings: Vec<_> = self
                .selection
                .human_decisions
                .iter()
                .filter(|binding| {
                    binding.transition.from == facts.from_state.as_deref().unwrap_or("")
                        && binding.transition.to == facts.to_state.as_deref().unwrap_or("")
                        && Some(binding.transition.event.as_str())
                            == facts
                                .data
                                .as_ref()
                                .and_then(|data| data.get("transitionEvent"))
                                .and_then(Value::as_str)
                })
                .collect();
            let timeouts = self
                .selection
                .timeout_decisions
                .iter()
                .filter(|binding| transition_matches_facts(&binding.transition, facts))
                .collect::<Vec<_>>();
            require(
                bindings.len() + timeouts.len() == 1,
                "actual determination has no unique capability binding",
            )?;
            decisions.push(if let Some(binding) = bindings.first() {
                human_decision(&context, facts, binding, evidence)?
            } else {
                timeout_decision(&context, facts, timeouts[0], evidence)?
            });
        }
        if let Some(due_process) = &self.selection.due_process {
            for decision in &mut decisions {
                if decision.counterfactual.is_some() {
                    decision.reasoning.adverse_notice =
                        Some(self.proposal_notice(&context, decision)?);
                }
            }
            self.check_effectiveness(&context, due_process, evidence)?;
        }
        let appeals = self.check_appeals(&context, evidence, &decisions)?;
        Ok(GovernanceResult { decisions, appeals })
    }
}

/// Private post-result owner; the existing AI pre-event hook remains separate.
pub(crate) struct SelectedGovernancePolicy {
    pub(crate) policy: QualifiedGovernancePolicy,
    evidence: CapturedGovernanceEvidence,
}
impl SelectedGovernancePolicy {
    pub(crate) fn check_result(
        &self,
        context: RuntimeGovernanceResultContext<'_>,
    ) -> Result<GovernanceResult, RuntimeError> {
        self.policy.check_result(context, &self.evidence)
    }
}

struct AppealAudit {
    actor: String,
    event: String,
    data: Value,
}

struct DecisionAudit {
    actor: Option<String>,
    event: String,
    reasoning: ReasoningEvidence,
    counterfactual: Option<CounterfactualEvidence>,
}

/// Closed checked result; only the owning policy can supply required evidence.
#[derive(Default)]
pub struct GovernanceResult {
    decisions: Vec<DecisionAudit>,
    appeals: Vec<AppealAudit>,
}

impl GovernanceResult {
    /// Continues only when no actual transition requires governance evidence.
    ///
    /// # Errors
    /// Refuses determination or adverse-decision tags under the default policy.
    pub fn without_required_governance(
        context: RuntimeGovernanceResultContext<'_>,
    ) -> Result<Self, RuntimeError> {
        require(
            context.kernel.governance.as_ref().and_then(|governance| {
                governance.pointer("/dueProcess/adverseDecisionPolicy/noticeRequired")
            }) != Some(&Value::Bool(true)),
            "required before-effective governance capability not installed",
        )?;
        require(
            required_facts(&context)?.is_empty(),
            "required governance capability not installed",
        )?;
        Ok(Self::default())
    }

    pub(crate) fn validate_coverage(
        &self,
        context: &RuntimeGovernanceResultContext<'_>,
    ) -> Result<(), RuntimeError> {
        let facts = required_facts(context)?;
        require(
            facts.len() == self.decisions.len(),
            "governance result coverage differs",
        )?;
        for record in facts {
            let decisions: Vec<_> = self
                .decisions
                .iter()
                .filter(|decision| decision.reasoning.facts_record_ref == record.id)
                .collect();
            require(
                decisions.len() == 1,
                "governance result is missing or duplicated",
            )?;
            let decision = decisions[0];
            let snapshot = record
                .case_file_snapshot
                .as_ref()
                .expect("snapshot checked");
            require(
                decision.reasoning.snapshot_sha256 == snapshot.sha256
                    && decision.actor == record.actor_id,
                "governance result Facts association differs",
            )?;
            if record
                .transition_tags
                .iter()
                .any(|tag| tag == "adverse-decision")
            {
                let cf = decision
                    .counterfactual
                    .as_ref()
                    .ok_or_else(|| refused("governance result counterfactuals missing"))?;
                require(
                    cf.facts_record_ref == record.id
                        && cf.snapshot_sha256 == snapshot.sha256
                        && cf.source_ref == decision.reasoning.source_ref,
                    "counterfactual result Facts association differs",
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn into_provenance(self, now: &str) -> Result<Vec<ProvenanceRecord>, RuntimeError> {
        let mut records = Vec::new();
        for decision in self.decisions {
            let mut record = ProvenanceRecord::blank(ProvenanceKind::ReasoningTierRecorded);
            record.actor_id = decision.actor.clone();
            record.event = Some(decision.event.clone());
            record.timestamp = now.to_owned();
            record.data = Some(
                serde_json::to_value(decision.reasoning)
                    .map_err(|_| refused("reasoning cannot serialize"))?,
            );
            records.push(record);
            if let Some(counterfactual) = decision.counterfactual {
                let mut record =
                    ProvenanceRecord::blank(ProvenanceKind::CounterfactualTierRecorded);
                record.actor_id = decision.actor;
                record.event = Some(decision.event);
                record.timestamp = now.to_owned();
                record.data = Some(
                    serde_json::to_value(counterfactual)
                        .map_err(|_| refused("counterfactual cannot serialize"))?,
                );
                records.push(record);
            }
        }
        for appeal in self.appeals {
            let mut record = ProvenanceRecord::blank(ProvenanceKind::AppealFiled);
            record.actor_id = Some(appeal.actor);
            record.event = Some(appeal.event);
            record.timestamp = now.into();
            record.data = Some(appeal.data);
            records.push(record);
        }
        Ok(records)
    }
}

fn required_facts<'a>(
    context: &'a RuntimeGovernanceResultContext<'_>,
) -> Result<Vec<&'a ProvenanceRecord>, RuntimeError> {
    let required: Vec<_> = context
        .transitions
        .iter()
        .filter(|transition| {
            transition
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "determination" | "adverse-decision"))
        })
        .collect();
    let mut found = Vec::new();
    let mut used = BTreeSet::new();
    for transition in required {
        require(
            transition.tags.iter().any(|tag| tag == "determination"),
            "adverse decision lacks determination Facts",
        )?;
        let candidates: Vec<_> = context
            .provenance
            .iter()
            .filter(|record| {
                record.record_kind == ProvenanceKind::StateTransition
                    && record.id == transition.provenance_record_id
                    && record.from_state.as_deref() == Some(&transition.from)
                    && record.to_state.as_deref() == Some(&transition.to)
                    && record
                        .data
                        .as_ref()
                        .and_then(|data| data.get("transitionEvent"))
                        .and_then(Value::as_str)
                        == Some(&transition.event)
                    && record.transition_tags == transition.tags
            })
            .collect();
        require(
            candidates.len() == 1,
            "determination Facts missing or ambiguous",
        )?;
        let facts = candidates[0];
        require(used.insert(&facts.id), "determination Facts repeated")?;
        let snapshot = facts
            .case_file_snapshot
            .as_ref()
            .ok_or_else(|| refused("determination snapshot missing"))?;
        require(
            wos_events::CaseFileSnapshot::from_case_state(&snapshot.value) == *snapshot,
            "determination snapshot hash differs",
        )?;
        found.push(facts);
    }
    // No new tagged Facts may be dropped by an incomplete observed-transition list.
    require(
        context
            .provenance
            .iter()
            .filter(|record| {
                record.record_kind == ProvenanceKind::StateTransition
                    && record
                        .transition_tags
                        .iter()
                        .any(|tag| matches!(tag.as_str(), "determination" | "adverse-decision"))
            })
            .all(|record| used.contains(&record.id)),
        "unaccounted determination Facts",
    )?;
    Ok(found)
}

fn human_decision(
    context: &RuntimeGovernanceResultContext<'_>,
    facts: &ProvenanceRecord,
    binding: &HumanDecisionBinding,
    captured: &CapturedGovernanceEvidence,
) -> Result<DecisionAudit, RuntimeError> {
    require(
        binding.transition.event == context.event.event,
        "human decision must bind its original accepted event",
    )?;
    let task = context
        .event
        .data
        .as_ref()
        .and_then(|data| data.get("taskId"))
        .and_then(Value::as_str)
        .ok_or_else(|| refused("decision task identity missing"))?;
    require(
        crate::task_process_id(task).ok().as_deref()
            == Some(context.prior.process.process_id.as_str()),
        "decision task belongs to another process",
    )?;
    let accepted: Vec<_> = context
        .prior
        .artifacts
        .iter()
        .filter(|(_, artifact)| {
            artifact.kind == TaskArtifactKind::Accepted && artifact.task_id == task
        })
        .collect();
    require(
        accepted.len() == 1,
        "accepted decision artifact missing or ambiguous",
    )?;
    let (artifact_id, artifact) = accepted[0];
    require(
        artifact_id == &artifact.artifact_id
            && context.event.actor_id.as_deref() == Some(artifact.actor_id.as_str())
            && context.event.timestamp == artifact.recorded_at,
        "accepted decision attribution differs",
    )?;
    let creation: Vec<_> = context
        .prior
        .provenance_log
        .iter()
        .filter(|record| {
            record.record_kind == ProvenanceKind::TaskCreated
                && record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("taskId"))
                    .and_then(Value::as_str)
                    == Some(task)
        })
        .collect();
    require(
        creation.len() == 1
            && creation[0]
                .data
                .as_ref()
                .and_then(|data| data.pointer("/details/taskRef"))
                .and_then(Value::as_str)
                == Some(binding.task_ref.as_str()),
        "decision task declaration differs",
    )?;
    let completion: Vec<_> = context
        .prior
        .provenance_log
        .iter()
        .filter(|record| {
            record.record_kind == ProvenanceKind::TaskCompleted
                && record
                    .data
                    .as_ref()
                    .and_then(|data| data.get("taskId"))
                    .and_then(Value::as_str)
                    == Some(task)
        })
        .collect();
    require(
        completion.len() == 1
            && completion[0].actor_id.as_deref() == Some(artifact.actor_id.as_str())
            && completion[0].timestamp == artifact.recorded_at
            && completion[0]
                .data
                .as_ref()
                .and_then(|data| data.pointer("/details/artifactId"))
                .and_then(Value::as_str)
                == Some(artifact.artifact_id.as_str()),
        "accepted decision completion differs",
    )?;
    let human = context
        .prior
        .process
        .actor_bindings
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter(|assigned| assigned.assigned_actor == artifact.actor_id)
        .collect::<Vec<_>>();
    require(human.len() == 1, "decision actor is not one resolved human")?;
    let normalized;
    let input = if let Some(mapping) = &binding.evidence_mapping {
        let response_sha256 = stack_common_hash::canonical_json_hash(&artifact.response)
            .map_err(|_| refused("original decision Response cannot canonicalize"))?;
        let matches = captured
            .decisions
            .iter()
            .filter(|decision| decision.artifact_id == artifact.artifact_id)
            .collect::<Vec<_>>();
        require(
            matches.len() == 1,
            "mapped decision evidence missing or ambiguous",
        )?;
        let decision = matches[0];
        require(
            decision.response_sha256 == response_sha256
                && decision.mapping_ref == mapping.reference
                && decision.mapping_sha256 == mapping.sha256,
            "mapped decision original Response or selected Mapping differs",
        )?;
        normalized = decision.evidence.clone();
        &normalized
    } else {
        artifact
            .response
            .pointer(&binding.evidence_pointer)
            .ok_or_else(|| refused("structured decision evidence missing"))?
    };
    let evidence = DecisionEvidence::from_json(input)
        .map_err(|_| refused("structured decision evidence invalid"))?;
    validate_evidence(&evidence, binding, facts, context.prior)?;
    let snapshot = facts
        .case_file_snapshot
        .as_ref()
        .expect("required_facts checked snapshot");
    let reasoning = ReasoningEvidence {
        facts_record_ref: facts.id.clone(),
        snapshot_sha256: snapshot.sha256.clone(),
        reason: evidence.reason,
        rules_applied: evidence.rules_applied,
        evidence_consulted: evidence.evidence_consulted,
        criteria_checked: evidence.criteria_checked,
        source_ref: artifact.artifact_id.clone(),
        adverse_notice: None,
    };
    let counterfactual = facts
        .transition_tags
        .iter()
        .any(|tag| tag == "adverse-decision")
        .then(|| CounterfactualEvidence {
            facts_record_ref: facts.id.clone(),
            snapshot_sha256: snapshot.sha256.clone(),
            positive_counterfactuals: evidence.positive_counterfactuals,
            negative_counterfactuals: evidence.negative_counterfactuals,
            source_ref: artifact.artifact_id.clone(),
        });
    Ok(DecisionAudit {
        actor: Some(artifact.actor_id.clone()),
        event: context.event.event.clone(),
        reasoning,
        counterfactual,
    })
}

fn validate_evidence(
    evidence: &DecisionEvidence,
    binding: &HumanDecisionBinding,
    facts: &ProvenanceRecord,
    prior: &RuntimeRecord,
) -> Result<(), RuntimeError> {
    require(
        !evidence.reason.trim().is_empty() && !evidence.evidence_consulted.is_empty(),
        "individualized reason or consulted evidence missing",
    )?;
    let permitted: BTreeMap<_, _> = binding
        .rules
        .iter()
        .map(|rule| (&rule.rule_id, rule))
        .collect();
    let actual: Vec<_> = evidence
        .rules_applied
        .iter()
        .map(|rule| rule.rule_id.clone())
        .collect();
    require_unique_nonblank(&actual)?;
    require(
        !actual.is_empty()
            && evidence.rules_applied.iter().all(|rule| {
                permitted.get(&rule.rule_id).is_some_and(|selected| {
                    serde_json::to_value(selected).ok() == serde_json::to_value(rule).ok()
                })
            }),
        "applied rule differs from selected source",
    )?;
    let mut criteria: Vec<_> = evidence
        .criteria_checked
        .iter()
        .map(|result| result.criterion.clone())
        .collect();
    require_unique_nonblank(&criteria)?;
    criteria.sort();
    let mut expected = binding.criteria.clone();
    expected.sort();
    require(criteria == expected, "criterion coverage differs")?;
    let snapshot = &facts
        .case_file_snapshot
        .as_ref()
        .expect("snapshot checked")
        .value;
    for reference in &evidence.evidence_consulted {
        let has_path = reference.case_field_path.as_ref().is_some_and(|path| {
            reference.kind == "caseField"
                && binding.case_field_paths.contains(path)
                && snapshot.pointer(path).is_some()
        });
        let has_artifact = reference.uri.as_ref().is_some_and(|uri| {
            matches!(reference.kind.as_str(), "document" | "externalRecord")
                && prior.artifacts.iter().any(|(id, artifact)| {
                    id == &artifact.artifact_id
                        && artifact.kind == TaskArtifactKind::Accepted
                        && uri == &format!("urn:wos:artifact:{}", artifact.artifact_id)
                })
        });
        require(
            (has_path && reference.uri.is_none())
                || (has_artifact && reference.case_field_path.is_none()),
            "consulted evidence is absent, mixed or unselected",
        )?;
    }
    let adverse = facts
        .transition_tags
        .iter()
        .any(|tag| tag == "adverse-decision");
    if adverse {
        require(
            evidence
                .criteria_checked
                .iter()
                .any(|result| !result.satisfied),
            "adverse decision has no failed criterion",
        )?;
        require(
            !evidence.positive_counterfactuals.is_empty()
                && !evidence.negative_counterfactuals.is_empty(),
            "required counterfactuals missing",
        )?;
    }
    for (records, polarity, allowed) in [
        (
            &evidence.positive_counterfactuals,
            CounterfactualPolarity::Positive,
            &binding.positive_factors,
        ),
        (
            &evidence.negative_counterfactuals,
            CounterfactualPolarity::Negative,
            &binding.negative_factors,
        ),
    ] {
        let factors: Vec<_> = records.iter().map(|record| record.factor.clone()).collect();
        require_unique_nonblank(&factors)?;
        require(
            records.iter().all(|record| {
                record.polarity == polarity
                    && allowed.contains(&record.factor)
                    && !record.effect.trim().is_empty()
            }),
            "counterfactual polarity or selected factor differs",
        )?;
    }
    if adverse {
        require(
            evidence.positive_counterfactuals.iter().all(|record| {
                evidence
                    .criteria_checked
                    .iter()
                    .any(|criterion| criterion.criterion == record.factor && !criterion.satisfied)
            }),
            "positive counterfactual does not bind a failed criterion",
        )?;
        let actual: BTreeSet<_> = evidence
            .negative_counterfactuals
            .iter()
            .map(|record| &record.factor)
            .collect();
        require(
            actual == binding.negative_factors.iter().collect(),
            "excluded factor coverage differs",
        )?;
    }
    Ok(())
}

fn transition_matches_observed(
    selected: &GovernanceTransition,
    actual: &ObservedTransition,
) -> bool {
    selected.from == actual.from && selected.to == actual.to && selected.event == actual.event
}
fn transition_matches_facts(selected: &GovernanceTransition, facts: &ProvenanceRecord) -> bool {
    facts.from_state.as_deref() == Some(selected.from.as_str())
        && facts.to_state.as_deref() == Some(selected.to.as_str())
        && facts
            .data
            .as_ref()
            .and_then(|data| data["transitionEvent"].as_str())
            == Some(selected.event.as_str())
}
fn accepted_event_artifact<'a>(
    context: &RuntimeGovernanceResultContext<'a>,
    task_ref: &str,
) -> Result<&'a crate::store::TaskArtifact, RuntimeError> {
    let task = context
        .event
        .data
        .as_ref()
        .and_then(|data| data["taskId"].as_str())
        .ok_or_else(|| refused("accepted task event missing"))?;
    require(
        crate::task_process_id(task).ok().as_deref()
            == Some(context.prior.process.process_id.as_str()),
        "accepted task belongs to another process",
    )?;
    let artifacts = context
        .prior
        .artifacts
        .iter()
        .filter(|(_, artifact)| {
            artifact.kind == TaskArtifactKind::Accepted && artifact.task_id == task
        })
        .collect::<Vec<_>>();
    require(
        artifacts.len() == 1,
        "accepted task artifact missing or ambiguous",
    )?;
    let (id, artifact) = artifacts[0];
    require(
        id == &artifact.artifact_id
            && context.event.actor_id.as_deref() == Some(artifact.actor_id.as_str())
            && context.event.timestamp == artifact.recorded_at,
        "accepted task event attribution differs",
    )?;
    let created = context
        .prior
        .provenance_log
        .iter()
        .filter(|record| {
            record.record_kind == ProvenanceKind::TaskCreated
                && record.data.as_ref().is_some_and(|data| {
                    data["taskId"] == task && data["details"]["taskRef"] == task_ref
                })
        })
        .collect::<Vec<_>>();
    let completed = context
        .prior
        .provenance_log
        .iter()
        .filter(|record| {
            record.record_kind == ProvenanceKind::TaskCompleted
                && record.data.as_ref().is_some_and(|data| {
                    data["taskId"] == task && data["details"]["artifactId"] == artifact.artifact_id
                })
                && record.actor_id.as_deref() == Some(artifact.actor_id.as_str())
                && record.timestamp == artifact.recorded_at
        })
        .collect::<Vec<_>>();
    require(
        created.len() == 1 && completed.len() == 1,
        "accepted task has no unique creation/completion witness",
    )?;
    Ok(artifact)
}
fn retained_proposal<'a>(
    prior: &'a RuntimeRecord,
    id: &str,
) -> Result<(&'a ProvenanceRecord, AdverseNoticeProposal), RuntimeError> {
    let facts = prior
        .provenance_log
        .iter()
        .filter(|record| {
            record.id == id
                && record.record_kind == ProvenanceKind::StateTransition
                && record
                    .transition_tags
                    .iter()
                    .any(|tag| tag == "determination")
                && record
                    .transition_tags
                    .iter()
                    .any(|tag| tag == "adverse-decision")
        })
        .collect::<Vec<_>>();
    require(
        facts.len() == 1,
        "original adverse proposal Facts missing or ambiguous",
    )?;
    let reasons = prior
        .provenance_log
        .iter()
        .filter_map(|record| {
            if record.record_kind != ProvenanceKind::ReasoningTierRecorded {
                return None;
            }
            let reason: ReasoningEvidence = serde_json::from_value(record.data.clone()?).ok()?;
            (reason.facts_record_ref == id).then_some(reason)
        })
        .collect::<Vec<_>>();
    require(
        reasons.len() == 1,
        "original proposal Reasoning missing or ambiguous",
    )?;
    let proposal = reasons[0]
        .adverse_notice
        .clone()
        .ok_or_else(|| refused("original notice proposal missing"))?;
    require(
        facts[0]
            .case_file_snapshot
            .as_ref()
            .is_some_and(|snapshot| {
                snapshot.sha256 == proposal.snapshot_sha256
                    && wos_events::CaseFileSnapshot::from_case_state(&snapshot.value) == *snapshot
            })
            && proposal.facts_record_ref == id
            && stack_common_hash::canonical_json_hash(&serde_json::json!({
            "machineReadable":proposal.machine_readable,"humanReadable":proposal.human_readable}))
            .ok()
            .as_ref()
                == Some(&proposal.content_sha256),
        "original proposal content or snapshot differs",
    )?;
    Ok((facts[0], proposal))
}
fn governance_time(ms: u64) -> Result<String, RuntimeError> {
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .map_err(|_| refused("governance deadline out of range"))?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| refused("governance deadline cannot format"))
}

fn timeout_decision(
    context: &RuntimeGovernanceResultContext<'_>,
    facts: &ProvenanceRecord,
    binding: &TimeoutDecisionBinding,
    evidence: &CapturedGovernanceEvidence,
) -> Result<DecisionAudit, RuntimeError> {
    require(
        context.event.actor_id.is_none()
            && context.event.event == binding.transition.event
            && context
                .event
                .data
                .as_ref()
                .is_some_and(|data| data["timerId"] == binding.timer_id),
        "timeout did not consume its actual native occurrence",
    )?;
    let timers = context
        .prior
        .process
        .timers
        .iter()
        .filter(|timer| timer.timer_id == binding.timer_id && timer.event == context.event.event)
        .collect::<Vec<_>>();
    require(
        timers.len() == 1,
        "original no-response timer missing or ambiguous",
    )?;
    let timer = timers[0];
    let period_ms = calendar_period_ms(&binding.response_period)?;
    let due = governance_timestamp(&timer.deadline)?;
    let start = timer
        .created_at_ms
        .ok_or_else(|| refused("original timer origin missing"))?;
    require(
        timer.duration_iso.as_deref() == Some(binding.response_period.as_str())
            && timer.duration_ms == Some(period_ms)
            && due
                == start
                    .checked_add(period_ms)
                    .ok_or_else(|| refused("no-response deadline overflow"))?
            && context.now_ms >= due,
        "original selected response timer has not expired or differs",
    )?;
    let fired = context
        .provenance
        .iter()
        .filter(|record| {
            record.record_kind == ProvenanceKind::TimerFired
                && record
                    .data
                    .as_ref()
                    .is_some_and(|data| data["timerId"] == binding.timer_id)
        })
        .collect::<Vec<_>>();
    require(
        fired.len() == 1,
        "actual native TimerFired witness missing or ambiguous",
    )?;
    let receipts = evidence
        .notices
        .iter()
        .filter(|receipt| receipt.binding_id == binding.request_notice_binding_id)
        .collect::<Vec<_>>();
    require(
        receipts.len() == 1 && validate_captured_notice(context, receipts[0])? == start,
        "timer is not anchored to its qualified original request notice",
    )?;
    let tasks = context
        .prior
        .provenance_log
        .iter()
        .filter_map(|record| {
            if record.record_kind != ProvenanceKind::TaskCreated {
                return None;
            }
            let data = record.data.as_ref()?;
            let task_ref = data["details"]["taskRef"].as_str()?;
            binding
                .correction_task_refs
                .iter()
                .any(|selected| selected == task_ref)
                .then(|| data["taskId"].as_str())
                .flatten()
        })
        .collect::<BTreeSet<_>>();
    require(
        !tasks.is_empty(),
        "no-response rule has no retained correction task",
    )?;
    for artifact in context.prior.artifacts.values().filter(|artifact| {
        artifact.kind == TaskArtifactKind::Accepted && tasks.contains(artifact.task_id.as_str())
    }) {
        let accepted = governance_timestamp(&artifact.recorded_at)?;
        require(
            accepted < start || accepted > due,
            "a timely correction was actually accepted",
        )?;
    }
    let snapshot = facts
        .case_file_snapshot
        .as_ref()
        .ok_or_else(|| refused("timeout Facts snapshot missing"))?;
    let source_ref = format!("urn:wos:timer:{}:{}", binding.timer_id, due);
    let reasoning = ReasoningEvidence {
        facts_record_ref: facts.id.clone(),
        snapshot_sha256: snapshot.sha256.clone(),
        reason: format!(
            "No requested correction was accepted during the {}-calendar-day period from {} through {}. This conclusion uses the complete retained accepted-artifact record.",
            period_ms / 86_400_000,
            governance_time(start)?,
            timer.deadline
        ),
        rules_applied: vec![binding.rule.clone()],
        evidence_consulted: vec![
            wos_events::governance::EvidenceReference {
                kind: "fact".into(),
                case_field_path: None,
                uri: Some(format!(
                    "urn:wos:provenance:{}",
                    receipts[0].callback_record_ref
                )),
                summary: Some("Actual qualified original request delivery".into()),
            },
            wos_events::governance::EvidenceReference {
                kind: "fact".into(),
                case_field_path: None,
                uri: Some(format!("urn:wos:provenance:{}", fired[0].id)),
                summary: Some("Actual original selected-period timer occurrence".into()),
            },
        ],
        criteria_checked: vec![wos_events::governance::CriterionResult {
            criterion: "timelyCorrectionAccepted".into(),
            satisfied: false,
            detail: Some(format!(
                "No accepted artifact for the selected correction tasks by {}",
                timer.deadline
            )),
        }],
        source_ref: source_ref.clone(),
        adverse_notice: None,
    };
    let counterfactual=CounterfactualEvidence {facts_record_ref:facts.id.clone(),snapshot_sha256:snapshot.sha256.clone(),
        positive_counterfactuals:vec![wos_events::governance::Counterfactual {polarity:CounterfactualPolarity::Positive,
            factor:"timelyCorrectionAccepted".into(),effect:format!("A correction accepted by {} would prevent this no-response outcome. Later evidence can use {} and still requires authorized human reconsideration.",timer.deadline,binding.reconsideration_route)}],
        negative_counterfactuals:binding.excluded_factors.iter().map(|factor|wos_events::governance::Counterfactual {
            polarity:CounterfactualPolarity::Negative,factor:factor.clone(),effect:format!("This no-response rule reads only the qualified original notice time, original timer and complete accepted correction records; it does not use {factor}.")}).collect(),source_ref};
    Ok(DecisionAudit {
        actor: None,
        event: context.event.event.clone(),
        reasoning,
        counterfactual: Some(counterfactual),
    })
}

fn qualify_due_process(
    selection: &GovernancePolicySelection,
    kernel: &KernelDocument,
) -> Result<(), RuntimeError> {
    let Some(due) = &selection.due_process else {
        return Ok(());
    };
    let policy = selection
        .kernel
        .pointer("/governance/dueProcess/adverseDecisionPolicy")
        .ok_or_else(|| refused("canonical adverse-decision policy missing"))?;
    require(
        selection
            .kernel
            .pointer("/governance/dueProcess/scope")
            .is_none(),
        "conditional due-process capability unsupported",
    )?;
    require(
        policy["noticeRequired"] == true
            && policy["noticeTiming"] == "beforeEffective"
            && policy["explanationLevel"] == "individualized"
            && policy["counterfactualRequired"] == true,
        "selected notice/explanation obligations are not supported",
    )?;
    if let Some(period) = policy["noticeGracePeriod"].as_str() {
        wos_core::eval::parse_iso_duration_to_ms(period)
            .map_err(|_| refused("notice grace period invalid"))?;
    }
    static DELIVERY: std::sync::OnceLock<Result<jsonschema::Validator, String>> =
        std::sync::OnceLock::new();
    let validator = DELIVERY
        .get_or_init(|| {
            let schema: Value = serde_json::from_str(include_str!(
                "../../../schemas/sidecars/wos-delivery.schema.json"
            ))
            .map_err(|_| "owning Delivery schema invalid".to_owned())?;
            jsonschema::options()
                .should_validate_formats(true)
                .build(&schema)
                .map_err(|_| "owning Delivery schema invalid".to_owned())
        })
        .as_ref()
        .map_err(|_| refused("owning Delivery schema unavailable"))?;
    require(
        validator.is_valid(&selection.delivery),
        "selected Delivery violates owning schema",
    )?;
    let key = policy["noticeTemplateKey"]
        .as_str()
        .ok_or_else(|| refused("explicit adverse template key missing"))?;
    let template = selection
        .delivery
        .pointer("/notifications/templates")
        .and_then(|all| all.get(key))
        .ok_or_else(|| refused("canonical adverse template key is unknown"))?;
    require(
        template["category"] == "adverse-decision",
        "selected notice category differs",
    )?;
    let sections = template["sections"]
        .as_array()
        .ok_or_else(|| refused("adverse notice sections missing"))?;
    for required in [
        "determination",
        "reasons",
        "appealRights",
        "appealInstructions",
    ] {
        require(
            sections
                .iter()
                .filter(|section| section["id"] == required)
                .count()
                == 1,
            "required selected notice section missing or ambiguous",
        )?;
    }
    require(
        !due.notice_binding_id.trim().is_empty()
            && !due.effective_transitions.is_empty()
            && !due.protected_effects.is_empty(),
        "before-effective binding incomplete",
    )?;
    let mut paths = BTreeSet::new();
    for field in &due.protected_effects {
        protected_pointer_segments(&field.path)?;
        require(
            field.path.starts_with('/')
                && paths.insert(&field.path)
                && !field.adverse_values.is_empty()
                && field.adverse_values.iter().all(|value| !value.is_null()),
            "protected business field selection invalid",
        )?;
    }
    let mut transitions = BTreeSet::new();
    for transition in &due.effective_transitions {
        require(
            transitions.insert(
                serde_json::to_string(transition)
                    .map_err(|_| refused("effective transition invalid"))?,
            ) && find_selected_transition(kernel, transition).is_some(),
            "effective transition missing or duplicated",
        )?;
    }
    let appeal = selection
        .appeal
        .as_ref()
        .ok_or_else(|| refused("independent appeal capability missing"))?;
    let promise = &policy["appealMechanism"];
    require(
        promise["enabled"] == true
            && promise["reviewerConstraint"] == "independentFromOriginal"
            && promise["continuationOfServices"] == false
            && promise["reviewerRoles"] == serde_json::json!([appeal.reviewer_actor]),
        "selected appeal obligations unsupported",
    )?;
    require(
        !appeal.policy_label.trim().is_empty()
            && !appeal.review_body.trim().is_empty()
            && !appeal.filing_instructions.trim().is_empty()
            && !appeal.filed_task_ref.trim().is_empty()
            && appeal.proposal_reference_pointer.starts_with('/')
            && !appeal.review_task_ref.trim().is_empty()
            && !appeal.review_transitions.is_empty()
            && find_selected_transition(kernel, &appeal.filed_transition).is_some(),
        "actual appeal binding incomplete",
    )?;
    calendar_period_ms(appeal_period(selection)?)?;
    let actor = kernel
        .actors
        .iter()
        .filter(|actor| actor.id == appeal.reviewer_actor)
        .collect::<Vec<_>>();
    require(
        actor.len() == 1 && actor[0].kind == wos_core::model::kernel::ActorKind::Human,
        "appeal reviewer is not a declared human",
    )?;
    for review in &appeal.review_transitions {
        require(
            selection
                .human_decisions
                .iter()
                .filter(|human| {
                    human.transition == *review && human.task_ref == appeal.review_task_ref
                })
                .count()
                == 1,
            "appeal review lacks exact human determination binding",
        )?;
    }
    for timeout in &selection.timeout_decisions {
        calendar_period_ms(&timeout.response_period)?;
        let transition = find_selected_transition(kernel, &timeout.transition)
            .ok_or_else(|| refused("timeout transition missing"))?;
        require(
            transition.guard.is_none()
                && !timeout.timer_id.trim().is_empty()
                && transition
                    .event
                    .as_ref()
                    .and_then(|event| serde_json::to_value(event).ok())
                    .is_some_and(|event| {
                        event["kind"] == "timer" && event["timerId"] == timeout.timer_id
                    }),
            "finite timeout rule must bind an unconditional actual native timer",
        )?;
        require(
            !timeout.request_notice_binding_id.trim().is_empty()
                && !timeout.correction_task_refs.is_empty()
                && !timeout.rule.rule_id.trim().is_empty()
                && !timeout.reconsideration_route.trim().is_empty(),
            "timeout rule source incomplete",
        )?;
        require_unique_nonblank(&timeout.correction_task_refs)?;
        require(
            !timeout.excluded_factors.is_empty(),
            "timeout excluded-factor declaration missing",
        )?;
        require_unique_nonblank(&timeout.excluded_factors)?;
    }
    Ok(())
}

fn find_selected_transition<'a>(
    kernel: &'a KernelDocument,
    selected: &GovernanceTransition,
) -> Option<&'a wos_core::model::kernel::Transition> {
    fn state<'a>(
        id: &str,
        current: &'a wos_core::model::kernel::State,
        selected: &GovernanceTransition,
    ) -> Option<&'a wos_core::model::kernel::Transition> {
        if id == selected.from {
            let mut found = current.transitions.iter().filter(|transition| {
                transition.target == selected.to
                    && transition
                        .event
                        .as_ref()
                        .is_some_and(|event| event.authoring_display_label() == selected.event)
            });
            let first = found.next()?;
            return found.next().is_none().then_some(first);
        }
        for (id, child) in &current.states {
            if let Some(found) = state(id, child, selected) {
                return Some(found);
            }
        }
        for region in current.regions.values() {
            for (id, child) in &region.states {
                if let Some(found) = state(id, child, selected) {
                    return Some(found);
                }
            }
        }
        None
    }
    kernel
        .lifecycle
        .states
        .iter()
        .find_map(|(id, current)| state(id, current, selected))
}

fn appeal_period(selection: &GovernancePolicySelection) -> Result<&str, RuntimeError> {
    selection
        .kernel
        .pointer("/governance/dueProcess/adverseDecisionPolicy/appealMechanism/appealWindow")
        .and_then(Value::as_str)
        .ok_or_else(|| refused("selected appeal period missing"))
}

fn calendar_period_ms(period: &str) -> Result<u64, RuntimeError> {
    let days = period
        .strip_prefix('P')
        .and_then(|value| value.strip_suffix('D'))
        .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or_else(|| refused("governance period must select whole calendar days"))?;
    require(
        days.parse::<u64>()
            .is_ok_and(|days| days > 0 && days.checked_mul(86_400_000).is_some()),
        "governance period must be positive",
    )?;
    wos_core::eval::parse_iso_duration_to_ms(period)
        .map_err(|_| refused("governance period invalid"))
}

fn governance_timestamp(raw: &str) -> Result<u64, RuntimeError> {
    let time = time::OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339)
        .map_err(|_| refused("qualified governance time invalid"))?;
    u64::try_from(time.unix_timestamp_nanos() / 1_000_000)
        .map_err(|_| refused("qualified governance time out of range"))
}

fn validate_captured_notice(
    context: &RuntimeGovernanceResultContext<'_>,
    receipt: &CapturedGovernanceNotice,
) -> Result<u64, RuntimeError> {
    require(
        receipt.process_id == context.prior.process.process_id
            && receipt.case_ledger_id == context.prior.process.case_ledger_id,
        "qualified notice belongs to another process or case",
    )?;
    let found = context
        .prior
        .provenance_log
        .iter()
        .filter(|record| record.id == receipt.callback_record_ref)
        .collect::<Vec<_>>();
    require(
        found.len() == 1 && found[0].record_kind == ProvenanceKind::CallbackReceived,
        "actual retained notice callback missing or ambiguous",
    )?;
    let data = found[0]
        .data
        .as_ref()
        .ok_or_else(|| refused("actual callback data missing"))?;
    require(
        stack_common_hash::canonical_json_hash(data).ok().as_ref()
            == Some(&receipt.callback_data_sha256)
            && data["bindingId"] == receipt.binding_id
            && data["data"]["delivery"]["intentId"] == receipt.intent_id
            && data["data"]["delivery"]["attemptId"] == receipt.attempt_id
            && data["data"]["clock"]["governedSentAt"] == receipt.governed_sent_at
            && data["data"]["clock"]["dateDecisionSha256"] == receipt.date_decision_sha256,
        "qualified original callback association differs",
    )?;
    let sent = governance_timestamp(&receipt.governed_sent_at)?;
    require(
        sent <= context.now_ms,
        "qualified original delivery lies after the command",
    )?;
    Ok(sent)
}

fn render_governance_text(
    source: &str,
    variables: &BTreeMap<String, String>,
) -> Result<String, RuntimeError> {
    let mut rest = source;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or_else(|| refused("notice placeholder is unterminated"))?;
        require(
            variables.contains_key(after[..end].trim()),
            "notice placeholder lacks an actual selected value",
        )?;
        rest = &after[end + 2..];
    }
    let mut rendered = String::new();
    let mut rest = source;
    while let Some(start) = rest.find("{{") {
        rendered.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or_else(|| refused("notice placeholder is unterminated"))?;
        rendered.push_str(
            variables
                .get(after[..end].trim())
                .ok_or_else(|| refused("notice placeholder lacks an actual selected value"))?,
        );
        rest = &after[end + 2..];
    }
    rendered.push_str(rest);
    Ok(rendered)
}

fn require_unique_nonblank(values: &[String]) -> Result<(), RuntimeError> {
    let seen: BTreeSet<_> = values.iter().collect();
    require(
        seen.len() == values.len() && values.iter().all(|value| !value.trim().is_empty()),
        "duplicate or blank evidence identifier",
    )
}
fn require(valid: bool, code: &'static str) -> Result<(), RuntimeError> {
    if valid { Ok(()) } else { Err(refused(code)) }
}
fn refused(code: &'static str) -> RuntimeError {
    RuntimeError::GovernancePolicy(code.into())
}

fn collect_required_transitions(
    id: &str,
    state: &wos_core::model::kernel::State,
    selected: &mut Vec<GovernanceTransition>,
    ids: &mut BTreeSet<String>,
) -> Result<(), RuntimeError> {
    require(
        ids.insert(id.to_owned()),
        "duplicate global state identifier",
    )?;
    require(
        state.body.is_none(),
        "dynamic governance topology unsupported",
    )?;
    for transition in &state.transitions {
        if transition
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "determination" | "adverse-decision"))
        {
            require(
                transition.tags.iter().any(|tag| tag == "determination"),
                "adverse transition requires determination Facts",
            )?;
            let event = transition
                .event
                .as_ref()
                .ok_or_else(|| refused("implicit governance transition unsupported"))?
                .authoring_display_label();
            selected.push(GovernanceTransition {
                from: id.into(),
                to: transition.target.clone(),
                event,
            });
        }
    }
    for (id, state) in &state.states {
        collect_required_transitions(id, state, selected, ids)?;
    }
    for region in state.regions.values() {
        for (id, state) in &region.states {
            collect_required_transitions(id, state, selected, ids)?;
        }
    }
    Ok(())
}

pub(crate) fn protected_pointer_segments(pointer: &str) -> Result<Vec<String>, RuntimeError> {
    if !pointer.starts_with('/') {
        return Err(refused("invalid governed protected pointer"));
    }
    pointer[1..]
        .split('/')
        .map(|segment| {
            let mut decoded = String::new();
            let mut chars = segment.chars();
            while let Some(c) = chars.next() {
                if c != '~' {
                    decoded.push(c);
                } else {
                    decoded.push(match chars.next() {
                        Some('0') => '~',
                        Some('1') => '/',
                        _ => return Err(refused("invalid governed protected pointer escape")),
                    });
                }
            }
            Ok(decoded)
        })
        .collect()
}
