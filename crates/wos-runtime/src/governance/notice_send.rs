// Rust guideline compliant 2026-02-21
//! Checks original send facts without granting provider or recipient authority.
use super::*;
use crate::runtime::DeferredOutboundEvent;
use stack_common_notify::{DeliveryOutcome, DeliveryRequest, SmtpCapability};
use wos_core::model::notice_requirements::{NoticeAnchor, NoticeObservation, NoticeScope};

/// Raw original records checked against the native prior and selected policy.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedNoticeSendInput {
    /// Exact original deferred native request.
    pub outbound: DeferredOutboundEvent,
    /// Original complete recipient and rendered message.
    pub request: DeliveryRequest,
    /// Complete actual retained transport result, including unsuccessful results.
    pub outcome: DeliveryOutcome,
    /// Owning host's original recipient authority locator; not self-authenticating.
    pub recipient_authority_ref: String,
    /// Original retained native callback record.
    pub callback_record_ref: String,
    /// Original date owner's complete decision; never an asserted passing flag.
    pub clock_decision: Option<Value>,
}

impl std::fmt::Debug for CapturedNoticeSendInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CapturedNoticeSendInput { protected: [REDACTED] }")
    }
}

impl QualifiedGovernancePolicy {
    /// Checks original message, result, native associations and selected notice meaning.
    ///
    /// The host authenticates durable origin, current recipient authority and date
    /// arithmetic. Native checks correlate those original inputs; they cannot prove
    /// that an SMTP server actually sent a caller-supplied result.
    ///
    /// # Errors
    /// Refuses unsuccessful sends, changed policy/records or missing governed clocks.
    pub fn capture_notice_send(
        &self,
        prior: &RuntimeRecord,
        input: CapturedNoticeSendInput,
    ) -> Result<CapturedGovernanceNotice, RuntimeError> {
        input
            .outcome
            .validate_for(&input.request)
            .map_err(|_| refused("original notice transport result differs"))?;
        let DeliveryOutcome::Accepted {
            observed_at: Some(physical),
            capability,
            ..
        } = &input.outcome
        else {
            return Err(refused(
                "notice requires actual accepted transport observation",
            ));
        };
        let outbound = &input.outbound.event;
        let requirement = self
            .selection
            .rules
            .notice_requirements
            .get(&outbound.binding_id)
            .ok_or_else(|| refused("original notice requirement missing"))?;
        let scope = match capability {
            SmtpCapability::Smtp => NoticeScope::Production,
            SmtpCapability::SmtpTestCapture => NoticeScope::SoftwareFixture,
        };
        require(
            requirement.observation == NoticeObservation::TransportAccepted
                && requirement.scope == scope
                && requirement.anchor == NoticeAnchor::QualifiedObservation,
            "actual send does not establish the selected notice requirement",
        )?;
        require(
            input.outbound.process_id == prior.process.process_id
                && outbound.definition_url == prior.process.definition_url
                && outbound.definition_version == prior.process.definition_version
                && outbound.definition_url == self.kernel.url.clone().unwrap_or_default()
                && outbound.definition_version == self.kernel.version.clone().unwrap_or_default()
                && !input.recipient_authority_ref.trim().is_empty()
                && self
                    .selection
                    .integration
                    .as_ref()
                    .and_then(|p| p.bindings.get(&outbound.binding_id))
                    == Some(&outbound.binding_snapshot)
                && outbound.request["id"] == outbound.invocation_id
                && outbound.request["subject"] == outbound.subject,
            "original notice outbound or selected binding differs",
        )?;
        let raw_outbound =
            serde_json::to_value(outbound).map_err(|_| refused("outbound cannot serialize"))?;
        let pending: Vec<_> = prior
            .provenance_log
            .iter()
            .filter(|record| {
                record.record_kind == ProvenanceKind::CallbackPending
                    && record.data.as_ref().is_some_and(|data| {
                        data["bindingId"] == outbound.binding_id
                            && data["invocationId"] == outbound.invocation_id
                            && data["subject"] == outbound.subject
                    })
            })
            .collect();
        require(
            pending.len() == 1
                && pending[0]
                    .data
                    .as_ref()
                    .is_some_and(|v| v["deferredRequest"] == raw_outbound),
            "original retained deferred notice request missing or ambiguous",
        )?;
        let found: Vec<_> = prior
            .provenance_log
            .iter()
            .filter(|record| record.id == input.callback_record_ref)
            .collect();
        require(
            found.len() == 1 && found[0].record_kind == ProvenanceKind::CallbackReceived,
            "actual retained notice callback missing or ambiguous",
        )?;
        let callback = found[0]
            .data
            .as_ref()
            .ok_or_else(|| refused("actual callback data missing"))?;
        let outcome = serde_json::to_value(&input.outcome)
            .map_err(|_| refused("outcome cannot serialize"))?;
        require(
            callback["bindingId"] == outbound.binding_id
                && callback["invocationId"] == outbound.invocation_id
                && callback["correlationSubject"] == outbound.subject
                && callback["data"]["delivery"] == outcome
                && callback["data"]["sentAt"] == *physical
                && callback.get("time").is_none_or(|v| {
                    v.as_str().is_some_and(|v| {
                        chrono::DateTime::parse_from_rfc3339(v).ok()
                            == chrono::DateTime::parse_from_rfc3339(physical).ok()
                    })
                }),
            "original callback send association differs",
        )?;
        let duplicates = prior
            .provenance_log
            .iter()
            .filter(|record| {
                record.record_kind == ProvenanceKind::CallbackReceived
                    && record.data.as_ref().is_some_and(|data| {
                        data["bindingId"] == outbound.binding_id
                            && data["invocationId"] == outbound.invocation_id
                    })
            })
            .count();
        require(
            duplicates == 1,
            "original notice callback association ambiguous",
        )?;
        let decision = input
            .clock_decision
            .as_ref()
            .ok_or_else(|| refused("original notice clock decision missing"))?;
        let clock = &callback["data"]["clock"];
        let sent = clock["governedSentAt"]
            .as_str()
            .ok_or_else(|| refused("original governed notice time missing"))?;
        governance_timestamp(sent)?;
        let date = stack_common_hash::canonical_json_hash(decision)
            .map_err(|_| refused("clock cannot canonicalize"))?;
        let witness = &decision["evidence"]["times"]["witness"];
        let profile = witness
            .get("profile")
            .filter(|v| v.is_object())
            .ok_or_else(|| refused("original notice clock profile missing"))?;
        let profile_hash = stack_common_hash::canonical_json_hash(profile)
            .map_err(|_| refused("clock profile cannot canonicalize"))?;
        require(
            clock["originalProfileSha256"] == profile_hash
                && witness["profileSha256"] == profile_hash
                && clock["version"] == profile["version"]
                && profile["version"].as_u64().is_some(),
            "original notice clock profile differs",
        )?;
        require(
            decision["version"] == 1
                && decision["status"] == "qualified"
                && matches!(
                    decision["code"].as_str(),
                    Some("date_agreement" | "observation_qualified")
                )
                && decision["outcomeSha256"]
                    == stack_common_hash::canonical_json_hash(&outcome)
                        .map_err(|_| refused("outcome cannot canonicalize"))?
                && decision["evidence"]["times"]["witness"]["rawPhysicalSentAt"] == *physical
                && decision["evidence"]["times"]["governedSentAt"] == sent
                && clock["dateDecisionSha256"] == date,
            "original physical observation or qualified clock decision differs",
        )?;
        let mut proposals = Vec::new();
        for record in &prior.provenance_log {
            if record.record_kind != ProvenanceKind::ReasoningTierRecorded {
                continue;
            }
            let Some(proposal) = record
                .data
                .as_ref()
                .and_then(|v| serde_json::from_value::<ReasoningEvidence>(v.clone()).ok())
                .and_then(|r| r.adverse_notice)
            else {
                continue;
            };
            if proposal.human_readable != input.request.message.text_body {
                continue;
            }
            let (_, retained) = retained_proposal(prior, &proposal.facts_record_ref)?;
            require(
                retained.content_sha256 == proposal.content_sha256,
                "original notice proposal differs",
            )?;
            proposals.push(proposal.content_sha256);
        }
        let adverse = self
            .selection
            .rules
            .due_process
            .as_ref()
            .is_some_and(|binding| binding.notice_binding_id == outbound.binding_id);
        require(
            proposals.len() <= 1 && (!adverse || proposals.len() == 1),
            "original notice proposal missing or ambiguous",
        )?;
        Ok(CapturedGovernanceNotice {
            process_id: prior.process.process_id.clone(),
            case_ledger_id: prior.process.case_ledger_id.clone(),
            binding_id: outbound.binding_id.clone(),
            intent_id: input.request.intent_id.clone(),
            attempt_id: input.request.attempt_id.clone(),
            callback_record_ref: input.callback_record_ref.clone(),
            callback_data_sha256: stack_common_hash::canonical_json_hash(callback)
                .map_err(|_| refused("callback cannot canonicalize"))?,
            date_decision_sha256: date,
            governed_sent_at: sent.into(),
            human_readable: input.request.message.text_body.clone(),
            proposal_content_sha256: proposals.pop(),
            selected_policy_fingerprint: self.fingerprint.clone(),
            requirement: *requirement,
            send_input: input,
        })
    }
}
