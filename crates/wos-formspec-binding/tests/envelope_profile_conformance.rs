// Rust guideline compliant 2026-02-21
//! Content-Attestation Envelope conformance suite (`fs-6pb7` + `fs-o1lw`).
//!
//! Exercises the 7 named Content-Attestation Envelope profiles defined in
//! `work-spec/specs/profiles/signature-ports.md` §3.2 (`basic-esign`, `ron`,
//! `compliance-strict`, `in-person-notary`, `intranet`, `single-locale`,
//! `audit-maxed`) across:
//! 1. Native Formspec `Definition` + `Response` validation (`ImmutableFormspecProcessor`),
//! 2. Real Ed25519 COSE_Sign1 document-attestation verification (`FormspecBinding`),
//! 3. Real Trellis export bundle assembly (`trellis-core`, `trellis-export-profile-wos`,
//!    `trellis-export-writer`) and structural/domain verification (`trellis-verify-wos`),
//! 4. The consumer-facing `EnvelopeVerifierFacade` (`P-13`, `signature-ports.md` §3.6)
//!    deriving its 6 canonical verification answers from the verified export bundle
//!    (`010-events.cbor`, `062-signature-affirmations.cbor`, `066-signed-acts.cbor`,
//!    `068-signed-acts-manifest.cbor`) and [`WosVerificationReport`], and
//! 5. Negative controls proving tamper/omission refusal across all `K-1`..`K-7` and
//!    `P-1`..`P-14` posture dimensions, including the `act_correlation_conflict` finding
//!    when two distinct documents share one `signingActId` in an exported bundle.

use std::collections::BTreeMap;
use std::path::Path;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use integrity_canonical::{DigestAlgorithm, ResponseJson};
use integrity_cbor::json_to_dcbor_bytes;
use integrity_signature::{KeyRef, MethodRegistry};
use integrity_signature_ring::RingVerifier;
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::{Value as JsonValue, json};
use trellis_core::{AuthoredEvent, LedgerStore, SigningKeyMaterial as CoreSigningKey, append_event};
use trellis_export_profile_wos::{
    WosExportProfile, wos_signature_admission_failed_event_type, wos_signature_affirmation_event_type,
};
use trellis_export_writer::{
    ExportSealFence, ExportWriterInput, PostureDeclaration, RegistrySnapshot,
    SignedActsCatalogMember, SignedActsManifestMember, SigningKeyMaterial as ExportSigningKey,
    TrellisTimestamp, write_export,
};
use trellis_server_ports::ExportProfile;
use trellis_types::{
    CONTENT_DOMAIN, StoredCheckpoint, StoredEvent, Value as CborValue, decode_cbor_value,
    domain_separated_sha256, encode_canonical_cbor_value,
};
use trellis_verify_wos::{WosVerificationReport, encode_signed_acts_manifest_v1, verify_export_zip};
use wos_formspec_binding::selected::{
    ImmutableFormspecProcessor, SelectedDocument, SelectedDocumentKind,
};
use wos_formspec_binding::{DocumentSignatureVerification, FormspecBinding, FormspecProcessor};
use wos_runtime::{ContractBindingAdapter, SignatureEvidence, SignaturePrimitiveStatus};

const ALL_PROFILES: [&str; 7] = [
    "basic-esign",
    "ron",
    "compliance-strict",
    "in-person-notary",
    "intranet",
    "single-locale",
    "audit-maxed",
];

/// Canonical consumer-facing answer to Question 1 ("Who signed, in what role, and witnessing whom?").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedSignerAct {
    pub signing_act_id: String,
    pub signer_id: String,
    pub role_id: String,
    pub role: String,
    pub witnessed_signature_ref: Option<String>,
}

/// Canonical consumer-facing answer to Question 2 ("What exact documents and presentation hashes were signed?").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedSignedDocument {
    pub document_id: String,
    pub sha256: String,
    pub presentation_hash: String,
    pub content_sub_kind: String,
}

/// Canonical consumer-facing answer to Question 3 ("When did each signing act occur, and under what timer posture?").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedSigningTimestamp {
    pub signing_act_id: String,
    pub signed_at: String,
    pub timer_anchoring_mode: String,
}

/// Canonical consumer-facing answer to Question 4 ("What pre-signature view evidence was recorded?").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedViewEvidence {
    pub recipient_id: String,
    pub document_id: String,
    pub presentation_hash: String,
    pub opened_at: String,
    pub completed_scroll_at: Option<String>,
    pub scroll_completed: bool,
    pub dwell_ms: u64,
}

/// Canonical consumer-facing answer to Question 5 ("Is the envelope sealed or voided?").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvelopeVoidStatus {
    pub is_voided: bool,
    pub is_sealed: bool,
    pub reason: Option<String>,
}

/// Canonical consumer-facing answer to Question 6 ("What identity proofing tier and assurance level bound each recipient?").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedProofingLevel {
    pub recipient_id: String,
    pub tier: String,
    pub assurance_level: String,
    pub identity_binding_ref: String,
}

/// Refusal reasons surfaced by [`EnvelopeVerifierFacade`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvelopeVerificationError {
    /// Formspec native Definition or Response validation failed.
    FormspecValidation(String),
    /// Cryptographic or declared-document signature verification failed (`K-2`).
    SignatureVerification(String),
    /// A signed document digest or presentation hash disagreed with the declared envelope kernel (`K-1` / `P-12`).
    DocumentDigestMismatch(String),
    /// A document's `contentSubKind` is not permitted by `P-11_documentSourceResolver`.
    DisallowedContentSubKind(String),
    /// Required pre-signature `viewEvidence` (`P-2_viewEvidenceCollector`) is missing or incomplete.
    ViewEvidenceViolation(String),
    /// Identity proofing (`P-8_recipientIdentityProofing`) is missing or fails profile posture requirements.
    IdentityProofingViolation(String),
    /// Witness / notary reference or commission requirements (`K-5` / `K-6` / `P-8`) were not satisfied.
    WitnessNotaryViolation(String),
    /// Agent proposal (`P-6_intentRegistry`) lacked required human principal ratification.
    UnratifiedAgentProposal(String),
    /// Timer anchoring (`P-10_envelopeTimerPolicy`) requirements were not met.
    TimerAnchoringViolation(String),
    /// Envelope is voided (`K-3` / `P-3_voidingPolicy`) and cannot be accepted as a sealed completion (`K-7`).
    EnvelopeVoided(String),
    /// Profile uses `wos-server-only` (`P-1_documentBytesStore`) / `wos-resident-only` (`P-14_sealingOrchestration`) and cannot satisfy an offline bundle claim.
    ServerOnlyPostureNoOfflineBundle(String),
    /// Trellis export ZIP construction or verification (`verify_export_zip`) failed.
    TrellisBundleVerification(String),
}

impl std::fmt::Display for EnvelopeVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for EnvelopeVerificationError {}

/// Verified Formspec signing act and its verified `SignatureEvidence` outputs.
struct VerifiedFormspecAct {
    signing_act_id: String,
    role_id: String,
    role: String,
    consent_ref: String,
    witnessed_ref: Option<String>,
    ratified_agent_proposal_id: Option<String>,
    evidences: Vec<SignatureEvidence>,
}

/// Consumer-facing verification facade (`P-13_envelopeVerifierFacade`, `work-spec/specs/profiles/signature-ports.md` §3.6).
///
/// Wraps [`trellis_verify_wos::verify_export_zip`] and [`FormspecBinding`] document-attestation
/// verification to answer the six canonical relying-party questions directly from the verified
/// export bundle while preserving direct access to the underlying [`WosVerificationReport`] via
/// [`Self::structural_inspector`].
#[derive(Debug)]
pub struct EnvelopeVerifierFacade {
    profile: String,
    who_signed: Vec<VerifiedSignerAct>,
    what_was_signed: Vec<VerifiedSignedDocument>,
    when_signed: Vec<VerifiedSigningTimestamp>,
    view_evidence: Vec<VerifiedViewEvidence>,
    void_status: EnvelopeVoidStatus,
    proofing_levels: BTreeMap<String, VerifiedProofingLevel>,
    trellis_report: Option<WosVerificationReport>,
}

impl EnvelopeVerifierFacade {
    /// Verifies a Content-Attestation Envelope conformance fixture end-to-end.
    ///
    /// For all offline-bundle profiles (`P-1_documentBytesStore` in `two-tier-cache-then-cas` or
    /// `cas-from-intake`), this method builds the Trellis export ZIP and runs
    /// [`verify_export_zip`] **first**, then parses the verified `010-events.cbor`,
    /// `062-signature-affirmations.cbor`, `066-signed-acts.cbor`, and
    /// `068-signed-acts-manifest.cbor` members from the verified ZIP bytes to derive the
    /// six canonical answers.
    ///
    /// # Errors
    /// Returns [`EnvelopeVerificationError`] if Formspec validation, COSE_Sign1 verification,
    /// kernel/port posture checks, or Trellis export bundle verification fails.
    pub fn verify_envelope_fixture(
        fixture: &JsonValue,
    ) -> Result<Self, EnvelopeVerificationError> {
        let profile = fixture["profile"]
            .as_str()
            .ok_or_else(|| {
                EnvelopeVerificationError::FormspecValidation("missing profile name".into())
            })?
            .to_string();

        // Check P-1_documentBytesStore and P-14_sealingOrchestration bundle posture up front.
        let p1_store = &fixture["ports"]["P-1_documentBytesStore"];
        let p1_mode = p1_store["mode"].as_str().unwrap_or_default();
        let p14_mode = fixture["ports"]["P-14_sealingOrchestration"]["mode"]
            .as_str()
            .unwrap_or_default();
        let offline_verifiable = fixture["kernel"]["offlineVerifiable"]
            .as_bool()
            .unwrap_or(false);

        let is_offline_profile = match p1_mode {
            "two-tier-cache-then-cas" | "cas-from-intake" => {
                if !offline_verifiable || p14_mode == "wos-resident-only" {
                    return Err(
                        EnvelopeVerificationError::ServerOnlyPostureNoOfflineBundle(format!(
                            "inconsistent P-1_documentBytesStore mode `{p1_mode}`, P-14 `{p14_mode}`, and offlineVerifiable={offline_verifiable}"
                        )),
                    );
                }
                true
            }
            "wos-server-only" => {
                let disclosure = p1_store["offlineExportDisclosure"]
                    .as_str()
                    .unwrap_or_default();
                if offline_verifiable || disclosure != "server-retained-no-offline-bundle" {
                    return Err(
                        EnvelopeVerificationError::ServerOnlyPostureNoOfflineBundle(
                            "P-1_documentBytesStore wos-server-only posture requires offlineVerifiable=false and explicit offlineExportDisclosure".into(),
                        ),
                    );
                }
                false
            }
            other => {
                return Err(EnvelopeVerificationError::ServerOnlyPostureNoOfflineBundle(
                    format!("unsupported P-1_documentBytesStore mode `{other}`"),
                ));
            }
        };

        // Verify Formspec Definition + signed Responses through FormspecBinding.
        let verified_formspec_acts = verify_formspec_signing_acts(fixture)?;

        if is_offline_profile {
            // 1. Build the Trellis export ZIP and run verify_export_zip(&export_zip_bytes) FIRST.
            let export_zip_bytes =
                build_trellis_export_zip(fixture, &verified_formspec_acts)?;
            let report = verify_export_zip(&export_zip_bytes);
            if !report.trellis.structure_verified
                || !report.trellis.integrity_verified
                || !report.trellis.readability_verified
                || !report.wos_findings.is_empty()
            {
                return Err(EnvelopeVerificationError::TrellisBundleVerification(
                    format!("verify_export_zip failed: {report:#?}"),
                ));
            }

            // 2. Parse verified 010-events.cbor, 062-signature-affirmations.cbor,
            //    066-signed-acts.cbor, and 068-signed-acts-manifest.cbor from export_zip_bytes
            //    and derive all 6 canonical questions from the verified archive + Formspec bindings.
            Self::derive_from_verified_export_zip(
                &profile,
                fixture,
                &verified_formspec_acts,
                &export_zip_bytes,
                report,
            )
        } else {
            // wos-server-only / wos-resident-only posture (intranet): derive from verified
            // WOS kernel events + verified Formspec bindings without an offline ZIP bundle.
            let sealed_events = fixture["lifecycleEvents"]
                .as_array()
                .ok_or_else(|| {
                    EnvelopeVerificationError::FormspecValidation("missing lifecycleEvents".into())
                })?
                .clone();
            let affirmation_rows =
                synthesize_server_only_affirmation_rows(&verified_formspec_acts);
            Self::derive_from_verified_records(
                &profile,
                fixture,
                &verified_formspec_acts,
                &sealed_events,
                &affirmation_rows,
                None,
            )
        }
    }

    fn derive_from_verified_export_zip(
        profile: &str,
        fixture: &JsonValue,
        verified_formspec_acts: &[VerifiedFormspecAct],
        export_zip_bytes: &[u8],
        report: WosVerificationReport,
    ) -> Result<Self, EnvelopeVerificationError> {
        let members = extract_stored_zip_members(export_zip_bytes)
            .map_err(EnvelopeVerificationError::TrellisBundleVerification)?;

        // Parse 010-events.cbor from the verified ZIP archive.
        let events_bytes = members.get("010-events.cbor").ok_or_else(|| {
            EnvelopeVerificationError::TrellisBundleVerification(
                "missing 010-events.cbor in verified export ZIP".into(),
            )
        })?;
        let sealed_events = decode_events_from_010_cbor(events_bytes)
            .map_err(EnvelopeVerificationError::TrellisBundleVerification)?;

        // Parse 062-signature-affirmations.cbor from the verified ZIP archive.
        let affirmations_bytes = members
            .get("062-signature-affirmations.cbor")
            .ok_or_else(|| {
                EnvelopeVerificationError::TrellisBundleVerification(
                    "missing 062-signature-affirmations.cbor in verified export ZIP".into(),
                )
            })?;
        let affirmation_rows = decode_062_affirmation_rows(affirmations_bytes)
            .map_err(EnvelopeVerificationError::TrellisBundleVerification)?;

        // Parse 066-signed-acts.cbor and cross-check against 062-signature-affirmations.cbor.
        let signed_acts_bytes = members.get("066-signed-acts.cbor").ok_or_else(|| {
            EnvelopeVerificationError::TrellisBundleVerification(
                "missing 066-signed-acts.cbor in verified export ZIP".into(),
            )
        })?;
        let signed_acts_json = decode_cbor_to_json(signed_acts_bytes)
            .map_err(EnvelopeVerificationError::TrellisBundleVerification)?;
        let projected_acts = signed_acts_json["acts"].as_array().ok_or_else(|| {
            EnvelopeVerificationError::TrellisBundleVerification(
                "066-signed-acts.cbor missing acts array".into(),
            )
        })?;
        if projected_acts.len() != affirmation_rows.len() {
            return Err(EnvelopeVerificationError::TrellisBundleVerification(
                format!(
                    "066-signed-acts.cbor act count {} != 062-signature-affirmations.cbor row count {}",
                    projected_acts.len(),
                    affirmation_rows.len()
                ),
            ));
        }

        // Parse 068-signed-acts-manifest.cbor and verify tuple count matches 062 rows.
        let manifest_bytes = members
            .get("068-signed-acts-manifest.cbor")
            .ok_or_else(|| {
                EnvelopeVerificationError::TrellisBundleVerification(
                    "missing 068-signed-acts-manifest.cbor in verified export ZIP".into(),
                )
            })?;
        let manifest_json = decode_cbor_to_json(manifest_bytes)
            .map_err(EnvelopeVerificationError::TrellisBundleVerification)?;
        let manifest_tuples = manifest_json.as_array().ok_or_else(|| {
            EnvelopeVerificationError::TrellisBundleVerification(
                "068-signed-acts-manifest.cbor is not an array".into(),
            )
        })?;
        if manifest_tuples.len() != affirmation_rows.len() {
            return Err(EnvelopeVerificationError::TrellisBundleVerification(
                format!(
                    "068-signed-acts-manifest.cbor tuple count {} != 062 row count {}",
                    manifest_tuples.len(),
                    affirmation_rows.len()
                ),
            ));
        }

        Self::derive_from_verified_records(
            profile,
            fixture,
            verified_formspec_acts,
            &sealed_events,
            &affirmation_rows,
            Some(report),
        )
    }

    fn derive_from_verified_records(
        profile: &str,
        fixture: &JsonValue,
        verified_formspec_acts: &[VerifiedFormspecAct],
        sealed_events: &[JsonValue],
        affirmation_rows: &[JsonValue],
        trellis_report: Option<WosVerificationReport>,
    ) -> Result<Self, EnvelopeVerificationError> {
        // 1. Derive Question 5 (void_status) from verified lifecycle events + K-3 / P-3_voidingPolicy.
        let seal_evt = sealed_events
            .iter()
            .find(|evt| evt["eventType"].as_str() == Some("wos.envelope.sealed"));
        let void_evt = sealed_events
            .iter()
            .find(|evt| evt["eventType"].as_str() == Some("wos.envelope.voided"));
        let seal_rule = &fixture["kernel"]["sealRule"];
        let is_voided = void_evt.is_some()
            || seal_evt
                .and_then(|e| e["data"]["voided"].as_bool())
                .unwrap_or(false)
            || seal_rule["voided"].as_bool().unwrap_or(false);
        let void_reason = void_evt
            .and_then(|e| e["data"]["voidReason"].as_str())
            .or_else(|| seal_evt.and_then(|e| e["data"]["voidReason"].as_str()))
            .or_else(|| seal_rule["voidReason"].as_str())
            .map(str::to_owned);
        let require_sealed_completion = seal_evt
            .and_then(|e| e["data"]["requireSealedCompletion"].as_bool())
            .unwrap_or_else(|| {
                seal_rule["requireSealedCompletion"]
                    .as_bool()
                    .unwrap_or(false)
            });

        let void_status = EnvelopeVoidStatus {
            is_voided,
            is_sealed: !is_voided && seal_evt.is_some(),
            reason: void_reason.clone(),
        };
        if void_status.is_voided && require_sealed_completion {
            return Err(EnvelopeVerificationError::EnvelopeVoided(
                void_reason.unwrap_or_else(|| "envelope voided under K-3 / P-3_voidingPolicy".into()),
            ));
        }

        // 2. Validate P-10_envelopeTimerPolicy across the verified sealed events.
        let timer_port = &fixture["ports"]["P-10_envelopeTimerPolicy"];
        let timer_mode = timer_port["timerAnchoringMode"]
            .as_str()
            .unwrap_or("ledger-clock-only")
            .to_string();
        let require_per_event_anchor = timer_port["requirePerEventAnchor"]
            .as_bool()
            .unwrap_or(false);
        if require_per_event_anchor {
            for (idx, evt) in sealed_events.iter().enumerate() {
                let seq = evt["sequence"].as_u64().unwrap_or(idx as u64);
                let token = evt["tsaAnchorToken"].as_str().unwrap_or_default();
                if token.trim().is_empty() {
                    return Err(EnvelopeVerificationError::TimerAnchoringViolation(format!(
                        "event sequence {seq} missing required per-event tsaAnchorToken under P-10_envelopeTimerPolicy in `{profile}`"
                    )));
                }
            }
        }
        if timer_mode.contains("seal-tsa-rfc3161") && void_status.is_sealed {
            let seal_event = seal_evt.ok_or_else(|| {
                EnvelopeVerificationError::TimerAnchoringViolation(
                    "missing wos.envelope.sealed event under P-10_envelopeTimerPolicy".into(),
                )
            })?;
            let seal_token = seal_event["data"]["tsaAnchorToken"]
                .as_str()
                .or_else(|| seal_event["tsaAnchorToken"].as_str())
                .unwrap_or_default();
            if seal_token.trim().is_empty() {
                return Err(EnvelopeVerificationError::TimerAnchoringViolation(
                    "sealed envelope missing seal tsaAnchorToken under P-10_envelopeTimerPolicy".into(),
                ));
            }
        }

        // 3. Derive Question 6 (proofing_levels) from verified wos.recipient.authenticated events
        //    and check P-8_recipientIdentityProofing.
        let proofing_port = &fixture["ports"]["P-8_recipientIdentityProofing"];
        let av_recording_required = proofing_port["avRecordingRequired"]
            .as_bool()
            .unwrap_or(false);
        let mut proofing_levels = BTreeMap::new();
        for evt in sealed_events
            .iter()
            .filter(|evt| evt["eventType"].as_str() == Some("wos.recipient.authenticated"))
        {
            let data = &evt["data"];
            let recipient_id = data["recipientId"].as_str().unwrap_or_default().to_string();
            let tier = data["proofingTier"].as_str().unwrap_or_default().to_string();
            let assurance = data["assuranceLevel"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let binding_ref = data["identityBindingRef"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            if tier.is_empty() || assurance.is_empty() || binding_ref.is_empty() {
                return Err(EnvelopeVerificationError::IdentityProofingViolation(
                    format!(
                        "incomplete P-8_recipientIdentityProofing evidence for `{recipient_id}`"
                    ),
                ));
            }
            if av_recording_required
                && recipient_id == "applicant"
                && data["avSessionId"]
                    .as_str()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
            {
                return Err(EnvelopeVerificationError::IdentityProofingViolation(
                    "RON profile P-8_recipientIdentityProofing requires avSessionId on applicant authentication".into(),
                ));
            }
            proofing_levels.insert(
                recipient_id.clone(),
                VerifiedProofingLevel {
                    recipient_id,
                    tier,
                    assurance_level: assurance,
                    identity_binding_ref: binding_ref,
                },
            );
        }

        // 4. Derive Question 4 (view_evidence) from verified wos.document.view_recorded events
        //    and check P-2_viewEvidenceCollector.
        let view_port = &fixture["ports"]["P-2_viewEvidenceCollector"];
        let require_scroll = view_port["requireScrollCompletion"]
            .as_bool()
            .unwrap_or(false);
        let min_page_dwell_ms = view_port["minPageDwellMs"].as_u64();
        let mut view_evidence = Vec::new();
        for evt in sealed_events
            .iter()
            .filter(|evt| evt["eventType"].as_str() == Some("wos.document.view_recorded"))
        {
            let data = &evt["data"];
            let recipient_id = data["recipientId"].as_str().unwrap_or_default().to_string();
            let document_id = data["documentId"].as_str().unwrap_or_default().to_string();
            let presentation_hash = data["presentationHash"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let scroll_completed = data["scrollCompleted"].as_bool().unwrap_or(false);
            if require_scroll && !scroll_completed {
                return Err(EnvelopeVerificationError::ViewEvidenceViolation(format!(
                    "recipient `{recipient_id}` did not complete scroll on `{document_id}` under P-2_viewEvidenceCollector"
                )));
            }
            if let Some(required_dwell) = min_page_dwell_ms {
                let observed = data["minPageDwellMsObserved"].as_u64().unwrap_or(0);
                if observed < required_dwell {
                    return Err(EnvelopeVerificationError::ViewEvidenceViolation(format!(
                        "recipient `{recipient_id}` dwell {observed}ms < required {required_dwell}ms on `{document_id}` under P-2_viewEvidenceCollector"
                    )));
                }
            }
            view_evidence.push(VerifiedViewEvidence {
                recipient_id,
                document_id,
                presentation_hash,
                opened_at: data["openedAt"].as_str().unwrap_or_default().to_string(),
                completed_scroll_at: data["completedScrollAt"].as_str().map(str::to_owned),
                scroll_completed,
                dwell_ms: data["dwellMs"].as_u64().unwrap_or(0),
            });
        }

        // 5. Validate P-11_documentSourceResolver allowedContentSubKinds and derive Question 2
        //    (what_was_signed) from verified 062-signature-affirmations.cbor + declared documents.
        let allowed_sub_kinds = fixture["ports"]["P-11_documentSourceResolver"]
            ["allowedContentSubKinds"]
            .as_array()
            .ok_or_else(|| {
                EnvelopeVerificationError::FormspecValidation(
                    "missing P-11_documentSourceResolver.allowedContentSubKinds".into(),
                )
            })?
            .iter()
            .filter_map(JsonValue::as_str)
            .collect::<Vec<_>>();

        let declared_docs = sealed_events
            .iter()
            .find(|evt| evt["eventType"].as_str() == Some("wos.envelope.documents_declared"))
            .and_then(|evt| evt["data"]["documents"].as_array())
            .or_else(|| fixture["kernel"]["documents"].as_array())
            .ok_or_else(|| {
                EnvelopeVerificationError::FormspecValidation("missing declared documents".into())
            })?;

        let mut what_was_signed = Vec::new();
        for doc in declared_docs {
            let doc_id = doc["documentId"].as_str().unwrap_or_default().to_string();
            let sub_kind = doc["contentSubKind"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            if !allowed_sub_kinds.contains(&sub_kind.as_str()) {
                return Err(EnvelopeVerificationError::DisallowedContentSubKind(
                    format!(
                        "document `{doc_id}` sub-kind `{sub_kind}` not permitted by P-11_documentSourceResolver {allowed_sub_kinds:?}"
                    ),
                ));
            }
            let declared_sha = doc["sha256"].as_str().unwrap_or_default().to_string();
            let declared_pres = doc["presentationHash"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            // Verify this document appears in the verified 062-signature-affirmations.cbor rows
            // with matching document_hash and presentation_hash.
            let matching_row = affirmation_rows
                .iter()
                .find(|row| row["document_id"].as_str() == Some(doc_id.as_str()))
                .ok_or_else(|| {
                    EnvelopeVerificationError::DocumentDigestMismatch(format!(
                        "declared document `{doc_id}` missing from verified 062-signature-affirmations.cbor"
                    ))
                })?;
            let row_doc_hash = matching_row["document_hash"]
                .as_str()
                .unwrap_or_default()
                .strip_prefix("sha256:")
                .unwrap_or_default();
            let row_pres_hash = matching_row["presentation_hash"]
                .as_str()
                .unwrap_or_default();
            if row_doc_hash != declared_sha {
                return Err(EnvelopeVerificationError::DocumentDigestMismatch(format!(
                    "verified 062 document_hash `{row_doc_hash}` != declared `{declared_sha}` for `{doc_id}`"
                )));
            }
            if row_pres_hash != declared_pres {
                return Err(EnvelopeVerificationError::DocumentDigestMismatch(format!(
                    "verified 062 presentation_hash `{row_pres_hash}` != declared `{declared_pres}` for `{doc_id}`"
                )));
            }

            what_was_signed.push(VerifiedSignedDocument {
                document_id: doc_id,
                sha256: row_doc_hash.to_string(),
                presentation_hash: row_pres_hash.to_string(),
                content_sub_kind: sub_kind,
            });
        }

        // 6. Derive Question 1 (who_signed) and Question 3 (when_signed) directly from the
        //    verified 062-signature-affirmations.cbor rows and cross-check K-5 / P-2 / P-8.
        let mut who_signed = Vec::with_capacity(affirmation_rows.len());
        let mut when_signed = Vec::with_capacity(affirmation_rows.len());

        for row in affirmation_rows {
            let signing_act_id = row["signing_act_id"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let signer_id = row["signer_id"].as_str().unwrap_or_default().to_string();
            let role_id = row["role_id"].as_str().unwrap_or_default().to_string();
            let role = row["role"].as_str().unwrap_or_default().to_string();
            let doc_id = row["document_id"].as_str().unwrap_or_default().to_string();
            let signed_at = row["signed_at"].as_str().unwrap_or_default().to_string();
            let row_pres_hash = row["presentation_hash"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let witnessed_signature_ref = row["witnessed_signature_ref"]
                .as_str()
                .map(str::to_owned);

            // Check P-8 proofing exists for this signer.
            if !proofing_levels.contains_key(&signer_id) {
                return Err(EnvelopeVerificationError::IdentityProofingViolation(
                    format!(
                        "signer `{signer_id}` has no verified wos.recipient.authenticated event under P-8_recipientIdentityProofing"
                    ),
                ));
            }

            // For principal signers, check P-2 viewEvidence exists for the attested document
            // and matches the verified presentation_hash (P-12_presentationHasher).
            if witnessed_signature_ref.is_none() {
                let view = view_evidence
                    .iter()
                    .find(|v| v.recipient_id == signer_id && v.document_id == doc_id)
                    .ok_or_else(|| {
                        EnvelopeVerificationError::ViewEvidenceViolation(format!(
                            "principal signer `{signer_id}` missing P-2 viewEvidence for `{doc_id}` before signing"
                        ))
                    })?;
                if view.presentation_hash != row_pres_hash {
                    return Err(EnvelopeVerificationError::DocumentDigestMismatch(format!(
                        "viewEvidence presentationHash `{}` != verified 062 presentation_hash `{row_pres_hash}` for `{doc_id}`",
                        view.presentation_hash
                    )));
                }
            }

            who_signed.push(VerifiedSignerAct {
                signing_act_id: signing_act_id.clone(),
                signer_id,
                role_id,
                role,
                witnessed_signature_ref,
            });
            when_signed.push(VerifiedSigningTimestamp {
                signing_act_id,
                signed_at,
                timer_anchoring_mode: timer_mode.clone(),
            });
        }

        // 7. Check P-6_intentRegistry agent proposal ratification across verified events.
        let intent_port = &fixture["ports"]["P-6_intentRegistry"];
        let human_ratification_required = intent_port["humanRatificationRequired"]
            .as_bool()
            .unwrap_or(false);
        for evt in sealed_events
            .iter()
            .filter(|evt| evt["eventType"].as_str() == Some("wos.agent.proposal.recorded"))
        {
            if human_ratification_required
                || evt["data"]["requiresHumanRatification"]
                    .as_bool()
                    .unwrap_or(false)
            {
                if let Some(proposal_id) = evt["data"]["proposalId"].as_str() {
                    let ratified = verified_formspec_acts.iter().any(|act| {
                        act.witnessed_ref.is_none()
                            && act.ratified_agent_proposal_id.as_deref() == Some(proposal_id)
                    });
                    if !ratified {
                        return Err(EnvelopeVerificationError::UnratifiedAgentProposal(format!(
                            "agent proposal `{proposal_id}` under P-6_intentRegistry was not ratified by a human principal signing act"
                        )));
                    }
                }
            }
        }

        Ok(Self {
            profile: profile.to_string(),
            who_signed,
            what_was_signed,
            when_signed,
            view_evidence,
            void_status,
            proofing_levels,
            trellis_report,
        })
    }

    /// Profile identifier (`basic-esign`, `ron`, `compliance-strict`, etc.).
    #[must_use]
    pub fn profile(&self) -> &str {
        &self.profile
    }

    /// Question 1: Who signed, in what role, and witnessing which prior signature (`K-5`, if any)?
    #[must_use]
    pub fn who_signed(&self) -> &[VerifiedSignerAct] {
        &self.who_signed
    }

    /// Question 2: What exact documents (SHA-256 + `presentationHash` + sub-kind) were signed (`K-1` / `P-12`)?
    #[must_use]
    pub fn what_was_signed(&self) -> &[VerifiedSignedDocument] {
        &self.what_was_signed
    }

    /// Question 3: When did each signing act occur, and under what timer anchoring mode (`P-10`)?
    #[must_use]
    pub fn when_signed(&self) -> &[VerifiedSigningTimestamp] {
        &self.when_signed
    }

    /// Question 4: What pre-signature view evidence (`P-2_viewEvidenceCollector`) was recorded for `(recipient, document)`?
    #[must_use]
    pub fn view_evidence_for(
        &self,
        recipient_id: &str,
        document_id: &str,
    ) -> Option<&VerifiedViewEvidence> {
        self.view_evidence
            .iter()
            .find(|v| v.recipient_id == recipient_id && v.document_id == document_id)
    }

    /// Returns all verified pre-signature view evidence records in lifecycle order.
    #[must_use]
    pub fn all_view_evidence(&self) -> &[VerifiedViewEvidence] {
        &self.view_evidence
    }

    /// Question 5: Is the envelope sealed (`K-7`) or voided (`K-3` / `P-3_voidingPolicy`)?
    #[must_use]
    pub fn void_status(&self) -> &EnvelopeVoidStatus {
        &self.void_status
    }

    /// Question 6: What identity proofing tier and assurance level (`P-8_recipientIdentityProofing`) bound `recipient_id`?
    #[must_use]
    pub fn proofing_level_for(&self, recipient_id: &str) -> Option<&VerifiedProofingLevel> {
        self.proofing_levels.get(recipient_id)
    }

    /// Direct access to the underlying Trellis structural and domain verification report
    /// (`signature-ports.md` §3.6: the facade wraps `verify_export_zip`, never replaces it).
    #[must_use]
    pub fn structural_inspector(&self) -> Option<&WosVerificationReport> {
        self.trellis_report.as_ref()
    }

    /// Demands a self-contained offline verification bundle (`P-1_documentBytesStore` in CAS mode).
    ///
    /// # Errors
    /// Refuses with [`EnvelopeVerificationError::ServerOnlyPostureNoOfflineBundle`] when the
    /// envelope was governed under `wos-server-only` (`intranet`).
    pub fn require_offline_bundle(&self) -> Result<&WosVerificationReport, EnvelopeVerificationError> {
        self.trellis_report.as_ref().ok_or_else(|| {
            EnvelopeVerificationError::ServerOnlyPostureNoOfflineBundle(format!(
                "profile `{}` uses wos-server-only P-1_documentBytesStore posture and does not emit an offline ZIP bundle",
                self.profile
            ))
        })
    }
}

struct FixedClock;
impl wos_runtime::Clock for FixedClock {
    fn now_ms(&self) -> u64 {
        1_760_987_100_000
    }
}

fn load_profile_fixture(profile: &str) -> JsonValue {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/conformance/envelope-profiles")
        .join(format!("{profile}.json"));
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()))
}

fn default_method_registry() -> MethodRegistry {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/document-attestations/registry.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn active_task_for(
    actor: &str,
    definition_url: &str,
    definition_version: &str,
) -> wos_core::instance::ActiveTask {
    serde_json::from_value(json!({
        "taskId": "wos-task:test:1:terms",
        "taskRef": "terms",
        "status": "assigned",
        "assignedActor": actor,
        "binding": "formspec",
        "definitionUrl": definition_url,
        "definitionVersion": definition_version,
        "createdAt": "2026-10-20T19:00:00Z",
        "updatedAt": "2026-10-20T19:00:00Z"
    }))
    .unwrap()
}

/// Validates and cryptographically verifies all Formspec signing acts declared in `fixture`.
fn verify_formspec_signing_acts(
    fixture: &JsonValue,
) -> Result<Vec<VerifiedFormspecAct>, EnvelopeVerificationError> {
    let profile = fixture["profile"].as_str().unwrap_or("unknown");
    let definition = &fixture["definition"];
    let declared_docs = fixture["kernel"]["documents"].as_array().ok_or_else(|| {
        EnvelopeVerificationError::FormspecValidation("missing kernel.documents".into())
    })?;
    let events = fixture["lifecycleEvents"].as_array().ok_or_else(|| {
        EnvelopeVerificationError::FormspecValidation("missing lifecycleEvents".into())
    })?;

    // Check K-5 / K-6 / P-8 notary & witness requirements on kernel.signingActs.
    let signing_acts = fixture["kernel"]["signingActs"].as_array().ok_or_else(|| {
        EnvelopeVerificationError::FormspecValidation("missing kernel.signingActs".into())
    })?;
    if signing_acts.is_empty() {
        return Err(EnvelopeVerificationError::SignatureVerification(
            "envelope contains no signingActs".into(),
        ));
    }

    let proofing_port = &fixture["ports"]["P-8_recipientIdentityProofing"];
    let notary_commission_required = proofing_port["notaryCommissionRequired"]
        .as_bool()
        .unwrap_or(false);
    let physical_presence_attested = proofing_port["physicalPresenceAttested"]
        .as_bool()
        .unwrap_or(false);
    if notary_commission_required {
        let notary_act = signing_acts
            .iter()
            .find(|act| !act["witnessedSignatureRef"].is_null())
            .ok_or_else(|| {
                EnvelopeVerificationError::WitnessNotaryViolation(
                    "profile requires a K-5 notary/witness countersignature act".into(),
                )
            })?;
        let commission = &notary_act["notaryCommission"];
        if commission.is_null()
            || commission["commissionId"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || commission["jurisdiction"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(EnvelopeVerificationError::WitnessNotaryViolation(
                "notary signing act missing notaryCommission metadata under P-8_recipientIdentityProofing".into(),
            ));
        }
        if profile == "ron"
            && commission["avRecordingDigest"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(EnvelopeVerificationError::WitnessNotaryViolation(
                "RON notary commission missing avRecordingDigest".into(),
            ));
        }
        if physical_presence_attested
            && !commission["physicalPresenceVerified"]
                .as_bool()
                .unwrap_or(false)
        {
            return Err(EnvelopeVerificationError::WitnessNotaryViolation(
                "IPEN notary commission missing physicalPresenceVerified=true under K-6 / P-8".into(),
            ));
        }
    }

    // Collect authentication metadata per recipient for Formspec identityBinding construction.
    let mut auth_by_recipient = BTreeMap::new();
    for evt in events
        .iter()
        .filter(|evt| evt["eventType"].as_str() == Some("wos.recipient.authenticated"))
    {
        let data = &evt["data"];
        let recipient_id = data["recipientId"].as_str().unwrap_or_default().to_string();
        auth_by_recipient.insert(
            recipient_id.clone(),
            VerifiedProofingLevel {
                recipient_id,
                tier: data["proofingTier"].as_str().unwrap_or("email-otp").to_string(),
                assurance_level: data["assuranceLevel"]
                    .as_str()
                    .unwrap_or("standard")
                    .to_string(),
                identity_binding_ref: data["identityBindingRef"]
                    .as_str()
                    .unwrap_or("identity-default")
                    .to_string(),
            },
        );
    }

    let processor = ImmutableFormspecProcessor::new(&[SelectedDocument {
        kind: SelectedDocumentKind::Definition,
        url: definition["url"].as_str().unwrap_or_default().into(),
        version: definition["version"].as_str().unwrap_or_default().into(),
        bytes: serde_json::to_vec(definition).unwrap(),
        definition: None,
    }])
    .map_err(|err| EnvelopeVerificationError::FormspecValidation(err.to_string()))?
    .with_clock(FixedClock);

    let mut signer_keys = BTreeMap::new();
    let mut signed_responses = Vec::with_capacity(signing_acts.len());
    let mut principal_digest_for_witness: Option<(String, String)> = None;

    for (act_idx, act) in signing_acts.iter().enumerate() {
        let act_id = act["signingActId"].as_str().unwrap_or_default().to_string();
        let field_path = act["fieldPath"].as_str().unwrap_or_else(|| {
            if act["witnessedSignatureRef"].is_null() {
                "signature"
            } else {
                "confirm"
            }
        });
        let signer_id = act["signerId"].as_str().unwrap_or_default().to_string();
        let role_id = act["roleId"].as_str().unwrap_or_default().to_string();
        let role = act["role"].as_str().unwrap_or_default().to_string();
        let signed_at = act["signedAt"].as_str().unwrap_or_default().to_string();
        let signing_intent = act["signingIntent"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let ceremony_id = act["ceremonyId"].as_str().unwrap_or_default().to_string();
        let consent_ref = act["consentReference"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let witnessed_ref = act["witnessedSignatureRef"].as_str().map(str::to_owned);
        let ratified_agent_proposal_id = act["ratifiedAgentProposalId"]
            .as_str()
            .map(str::to_owned);

        let signer_proofing = auth_by_recipient.get(&signer_id).ok_or_else(|| {
            EnvelopeVerificationError::IdentityProofingViolation(format!(
                "signer `{signer_id}` has no wos.recipient.authenticated event"
            ))
        })?;

        let attested_doc_ids = act["attestedDocumentIds"]
            .as_array()
            .ok_or_else(|| {
                EnvelopeVerificationError::FormspecValidation("missing attestedDocumentIds".into())
            })?
            .iter()
            .filter_map(JsonValue::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>();

        let response_id = format!("{profile}-response-{act_idx}");
        let is_witness = witnessed_ref.is_some();
        let (response_json, public_key_ref, payload_digest) = build_and_sign_formspec_response(
            definition,
            declared_docs,
            &response_id,
            &act_id,
            field_path,
            &signer_id,
            &signed_at,
            &signing_intent,
            &ceremony_id,
            &consent_ref,
            signer_proofing,
            &attested_doc_ids,
            witnessed_ref.as_deref(),
            principal_digest_for_witness.as_ref(),
            signer_keys.get(&signer_id),
        )?;

        if !is_witness && principal_digest_for_witness.is_none() {
            principal_digest_for_witness = Some((response_id.clone(), payload_digest));
        }

        let validation_errors = processor
            .validate_envelope(response_json.value())
            .map_err(|err| EnvelopeVerificationError::FormspecValidation(err.to_string()))?;
        if !validation_errors.is_empty() {
            return Err(EnvelopeVerificationError::FormspecValidation(format!(
                "Formspec validation errors: {validation_errors:?}"
            )));
        }

        signer_keys.insert(signer_id.clone(), public_key_ref);
        signed_responses.push((
            act_id,
            signer_id,
            role_id,
            role,
            consent_ref,
            witnessed_ref,
            ratified_agent_proposal_id,
            response_json,
        ));
    }

    let binding = FormspecBinding::new(processor).with_document_verification(
        DocumentSignatureVerification::new(
            RingVerifier::new_for_method_prefix("urn:formspec:sig-method:"),
            default_method_registry(),
            signer_keys,
        ),
    );

    let mut verified_acts = Vec::with_capacity(signed_responses.len());
    for (
        act_id,
        signer_id,
        role_id,
        role,
        consent_ref,
        witnessed_ref,
        ratified_agent_proposal_id,
        response_json,
    ) in signed_responses
    {
        let active_task = active_task_for(
            &signer_id,
            definition["url"].as_str().unwrap_or_default(),
            definition["version"].as_str().unwrap_or_default(),
        );
        let evidence_list = binding
            .signature_evidence(&active_task, &response_json)
            .map_err(|err| EnvelopeVerificationError::SignatureVerification(err.to_string()))?
            .ok_or_else(|| {
                EnvelopeVerificationError::SignatureVerification(
                    "expected signature evidence from FormspecBinding".into(),
                )
            })?;

        let retained_list = binding
            .retained_document_signature_evidence(&response_json, &active_task.task_id)
            .map_err(|err| EnvelopeVerificationError::SignatureVerification(err.to_string()))?;
        if evidence_list.len() != retained_list.len() {
            return Err(EnvelopeVerificationError::SignatureVerification(
                "retained document signature evidence count mismatch".into(),
            ));
        }

        for evidence in &evidence_list {
            if evidence.primitive_verification != SignaturePrimitiveStatus::Verified {
                return Err(EnvelopeVerificationError::SignatureVerification(format!(
                    "primitive verification failed for `{act_id}`: {:?}",
                    evidence.primitive_verification
                )));
            }
        }

        verified_acts.push(VerifiedFormspecAct {
            signing_act_id: act_id,
            role_id,
            role,
            consent_ref,
            witnessed_ref,
            ratified_agent_proposal_id,
            evidences: evidence_list,
        });
    }

    Ok(verified_acts)
}

#[allow(clippy::too_many_arguments)]
fn build_and_sign_formspec_response(
    definition: &JsonValue,
    declared_docs: &[JsonValue],
    response_id: &str,
    signing_act_id: &str,
    field_path: &str,
    signer_id: &str,
    signed_at: &str,
    signing_intent: &str,
    ceremony_id: &str,
    consent_ref: &str,
    signer_proofing: &VerifiedProofingLevel,
    attested_doc_ids: &[String],
    witnessed_sig_ref: Option<&str>,
    principal_digest_for_witness: Option<&(String, String)>,
    existing_key: Option<&KeyRef>,
) -> Result<(ResponseJson, KeyRef, String), EnvelopeVerificationError> {
    let is_witness = witnessed_sig_ref.is_some();
    let authority_extension = if is_witness {
        json!({
            "class": "witness",
            "evidenceBinding": {
                "evidenceHash": "9a97685c368331e79f1d699137b400dc394f4c4bc65acf0b4817a4daadbb2f83",
                "evidenceHashAlgorithm": "sha-256"
            }
        })
    } else {
        json!({ "class": "self" })
    };

    let witnessed_obj = if let Some(sig_id) = witnessed_sig_ref {
        let (principal_resp_id, principal_digest) =
            principal_digest_for_witness.ok_or_else(|| {
                EnvelopeVerificationError::WitnessNotaryViolation(
                    "witness signing act precedes principal signing act".into(),
                )
            })?;
        json!({
            "responseId": principal_resp_id,
            "signatureId": sig_id,
            "signedPayloadDigest": principal_digest
        })
    } else {
        JsonValue::Null
    };

    let formspec_identity_method = match signer_proofing.tier.as_str() {
        "none" | "email-otp" | "sms-otp" | "knowledge-based" | "oidc" | "webauthn"
        | "credential" | "in-person" | "notary" => signer_proofing.tier.clone(),
        other => format!("x-{other}"),
    };
    let formspec_assurance_level = match signer_proofing.assurance_level.as_str() {
        "none" | "low" | "standard" | "high" | "qualified" => {
            signer_proofing.assurance_level.as_str()
        }
        "IAL3-AAL3" | "IAL3-FIDO2" | "IAL2-RON" | "IAL2-RON-NOTARY" | "IPEN-Physical-ID"
        | "IPEN-Notary-Commission" => "high",
        _ => "standard",
    };

    let mut resolved_documents = Vec::with_capacity(attested_doc_ids.len());
    let mut attestations = Vec::with_capacity(attested_doc_ids.len());
    let mut authored_signatures = Vec::with_capacity(attested_doc_ids.len());

    for (idx, doc_id) in attested_doc_ids.iter().enumerate() {
        let doc = declared_docs
            .iter()
            .find(|d| d["documentId"].as_str() == Some(doc_id.as_str()))
            .ok_or_else(|| {
                EnvelopeVerificationError::DocumentDigestMismatch(format!(
                    "signing act references undeclared document `{doc_id}`"
                ))
            })?;
        let locale = doc["locale"].as_str().unwrap_or("en-US");
        let sig_id = if is_witness {
            format!("sig-witness-{}", idx + 1)
        } else {
            format!("sig-{}", idx + 1)
        };
        let doc_hash = doc["sha256"].as_str().unwrap_or_default();
        let pres_hash = doc["presentationHash"].as_str().unwrap_or_default();
        let media_type = doc["mediaType"].as_str().unwrap_or("text/plain");
        let inline_data = doc["inlineDataBase64"].as_str().unwrap_or("wqFIb2xhIQ==");

        resolved_documents.push(json!({
            "documentId": doc_id,
            "locale": locale,
            "source": {
                "kind": "inline",
                "data": inline_data
            },
            "mediaType": media_type,
            "documentHash": doc_hash,
            "presentationHash": pres_hash,
            "pinnedAt": "2026-10-20T19:00:00Z"
        }));

        attestations.push(json!({
            "signatureId": sig_id,
            "documentRef": {
                "documentId": doc_id,
                "locale": locale
            },
            "witnessedSignatureRef": witnessed_obj
        }));

        authored_signatures.push(json!({
            "signatureId": sig_id,
            "documentId": doc_id,
            "signingIntent": signing_intent,
            "signatureValue": "",
            "signerName": format!("Signer {signer_id}"),
            "consentAccepted": true,
            "consentTextRef": consent_ref,
            "consentVersion": "1.0",
            "affirmationText": "I attest to the declared envelope documents.",
            "signedPayload": {
                "canonicalization": "formspec-response-signing-v1",
                "digestAlgorithm": "sha-256",
                "digest": "",
                "responseId": response_id,
                "definitionUrl": definition["url"],
                "definitionVersion": definition["version"],
                "signedAt": signed_at,
                "signingIntent": signing_intent
            },
            "documentHash": doc_hash,
            "documentHashAlgorithm": "sha-256",
            "signatureProvider": "urn:example:signing-provider",
            "ceremonyId": ceremony_id,
            "signerId": signer_id,
            "identityBinding": {
                "method": formspec_identity_method,
                "assuranceLevel": formspec_assurance_level,
                "providerRef": signer_proofing.identity_binding_ref
            },
            "signingActId": signing_act_id,
            "documentRef": {
                "documentId": doc_id,
                "locale": locale
            },
            "presentationHash": pres_hash,
            "witnessedSignatureRef": witnessed_obj
        }));
    }

    let response_value = json!({
        "$formspecResponse": "1.0",
        "definitionUrl": definition["url"],
        "definitionVersion": definition["version"],
        "status": "completed",
        "data": {
            "signature": null,
            "disclosureSignature": null,
            "confirm": null
        },
        "authored": "2026-10-20T19:00:00Z",
        "validationResults": [],
        "id": response_id,
        "author": {
            "id": signer_id,
            "name": format!("Signer {signer_id}")
        },
        "extensions": {
            "x-wos-signer-authority": authority_extension,
            "x-formspec-document-attestations": {
                "version": "1.0",
                "signingActId": signing_act_id,
                "fieldPath": field_path,
                "signerId": signer_id,
                "signerName": format!("Signer {signer_id}"),
                "signedAt": signed_at,
                "signingIntent": signing_intent,
                "consentAccepted": true,
                "consentTextRef": consent_ref,
                "consentVersion": "1.0",
                "affirmationText": "I attest to the declared envelope documents.",
                "signatureProvider": "urn:example:signing-provider",
                "ceremonyId": ceremony_id,
                "attestations": attestations,
                "identityBinding": {
                    "method": formspec_identity_method,
                    "assuranceLevel": formspec_assurance_level,
                    "providerRef": signer_proofing.identity_binding_ref
                }
            }
        },
        "resolvedDocuments": resolved_documents,
        "authoredSignatures": authored_signatures
    });

    let mut source = ResponseJson::from_value(&response_value)
        .map_err(|err| EnvelopeVerificationError::FormspecValidation(err.to_string()))?;

    // Deterministic per-signer Ed25519 key pair so multiple acts by the same signer share one key.
    let mut seed = [0u8; 32];
    let signer_bytes = signer_id.as_bytes();
    for (i, b) in signer_bytes.iter().enumerate() {
        seed[i % 32] ^= b.wrapping_add((i as u8).wrapping_mul(17));
    }
    seed[0] ^= 0x5a;
    let key = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
    let expected_key_ref = KeyRef::RawPublicKey(key.public_key().as_ref().to_vec());
    if let Some(existing) = existing_key {
        debug_assert_eq!(existing, &expected_key_ref);
    }

    let payload = source
        .signed_payload(DigestAlgorithm::Sha256)
        .map_err(|err| EnvelopeVerificationError::SignatureVerification(err.to_string()))?;
    let protected = integrity_cose::detached_signature_protected_header(
        -8,
        signer_id.as_bytes(),
        "urn:formspec:sig-method:ed25519-cose-sign1@1",
    );
    let signature = key.sign(&integrity_cose::sig_structure_bytes(
        &protected,
        &payload.canonical_bytes,
    ));
    let cose = integrity_cose::encode_cose_sign1(&protected, None, signature.as_ref());
    let mut signatures = source.value()["authoredSignatures"].clone();
    for record in signatures.as_array_mut().unwrap() {
        record["signedPayload"]["digest"] = payload.digest.clone().into();
        record["signatureValue"] = STANDARD.encode(&cose).into();
    }
    source = source
        .with_member("authoredSignatures", &signatures)
        .map_err(|err| EnvelopeVerificationError::SignatureVerification(err.to_string()))?;

    Ok((source, expected_key_ref, payload.digest))
}

#[derive(Default)]
struct MemoryLedger {
    events: Vec<StoredEvent>,
}

impl LedgerStore for MemoryLedger {
    type Error = std::io::Error;
    fn append_event(&mut self, event: StoredEvent) -> Result<(), Self::Error> {
        self.events.push(event);
        Ok(())
    }
}

fn cbor_map(entries: Vec<(&str, CborValue)>) -> CborValue {
    CborValue::Map(
        entries
            .into_iter()
            .map(|(k, v)| (CborValue::Text(k.to_string()), v))
            .collect(),
    )
}

fn issue_checkpoints_for_events(
    scope: &[u8],
    events: &[StoredEvent],
    signing_key: &[u8],
    timestamps: &[TrellisTimestamp],
) -> Vec<StoredCheckpoint> {
    let key = CoreSigningKey::new(signing_key.to_vec());
    let mut prior: Option<trellis_core::checkpoint::IssuedCheckpoint> = None;
    events
        .iter()
        .zip(timestamps)
        .map(|(event, timestamp)| {
            let issued = trellis_core::checkpoint::issue_checkpoint(
                &key,
                scope,
                prior
                    .as_ref()
                    .map(|p| trellis_core::checkpoint::PriorCheckpoint {
                        digest: p.digest,
                        frontier: &p.frontier,
                    }),
                *event
                    .canonical_event_hash()
                    .expect("event has canonical_event_hash"),
                (timestamp.unix_secs, timestamp.subsec_nanos),
            )
            .expect("issue_checkpoint");
            let stored = issued.stored(scope, Vec::new());
            prior = Some(issued);
            stored
        })
        .collect()
}

/// Builds a complete Trellis export ZIP containing the envelope's lifecycle events and
/// `wos.signature.affirmation.recorded` events with their exact `signingActId` verbatim.
fn build_trellis_export_zip(
    fixture: &JsonValue,
    verified_acts: &[VerifiedFormspecAct],
) -> Result<Vec<u8>, EnvelopeVerificationError> {
    let scope = fixture["kernel"]["caseLedgerScope"]
        .as_str()
        .unwrap_or("case-default")
        .as_bytes();
    let vectors_root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../trellis/fixtures/vectors");
    let cose_key_bytes = std::fs::read(vectors_root.join("_keys/issuer-001.cose_key"))
        .map_err(|err| {
            EnvelopeVerificationError::TrellisBundleVerification(format!(
                "read issuer-001.cose_key: {err}"
            ))
        })?;
    let parsed_key = trellis_cddl::parse_ed25519_cose_key(&cose_key_bytes).map_err(|err| {
        EnvelopeVerificationError::TrellisBundleVerification(format!(
            "parse issuer-001.cose_key: {err}"
        ))
    })?;
    let core_key = CoreSigningKey::new(cose_key_bytes.clone());

    let mut store = MemoryLedger::default();
    let mut previous_hash: Option<[u8; 32]> = None;
    let mut timestamps = Vec::new();
    let mut seq_idx: u64 = 0;

    let mut append_json_event = |event_type: &str,
                                 payload_json: &JsonValue|
     -> Result<(), EnvelopeVerificationError> {
        let payload_bytes = json_to_dcbor_bytes(payload_json, &[]).map_err(|err| {
            EnvelopeVerificationError::TrellisBundleVerification(format!(
                "json_to_dcbor_bytes: {err}"
            ))
        })?;
        let content_hash = domain_separated_sha256(CONTENT_DOMAIN, &payload_bytes);
        let unix_secs = 1_745_000_100 + seq_idx * 10;
        let header = cbor_map(vec![
            ("event_type", CborValue::Bytes(event_type.as_bytes().to_vec())),
            (
                "authored_at",
                CborValue::Array(vec![
                    CborValue::Integer(unix_secs.into()),
                    CborValue::Integer(0.into()),
                ]),
            ),
            ("retention_tier", CborValue::Integer(1.into())),
            (
                "classification",
                CborValue::Bytes(b"x-trellis-service/public-metadata".to_vec()),
            ),
            ("outcome_commitment", CborValue::Null),
            ("subject_ref_commitment", CborValue::Null),
            ("tag_commitment", CborValue::Null),
            ("witness_ref", CborValue::Null),
            ("extensions", CborValue::Null),
        ]);
        let payload_ref = cbor_map(vec![
            ("ref_type", CborValue::Text("inline".to_string())),
            ("ciphertext", CborValue::Bytes(payload_bytes)),
        ]);
        let authored_cbor = encode_canonical_cbor_value(&cbor_map(vec![
            ("version", CborValue::Integer(1.into())),
            ("ledger_scope", CborValue::Bytes(scope.to_vec())),
            ("sequence", CborValue::Integer(seq_idx.into())),
            (
                "prev_hash",
                previous_hash.map_or(CborValue::Null, |h| CborValue::Bytes(h.to_vec())),
            ),
            ("causal_deps", CborValue::Null),
            ("content_hash", CborValue::Bytes(content_hash.to_vec())),
            ("header", header),
            ("commitments", CborValue::Null),
            ("payload_ref", payload_ref),
            ("key_bag", CborValue::Null),
            (
                "idempotency_key",
                CborValue::Bytes(format!("idem-{seq_idx}").into_bytes()),
            ),
            ("extensions", CborValue::Null),
        ]))
        .map_err(|err| {
            EnvelopeVerificationError::TrellisBundleVerification(format!(
                "encode authored event: {err}"
            ))
        })?;

        let artifacts = append_event(&mut store, &core_key, &AuthoredEvent::new(authored_cbor))
            .map_err(|err| {
                EnvelopeVerificationError::TrellisBundleVerification(format!("append_event: {err}"))
            })?;
        let canonical_hash = artifacts.canonical_event_hash;
        let last = store.events.last_mut().expect("event appended");
        *last = last.clone().with_canonical_event_hash(Some(canonical_hash));
        previous_hash = Some(canonical_hash);
        timestamps.push(TrellisTimestamp::new(unix_secs, 0).expect("timestamp"));
        seq_idx += 1;
        Ok(())
    };

    // 1. Append non-affirmation lifecycle events before sealing (`wos.case.created`,
    //    `wos.envelope.documents_declared`, `wos.agent.proposal.recorded`,
    //    `wos.recipient.authenticated`, `wos.document.view_recorded`).
    let lifecycle_events = fixture["lifecycleEvents"].as_array().ok_or_else(|| {
        EnvelopeVerificationError::FormspecValidation("missing lifecycleEvents".into())
    })?;
    for evt in lifecycle_events {
        let event_type = evt["eventType"].as_str().unwrap_or_default();
        if event_type == "wos.signature.affirmation.recorded"
            || event_type == "wos.envelope.sealed"
            || event_type == "wos.envelope.voided"
        {
            continue;
        }
        let mut enriched = evt.clone();
        if event_type == "wos.envelope.documents_declared" {
            enriched["data"]["documents"] = fixture["kernel"]["documents"].clone();
        }
        append_json_event(event_type, &enriched)?;
    }

    // 2. Append canonical WOS SignatureAffirmation events using each act's `signingActId`
    //    VERBATIM (never rewritten with `#{document_id}`).
    let sig_event_literal = wos_signature_affirmation_event_type();
    let mut affirmation_idx = 0usize;
    for act in verified_acts {
        for evidence in &act.evidences {
            let claims = evidence.document_claims.as_ref().ok_or_else(|| {
                EnvelopeVerificationError::SignatureVerification("missing document_claims".into())
            })?;
            let verbatim_act_id = evidence
                .signing_act_id
                .as_deref()
                .unwrap_or(&act.signing_act_id);
            let matching_lifecycle = lifecycle_events.iter().find(|evt| {
                evt["eventType"].as_str() == Some("wos.signature.affirmation.recorded")
                    && evt["data"]["signingActId"].as_str() == Some(verbatim_act_id)
            });
            let mut event_json = json!({
                "event": sig_event_literal,
                "data": {
                    "signerId": evidence.signer_id,
                    "roleId": act.role_id,
                    "role": act.role,
                    "documentId": evidence.document_id,
                    "documentHash": format!("sha256:{}", evidence.document_hash),
                    "documentHashAlgorithm": evidence.document_hash_algorithm,
                    "presentationHash": claims.presentation_hash,
                    "signedAt": evidence.signed_at,
                    "identityBinding": evidence.identity_binding,
                    "consentReference": {
                        "textRef": act.consent_ref,
                        "version": "1.0"
                    },
                    "signatureProvider": evidence.signature_provider,
                    "ceremonyId": evidence.ceremony_id,
                    "sourceSignatureSystem": evidence.source_system,
                    "sourceSignatureId": evidence.source_signature_id,
                    "signedPayloadDigest": evidence.signed_payload_digest,
                    "signedPayloadDigestAlgorithm": evidence.signed_payload_digest_algorithm,
                    "signingIntent": evidence.signing_intent,
                    "signingActId": verbatim_act_id,
                    "sourceResponseRef": format!("urn:wos:artifact:{}:{}", claims.response_id, affirmation_idx),
                    "primitiveVerification": {
                        "status": "verified"
                    },
                    "witnessedSignatureRef": act.witnessed_ref
                }
            });
            if let Some(lc) = matching_lifecycle {
                if let Some(token) = lc.get("tsaAnchorToken") {
                    event_json["tsaAnchorToken"] = token.clone();
                }
                if let Some(seq) = lc.get("sequence") {
                    event_json["sequence"] = seq.clone();
                }
            }
            append_json_event(sig_event_literal, &event_json)?;
            affirmation_idx += 1;
        }
    }

    // 3. Append terminal `wos.envelope.sealed` / `wos.envelope.voided` lifecycle events.
    for evt in lifecycle_events {
        let event_type = evt["eventType"].as_str().unwrap_or_default();
        if event_type == "wos.envelope.sealed" || event_type == "wos.envelope.voided" {
            let mut enriched = evt.clone();
            enriched["data"]["voided"] = fixture["kernel"]["sealRule"]["voided"].clone();
            enriched["data"]["voidReason"] = fixture["kernel"]["sealRule"]["voidReason"].clone();
            enriched["data"]["requireSealedCompletion"] =
                fixture["kernel"]["sealRule"]["requireSealedCompletion"].clone();
            append_json_event(event_type, &enriched)?;
        }
    }

    let events = store.events;
    let checkpoints = issue_checkpoints_for_events(scope, &events, &cose_key_bytes, &timestamps);
    let generated_at = *timestamps.last().expect("at least one timestamp");

    let event_refs = events.iter().collect::<Vec<_>>();
    // 062-signature-affirmations.cbor plus 060-payloads/* for all events in 010-events.cbor.
    let mut evidence_members = WosExportProfile
        .evidence_members(scope, &event_refs)
        .map_err(|err| {
            EnvelopeVerificationError::TrellisBundleVerification(format!("evidence_members: {err}"))
        })?;
    let mut existing_payload_hashes = evidence_members
        .iter()
        .filter_map(|m| match m {
            trellis_export_writer::ExportEvidenceMember::Payload { content_hash, .. } => {
                Some(*content_hash)
            }
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    for event in &events {
        let canonical = decode_cbor_value(event.canonical_event()).map_err(|err| {
            EnvelopeVerificationError::TrellisBundleVerification(format!(
                "decode canonical event: {err}"
            ))
        })?;
        let map = canonical.as_map().unwrap();
        let payload_ref = map
            .iter()
            .find(|(k, _)| k.as_text() == Some("payload_ref"))
            .and_then(|(_, v)| v.as_map())
            .unwrap();
        let bytes = payload_ref
            .iter()
            .find(|(k, _)| k.as_text() == Some("ciphertext"))
            .and_then(|(_, v)| v.as_bytes())
            .unwrap()
            .clone();
        let content_hash = domain_separated_sha256(CONTENT_DOMAIN, &bytes);
        if existing_payload_hashes.insert(content_hash) {
            evidence_members.push(trellis_export_writer::ExportEvidenceMember::Payload {
                content_hash,
                bytes,
            });
        }
    }

    // Build 066-signed-acts.cbor, 067-policy-closure.cbor, and 068-signed-acts-manifest.cbor.
    // When two documents share one signingActId (`conflicting_multi_document_same_act_id`),
    // `WosExportProfile.profile_members` fails with `act_correlation_conflict`; we still emit
    // a syntactically valid 066 root catalog and 068 manifest so `verify_export_zip` runs on
    // the exported archive and surfaces the `act_correlation_conflict` finding in `wos_findings`
    // while `062-signature-affirmations.cbor` retains both per-document records.
    let profile_members = match WosExportProfile.profile_members(scope, &events, generated_at) {
        Ok(members) => members,
        Err(_) => {
            let empty_066 = encode_canonical_cbor_value(&cbor_map(vec![
                ("projection_schema_version", CborValue::Integer(1.into())),
                (
                    "derivation_rule_id",
                    CborValue::Text("signed-act-projection-wos-formspec-v1".to_string()),
                ),
                ("acts", CborValue::Array(Vec::new())),
            ]))
            .map_err(|err| {
                EnvelopeVerificationError::TrellisBundleVerification(format!(
                    "encode fallback 066: {err}"
                ))
            })?;
            let sig_literal = wos_signature_affirmation_event_type();
            let fail_literal = wos_signature_admission_failed_event_type();
            let mut tuples = Vec::new();
            for event in &events {
                let canonical = decode_cbor_value(event.canonical_event()).unwrap();
                let header = canonical
                    .as_map()
                    .unwrap()
                    .iter()
                    .find(|(k, _)| k.as_text() == Some("header"))
                    .and_then(|(_, v)| v.as_map())
                    .unwrap();
                let event_type_bytes = header
                    .iter()
                    .find(|(k, _)| k.as_text() == Some("event_type"))
                    .and_then(|(_, v)| v.as_bytes())
                    .unwrap();
                let event_type = String::from_utf8_lossy(event_type_bytes).to_string();
                if event_type == sig_literal || event_type == fail_literal {
                    tuples.push((
                        event.canonical_event_hash().unwrap().to_vec(),
                        event_type,
                    ));
                }
            }
            tuples.sort();
            let manifest_bytes = encode_signed_acts_manifest_v1(&tuples).map_err(|err| {
                EnvelopeVerificationError::TrellisBundleVerification(format!(
                    "encode 068 manifest: {err}"
                ))
            })?;
            trellis_server_ports::ExportProfileMembers {
                signed_acts_catalog: Some(SignedActsCatalogMember {
                    bytes: empty_066,
                    derivation_rule: "signed-act-projection-wos-formspec-v1".to_string(),
                }),
                signed_acts_manifest: Some(SignedActsManifestMember {
                    bytes: manifest_bytes,
                    derivation_rule: "signed-acts-manifest-v1".to_string(),
                }),
                policy_closure: None,
            }
        }
    };

    let registered_event_types = [
        "wos.case.created",
        "wos.envelope.documents_declared",
        "wos.agent.proposal.recorded",
        "wos.recipient.authenticated",
        "wos.document.view_recorded",
        sig_event_literal,
        "wos.envelope.sealed",
        "wos.envelope.voided",
    ];
    let registry_bytes = encode_canonical_cbor_value(&cbor_map(vec![
        (
            "event_types",
            CborValue::Map(
                registered_event_types
                    .into_iter()
                    .map(|et| {
                        (
                            CborValue::Text(et.to_string()),
                            cbor_map(vec![
                                (
                                    "privacy_class",
                                    CborValue::Text("publicMetadata".to_string()),
                                ),
                                (
                                    "binding_family",
                                    CborValue::Text("wos.signature".to_string()),
                                ),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
        (
            "classifications",
            CborValue::Array(vec![
                CborValue::Text("x-trellis-service/public-metadata".to_string()),
                CborValue::Text("x-trellis-service/private-attachment".to_string()),
            ]),
        ),
        (
            "registry_version",
            CborValue::Text("trellis-events:2026-05-15".to_string()),
        ),
    ]))
    .map_err(|err| {
        EnvelopeVerificationError::TrellisBundleVerification(format!("encode registry: {err}"))
    })?;

    let export_signing_key = ExportSigningKey {
        private_seed: parsed_key.private_seed,
        public_key: parsed_key.public_key,
        valid_from: TrellisTimestamp::new(1_745_000_000, 0).expect("timestamp"),
        valid_to: None,
    };

    let high_water_hash = events
        .last()
        .and_then(StoredEvent::canonical_event_hash)
        .copied()
        .ok_or_else(|| {
            EnvelopeVerificationError::TrellisBundleVerification(
                "missing canonical_event_hash".into(),
            )
        })?;
    let seal_version = events.len() as u64;
    let high_water_seq = seal_version - 1;
    let attempt_preimage = encode_canonical_cbor_value(&cbor_map(vec![
        ("bundle_scope", CborValue::Bytes(scope.to_vec())),
        ("seal_version", CborValue::Integer(seal_version.into())),
        (
            "high_water_sequence",
            CborValue::Integer(high_water_seq.into()),
        ),
        (
            "high_water_event_hash",
            CborValue::Bytes(high_water_hash.to_vec()),
        ),
    ]))
    .map_err(|err| {
        EnvelopeVerificationError::TrellisBundleVerification(format!("attempt preimage: {err}"))
    })?;
    let attempt_digest = domain_separated_sha256("trellis-export-attempt-v1", &attempt_preimage);
    let attempt_hex = attempt_digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    let package = write_export(ExportWriterInput {
        scope: scope.to_vec(),
        events,
        checkpoints,
        registries: vec![RegistrySnapshot {
            bytes: registry_bytes,
            registry_format: 1,
            registry_version: "trellis-events:2026-05-15".to_string(),
            bound_at_sequence: 0,
        }],
        signing_key: export_signing_key.clone(),
        signing_key_registry: vec![export_signing_key.registry_entry()],
        generator: "wos-formspec-binding/envelope-conformance".to_string(),
        generated_at,
        posture_declaration: PostureDeclaration {
            provider_readable: true,
            reader_held: false,
            delegated_compute: false,
            external_anchor_required: false,
            external_anchor_name: None,
            recovery_without_user: true,
            metadata_leakage_summary:
                "Envelope conformance export: reveals event_type, authored_at, retention_tier, classification, ledger_scope, and COSE kid."
                    .to_string(),
        },
        omitted_payload_checks: Vec::new(),
        readme_title: "Trellis Envelope Profile Conformance Export".to_string(),
        root_dir_override: None,
        external_anchors: Vec::new(),
        extensions: None,
        seal_fence: Some(ExportSealFence {
            bundle_scope: scope.to_vec(),
            export_attempt_id: format!("sha256:{attempt_hex}"),
            seal_version,
            event_count: seal_version,
            high_water_sequence: high_water_seq,
        }),
        witness_key_registry: None,
        signed_acts_catalog: profile_members.signed_acts_catalog,
        signed_acts_manifest: profile_members.signed_acts_manifest,
        policy_closure: profile_members.policy_closure,
        evidence_members,
    })
    .map_err(|err| {
        EnvelopeVerificationError::TrellisBundleVerification(format!("write_export: {err}"))
    })?;

    Ok(package.zip_bytes)
}

/// Extracts STORED ZIP members from `zip_bytes`, keyed by relative path inside the bundle root dir.
fn extract_stored_zip_members(zip_bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, String> {
    const LOCAL_HEADER_SIG: u32 = 0x0403_4b50;
    const CENTRAL_HEADER_SIG: u32 = 0x0201_4b50;

    let mut offset = 0usize;
    let mut members = BTreeMap::new();
    while offset + 4 <= zip_bytes.len() {
        let sig = u32::from_le_bytes(zip_bytes[offset..offset + 4].try_into().unwrap());
        if sig == CENTRAL_HEADER_SIG {
            break;
        }
        if sig != LOCAL_HEADER_SIG {
            return Err(format!("unexpected ZIP header signature 0x{sig:08x} at {offset}"));
        }
        if offset + 30 > zip_bytes.len() {
            return Err("truncated ZIP local file header".into());
        }
        let compression = u16::from_le_bytes(zip_bytes[offset + 8..offset + 10].try_into().unwrap());
        if compression != 0 {
            return Err(format!("unsupported ZIP compression method {compression}"));
        }
        let compressed_size =
            u32::from_le_bytes(zip_bytes[offset + 18..offset + 22].try_into().unwrap()) as usize;
        let name_len =
            u16::from_le_bytes(zip_bytes[offset + 26..offset + 28].try_into().unwrap()) as usize;
        let extra_len =
            u16::from_le_bytes(zip_bytes[offset + 28..offset + 30].try_into().unwrap()) as usize;
        let name_start = offset + 30;
        let data_start = name_start + name_len + extra_len;
        let data_end = data_start + compressed_size;
        if data_end > zip_bytes.len() {
            return Err("truncated ZIP entry payload".into());
        }
        let full_path = std::str::from_utf8(&zip_bytes[name_start..name_start + name_len])
            .map_err(|err| format!("non-utf8 ZIP entry path: {err}"))?;
        let rel_path = full_path
            .split_once('/')
            .map_or(full_path, |(_, rest)| rest)
            .to_string();
        members.insert(rel_path, zip_bytes[data_start..data_end].to_vec());
        offset = data_end;
    }
    Ok(members)
}

fn cbor_value_to_json(value: &CborValue) -> JsonValue {
    match value {
        CborValue::Null => JsonValue::Null,
        CborValue::Bool(b) => JsonValue::Bool(*b),
        CborValue::Integer(int) => {
            let val: i128 = (*int).into();
            if let Ok(u) = u64::try_from(val) {
                JsonValue::Number(u.into())
            } else if let Ok(i) = i64::try_from(val) {
                JsonValue::Number(i.into())
            } else {
                JsonValue::Null
            }
        }
        CborValue::Float(_) => JsonValue::Null,
        CborValue::Bytes(bytes) => {
            let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
            JsonValue::String(hex)
        }
        CborValue::Text(s) => JsonValue::String(s.clone()),
        CborValue::Array(arr) => JsonValue::Array(arr.iter().map(cbor_value_to_json).collect()),
        CborValue::Map(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                if let Some(key_str) = k.as_text() {
                    obj.insert(key_str.to_string(), cbor_value_to_json(v));
                }
            }
            JsonValue::Object(obj)
        }
        CborValue::Tag(_, inner) => cbor_value_to_json(inner),
        _ => JsonValue::Null,
    }
}

fn decode_cbor_to_json(bytes: &[u8]) -> Result<JsonValue, String> {
    let cbor = decode_cbor_value(bytes).map_err(|err| format!("decode_cbor_value: {err}"))?;
    Ok(cbor_value_to_json(&cbor))
}

/// Decodes `010-events.cbor` into the list of inline event JSON payloads.
fn decode_events_from_010_cbor(events_cbor_bytes: &[u8]) -> Result<Vec<JsonValue>, String> {
    let root = decode_cbor_value(events_cbor_bytes)
        .map_err(|err| format!("decode 010-events.cbor: {err}"))?;
    let cose_events = root
        .as_array()
        .ok_or_else(|| "010-events.cbor root is not an array".to_string())?;
    let mut decoded = Vec::with_capacity(cose_events.len());
    for cose_item in cose_events {
        let unwrapped = match cose_item {
            CborValue::Tag(_, inner) => inner.as_ref(),
            other => other,
        };
        let cose_arr = unwrapped
            .as_array()
            .ok_or_else(|| "010-events.cbor item is not COSE_Sign1 array".to_string())?;
        let canonical_bytes = cose_arr
            .get(2)
            .and_then(CborValue::as_bytes)
            .ok_or_else(|| "COSE_Sign1 payload is not bstr".to_string())?;
        let canonical = decode_cbor_value(canonical_bytes)
            .map_err(|err| format!("decode canonical_event: {err}"))?;
        let map = canonical
            .as_map()
            .ok_or_else(|| "canonical_event is not a map".to_string())?;
        let payload_ref = map
            .iter()
            .find(|(k, _)| k.as_text() == Some("payload_ref"))
            .and_then(|(_, v)| v.as_map())
            .ok_or_else(|| "missing payload_ref".to_string())?;
        let ciphertext = payload_ref
            .iter()
            .find(|(k, _)| k.as_text() == Some("ciphertext"))
            .and_then(|(_, v)| v.as_bytes())
            .ok_or_else(|| "missing ciphertext".to_string())?;
        let payload_json = decode_cbor_to_json(ciphertext)?;
        decoded.push(payload_json);
    }
    Ok(decoded)
}

/// Decodes `062-signature-affirmations.cbor` into its JSON row representations.
fn decode_062_affirmation_rows(bytes: &[u8]) -> Result<Vec<JsonValue>, String> {
    let json_val = decode_cbor_to_json(bytes)?;
    json_val
        .as_array()
        .cloned()
        .ok_or_else(|| "062-signature-affirmations.cbor is not an array".to_string())
}

/// Synthesizes equivalent 062-shaped rows for `wos-server-only` (`intranet`) verification.
fn synthesize_server_only_affirmation_rows(
    verified_acts: &[VerifiedFormspecAct],
) -> Vec<JsonValue> {
    let mut rows = Vec::new();
    for act in verified_acts {
        for evidence in &act.evidences {
            let claims = evidence
                .document_claims
                .as_ref()
                .expect("verified evidence has document_claims");
            rows.push(json!({
                "signing_act_id": evidence.signing_act_id.as_deref().unwrap_or(&act.signing_act_id),
                "signer_id": evidence.signer_id,
                "role_id": act.role_id,
                "role": act.role,
                "document_id": evidence.document_id,
                "document_hash": format!("sha256:{}", evidence.document_hash),
                "presentation_hash": claims.presentation_hash,
                "signed_at": evidence.signed_at,
                "witnessed_signature_ref": act.witnessed_ref,
            }));
        }
    }
    rows
}

#[test]
fn all_seven_named_envelope_profiles_satisfy_conformance_and_facade_questions() {
    for profile in ALL_PROFILES {
        let fixture = load_profile_fixture(profile);
        let facade = EnvelopeVerifierFacade::verify_envelope_fixture(&fixture)
            .unwrap_or_else(|err| panic!("profile `{profile}` failed verification: {err}"));

        assert_eq!(facade.profile(), profile);

        // Question 1: Who signed?
        let expected_who = fixture["expectedVerification"]["whoSigned"]
            .as_array()
            .unwrap();
        assert_eq!(
            facade.who_signed().len(),
            expected_who.len(),
            "whoSigned count mismatch for {profile}"
        );
        for (actual, expected) in facade.who_signed().iter().zip(expected_who) {
            assert_eq!(actual.signing_act_id, expected["signingActId"]);
            assert_eq!(actual.signer_id, expected["signerId"]);
            assert_eq!(actual.role_id, expected["roleId"]);
            assert_eq!(
                actual.witnessed_signature_ref.as_deref(),
                expected["witnessedSignatureRef"].as_str()
            );
        }

        // Question 2: What was signed?
        let expected_what = fixture["expectedVerification"]["whatWasSigned"]
            .as_array()
            .unwrap();
        assert_eq!(
            facade.what_was_signed().len(),
            expected_what.len(),
            "whatWasSigned count mismatch for {profile}"
        );
        for (actual, expected) in facade.what_was_signed().iter().zip(expected_what) {
            assert_eq!(actual.document_id, expected["documentId"]);
            assert_eq!(actual.sha256, expected["sha256"]);
            assert_eq!(actual.presentation_hash, expected["presentationHash"]);
        }

        // Question 3: When signed?
        let expected_when = fixture["expectedVerification"]["whenSigned"]
            .as_array()
            .unwrap();
        assert_eq!(
            facade.when_signed().len(),
            expected_when.len(),
            "whenSigned count mismatch for {profile}"
        );
        for (actual, expected) in facade.when_signed().iter().zip(expected_when) {
            assert_eq!(actual.signing_act_id, expected["signingActId"]);
            assert_eq!(actual.signed_at, expected["signedAt"]);
            assert_eq!(actual.timer_anchoring_mode, expected["timerAnchoringMode"]);
        }

        // Question 4: View evidence?
        let expected_views = fixture["expectedVerification"]["viewEvidence"]
            .as_array()
            .unwrap();
        assert_eq!(facade.all_view_evidence().len(), expected_views.len());
        for expected_view in expected_views {
            let recipient_id = expected_view["recipientId"].as_str().unwrap();
            let doc_id = expected_view["documentId"].as_str().unwrap();
            let actual_view = facade
                .view_evidence_for(recipient_id, doc_id)
                .unwrap_or_else(|| {
                    panic!("missing view_evidence_for({recipient_id}, {doc_id}) in {profile}")
                });
            assert_eq!(
                actual_view.scroll_completed,
                expected_view["scrollCompleted"].as_bool().unwrap()
            );
        }

        // Question 5: Void status?
        let expected_void = &fixture["expectedVerification"]["voidStatus"];
        assert_eq!(
            facade.void_status().is_voided,
            expected_void["isVoided"].as_bool().unwrap()
        );
        assert_eq!(
            facade.void_status().is_sealed,
            expected_void["isSealed"].as_bool().unwrap()
        );

        // Question 6: Proofing levels?
        let expected_proofing = fixture["expectedVerification"]["proofingLevels"]
            .as_object()
            .unwrap();
        for (recipient_id, expected_level) in expected_proofing {
            let actual_level = facade
                .proofing_level_for(recipient_id)
                .unwrap_or_else(|| panic!("missing proofing_level_for({recipient_id}) in {profile}"));
            assert_eq!(actual_level.tier, expected_level["tier"]);
            assert_eq!(
                actual_level.assurance_level,
                expected_level["assuranceLevel"]
            );
        }

        // Structural inspector & P-1_documentBytesStore offline bundle posture check.
        let offline_supported = fixture["expectedVerification"]["exportPosture"]
            ["offlineBundleSupported"]
            .as_bool()
            .unwrap();
        if offline_supported {
            let report = facade
                .require_offline_bundle()
                .expect("offline bundle must be present");
            assert!(report.trellis.structure_verified);
            assert!(report.trellis.integrity_verified);
            assert!(report.trellis.readability_verified);
            assert!(report.wos_findings.is_empty());
            assert!(facade.structural_inspector().is_some());
        } else {
            assert!(facade.structural_inspector().is_none());
            assert!(matches!(
                facade.require_offline_bundle(),
                Err(EnvelopeVerificationError::ServerOnlyPostureNoOfflineBundle(_))
            ));
        }
    }
}

#[test]
fn conflicting_multi_document_same_act_id_preserves_per_document_evidence_and_finding() {
    // Per signature-ports.md §§3.3, 3.6 (lines 289, 293): when one signing gesture emits two
    // distinct document affirmations sharing the same `signingActId`, `062-signature-affirmations.cbor`
    // retains both per-document affirmation records while `verify_export_zip` surfaces the
    // `act_correlation_conflict` finding in `report.wos_findings`.
    let mut fixture = load_profile_fixture("basic-esign");
    fixture["definition"]["items"][0]["attests"] = json!([
        "master-service-agreement",
        "esign-consent-disclosure"
    ]);
    fixture["kernel"]["signingActs"] = json!([
        {
            "signingActId": "01JM8IC0000000000000000999",
            "fieldPath": "signature",
            "signerId": "applicant",
            "roleId": "applicant",
            "role": "Applicant",
            "signedAt": "2026-10-20T19:01:00Z",
            "signingIntent": "urn:wos:signing-intent:applicant-signature",
            "ceremonyId": "ceremony-basic-esign-001",
            "consentReference": "urn:example:consent:esign:v1",
            "attestedDocumentIds": [
                "master-service-agreement",
                "esign-consent-disclosure"
            ],
            "witnessedSignatureRef": null
        }
    ]);

    let verified_acts = verify_formspec_signing_acts(&fixture)
        .expect("Formspec multi-document response with shared signingActId is valid");
    assert_eq!(verified_acts.len(), 1);
    assert_eq!(verified_acts[0].evidences.len(), 2);

    let zip_bytes = build_trellis_export_zip(&fixture, &verified_acts)
        .expect("export ZIP builds with verbatim shared signingActId");
    let report = verify_export_zip(&zip_bytes);

    assert!(report.trellis.structure_verified);
    assert!(report.trellis.integrity_verified);
    assert!(report.trellis.readability_verified);
    assert!(
        report
            .wos_findings
            .iter()
            .any(|f| f.message.contains("act_correlation_conflict")),
        "expected act_correlation_conflict in wos_findings, got: {:?}",
        report.wos_findings
    );

    // Verify that 062-signature-affirmations.cbor inside the exported ZIP retains BOTH
    // per-document affirmation records with the verbatim shared signingActId.
    let members = extract_stored_zip_members(&zip_bytes).unwrap();
    let rows = decode_062_affirmation_rows(
        members
            .get("062-signature-affirmations.cbor")
            .expect("062-signature-affirmations.cbor present"),
    )
    .unwrap();
    assert_eq!(rows.len(), 2, "062 must retain both per-document records");
    assert_eq!(rows[0]["signing_act_id"], "01JM8IC0000000000000000999");
    assert_eq!(rows[1]["signing_act_id"], "01JM8IC0000000000000000999");
    assert_eq!(rows[0]["document_id"], "master-service-agreement");
    assert_eq!(rows[1]["document_id"], "esign-consent-disclosure");

    // And the facade refuses to fabricate a positive answer when `act_correlation_conflict` fires.
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&fixture).unwrap_err();
    assert!(
        matches!(
            &err,
            EnvelopeVerificationError::TrellisBundleVerification(msg)
                if msg.contains("act_correlation_conflict")
        ),
        "expected TrellisBundleVerification containing act_correlation_conflict, got {err:?}"
    );
}

#[test]
fn negative_control_tampered_document_digest_or_presentation_hash_is_refused() {
    let mut fixture = load_profile_fixture("basic-esign");
    // Corrupt the declared presentationHash in viewEvidence so it disagrees with the signed document.
    fixture["lifecycleEvents"][3]["data"]["presentationHash"] =
        json!("0000000000000000000000000000000000000000000000000000000000000000");
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&fixture).unwrap_err();
    assert!(
        matches!(err, EnvelopeVerificationError::DocumentDigestMismatch(_)),
        "expected DocumentDigestMismatch, got {err:?}"
    );

    // Corrupt kernel.documents[0].sha256 so it disagrees with the native Formspec Definition.
    let mut fixture_bad_sha = load_profile_fixture("basic-esign");
    fixture_bad_sha["kernel"]["documents"][0]["sha256"] =
        json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&fixture_bad_sha).unwrap_err();
    assert!(
        matches!(err, EnvelopeVerificationError::FormspecValidation(_)),
        "expected FormspecValidation refusal on mismatched Definition/Response documentHash, got {err:?}"
    );
}

#[test]
fn negative_control_missing_view_evidence_or_incomplete_scroll_or_dwell_is_refused() {
    // 1. Missing viewEvidence for an attested document under basic-esign (P-2_viewEvidenceCollector).
    let mut missing_view = load_profile_fixture("basic-esign");
    missing_view["lifecycleEvents"]
        .as_array_mut()
        .unwrap()
        .retain(|evt| {
            !(evt["eventType"] == "wos.document.view_recorded"
                && evt["data"]["documentId"] == "esign-consent-disclosure")
        });
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&missing_view).unwrap_err();
    assert!(
        matches!(err, EnvelopeVerificationError::ViewEvidenceViolation(_)),
        "expected ViewEvidenceViolation for missing view_recorded event, got {err:?}"
    );

    // 2. Incomplete scroll under single-locale (P-2_viewEvidenceCollector.requireScrollCompletion = true).
    let mut incomplete_scroll = load_profile_fixture("single-locale");
    incomplete_scroll["lifecycleEvents"][3]["data"]["scrollCompleted"] = json!(false);
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&incomplete_scroll).unwrap_err();
    assert!(
        matches!(err, EnvelopeVerificationError::ViewEvidenceViolation(_)),
        "expected ViewEvidenceViolation for scrollCompleted=false, got {err:?}"
    );

    // 3. Insufficient page dwell under compliance-strict (P-2_viewEvidenceCollector.minPageDwellMs = 2000).
    let mut short_dwell = load_profile_fixture("compliance-strict");
    short_dwell["lifecycleEvents"][3]["data"]["minPageDwellMsObserved"] = json!(500);
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&short_dwell).unwrap_err();
    assert!(
        matches!(err, EnvelopeVerificationError::ViewEvidenceViolation(_)),
        "expected ViewEvidenceViolation for insufficient dwell time, got {err:?}"
    );
}

#[test]
fn negative_control_ron_and_ipen_missing_notary_commission_or_av_session_is_refused() {
    // 1. RON missing avSessionId on applicant authentication (P-8_recipientIdentityProofing.avRecordingRequired = true).
    let mut ron_no_av = load_profile_fixture("ron");
    ron_no_av["lifecycleEvents"][2]["data"]
        .as_object_mut()
        .unwrap()
        .remove("avSessionId");
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&ron_no_av).unwrap_err();
    assert!(
        matches!(err, EnvelopeVerificationError::IdentityProofingViolation(_)),
        "expected IdentityProofingViolation for missing RON avSessionId, got {err:?}"
    );

    // 2. RON missing notary countersignature act (K-5 / P-8_recipientIdentityProofing.notaryCommissionRequired = true).
    let mut ron_no_notary = load_profile_fixture("ron");
    ron_no_notary["kernel"]["signingActs"]
        .as_array_mut()
        .unwrap()
        .retain(|act| act["witnessedSignatureRef"].is_null());
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&ron_no_notary).unwrap_err();
    assert!(
        matches!(err, EnvelopeVerificationError::WitnessNotaryViolation(_)),
        "expected WitnessNotaryViolation when RON notary act is omitted, got {err:?}"
    );

    // 3. IPEN with physicalPresenceVerified = false (K-6 / P-8_recipientIdentityProofing.physicalPresenceAttested = true).
    let mut ipen_no_presence = load_profile_fixture("in-person-notary");
    ipen_no_presence["kernel"]["signingActs"][2]["notaryCommission"]["physicalPresenceVerified"] =
        json!(false);
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&ipen_no_presence).unwrap_err();
    assert!(
        matches!(err, EnvelopeVerificationError::WitnessNotaryViolation(_)),
        "expected WitnessNotaryViolation when IPEN physicalPresenceVerified=false, got {err:?}"
    );
}

#[test]
fn negative_control_compliance_strict_missing_per_event_tsa_anchor_is_refused() {
    let mut missing_anchor = load_profile_fixture("compliance-strict");
    missing_anchor["lifecycleEvents"][3]
        .as_object_mut()
        .unwrap()
        .remove("tsaAnchorToken");
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&missing_anchor).unwrap_err();
    assert!(
        matches!(err, EnvelopeVerificationError::TimerAnchoringViolation(_)),
        "expected TimerAnchoringViolation for missing per-event TSA token under P-10_envelopeTimerPolicy, got {err:?}"
    );
}

#[test]
fn negative_control_audit_maxed_unratified_agent_proposal_is_refused() {
    let mut unratified = load_profile_fixture("audit-maxed");
    for act in unratified["kernel"]["signingActs"].as_array_mut().unwrap() {
        act.as_object_mut().unwrap().remove("ratifiedAgentProposalId");
    }
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&unratified).unwrap_err();
    assert!(
        matches!(err, EnvelopeVerificationError::UnratifiedAgentProposal(_)),
        "expected UnratifiedAgentProposal when principal act omits ratifiedAgentProposalId under P-6_intentRegistry, got {err:?}"
    );
}

#[test]
fn negative_control_intranet_server_only_posture_refuses_undisclosed_offline_claim() {
    let mut intranet_bad_posture = load_profile_fixture("intranet");
    // Claim offlineVerifiable=true while P-1_documentBytesStore is wos-server-only.
    intranet_bad_posture["kernel"]["offlineVerifiable"] = json!(true);
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&intranet_bad_posture).unwrap_err();
    assert!(
        matches!(
            err,
            EnvelopeVerificationError::ServerOnlyPostureNoOfflineBundle(_)
        ),
        "expected ServerOnlyPostureNoOfflineBundle on contradictory P-1_documentBytesStore posture, got {err:?}"
    );
}

#[test]
fn negative_control_voided_envelope_surfaces_void_status_and_refuses_sealed_claim() {
    let mut voided = load_profile_fixture("basic-esign");
    voided["kernel"]["sealRule"]["voided"] = json!(true);
    voided["kernel"]["sealRule"]["voidReason"] = json!("Recipient declined terms");

    // Without requireSealedCompletion, facade succeeds and surfaces void_status().is_voided == true.
    let facade = EnvelopeVerifierFacade::verify_envelope_fixture(&voided).unwrap();
    assert!(facade.void_status().is_voided);
    assert!(!facade.void_status().is_sealed);
    assert_eq!(
        facade.void_status().reason.as_deref(),
        Some("Recipient declined terms")
    );

    // When sealed completion is required by the caller (K-7), verification refuses with EnvelopeVoided.
    voided["kernel"]["sealRule"]["requireSealedCompletion"] = json!(true);
    let err = EnvelopeVerifierFacade::verify_envelope_fixture(&voided).unwrap_err();
    assert!(
        matches!(err, EnvelopeVerificationError::EnvelopeVoided(_)),
        "expected EnvelopeVoided when requireSealedCompletion=true on voided envelope, got {err:?}"
    );
}
