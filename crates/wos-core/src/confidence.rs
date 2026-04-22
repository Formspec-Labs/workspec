// Rust guideline compliant 2026-04-11

//! Confidence framework evaluation (AI S7).
//!
//! Validates confidence reports, checks calibration, applies decay,
//! monitors cumulative confidence, and records ground-truth labels.

use crate::model::ai::AIIntegrationDocument;
use crate::provenance::{ProvenanceKind, ProvenanceRecord};

/// Result of confidence evaluation.
#[derive(Debug, Clone)]
pub struct ConfidenceResult {
    pub provenance: Vec<ProvenanceRecord>,
    /// Whether the confidence check requires escalation to human.
    pub requires_escalation: bool,
}

/// Evaluate confidence constraints for an agent event.
pub fn evaluate_confidence(
    ai_doc: &AIIntegrationDocument,
    agent_id: &str,
    data: &serde_json::Value,
) -> ConfidenceResult {
    let mut provenance = Vec::new();
    let mut requires_escalation = false;
    let _agent = ai_doc.agents.iter().find(|a| a.id == agent_id);

    let confidence_report = data.get("confidenceReport");

    // ── AI-034: Confidence report required ──────────────────────────
    if data.get("output").is_some() && confidence_report.is_none() {
        provenance.push(prov(
            ProvenanceKind::ConfidenceViolation,
            serde_json::json!({ "reason": "missing-confidence-report" }),
        ));
        return ConfidenceResult {
            provenance,
            requires_escalation: false,
        };
    }

    if let Some(report) = confidence_report {
        let overall = report
            .get("overall")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let calibrated = report.get("calibrated").and_then(|v| v.as_bool());

        // ── AI-035: Uncalibrated model-native scores ────────────────
        if calibrated == Some(false) && report.get("modelNative").is_some() {
            provenance.push(prov(
                ProvenanceKind::ConfidenceViolation,
                serde_json::json!({ "reason": "uncalibrated-model-native" }),
            ));
        }

        // ── AI-037: Decay trigger ───────────────────────────────────
        let mut effective_confidence = overall;
        if let Some(decay_event) = data.get("decayEvent") {
            let factor = decay_event
                .get("decayFactor")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            let decayed = overall * factor;
            // Round to 2 decimal places for provenance
            let decayed_rounded = (decayed * 100.0).round() / 100.0;
            provenance.push(prov(
                ProvenanceKind::ConfidenceDecay,
                serde_json::json!({
                    "original": overall,
                    "factor": factor,
                    "decayed": decayed_rounded,
                }),
            ));
            effective_confidence = decayed_rounded;
        }

        // ── AI-036: Confidence below floor ──────────────────────────
        if let Some(ref floor) = ai_doc.confidence_floor {
            if effective_confidence < floor.threshold {
                requires_escalation = true;
                provenance.push(prov(
                    ProvenanceKind::ConfidenceViolation,
                    serde_json::json!({
                        "threshold": floor.threshold,
                        "actual": effective_confidence,
                        "action": "escalateToHuman",
                    }),
                ));
            }
        }
    }

    // ── AI-038: Cumulative confidence ───────────────────────────────
    if let Some(session) = data.get("sessionState") {
        let cumulative = session.get("cumulativeConfidence").and_then(|v| v.as_f64());
        let is_checkpoint = session
            .get("isCheckpoint")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if let Some(cum_val) = cumulative {
            if let Some(ref floor) = ai_doc.confidence_floor {
                if cum_val < floor.threshold {
                    requires_escalation = true;
                    // AG-004: Session pause at checkpoint
                    if is_checkpoint {
                        provenance.push(prov(
                            ProvenanceKind::SessionPaused,
                            serde_json::json!({
                                "reason": "cumulative-confidence-below-floor",
                                "checkpoint": true,
                            }),
                        ));
                    } else {
                        provenance.push(prov(
                            ProvenanceKind::CumulativeConfidenceViolation,
                            serde_json::json!({
                                "cumulative": cum_val,
                                "threshold": floor.threshold,
                                "action": "pause-for-review",
                            }),
                        ));
                    }
                }
            }
        }
    }

    ConfidenceResult {
        provenance,
        requires_escalation,
    }
}

/// Evaluate review events for ground-truth labels (AG-016).
pub fn evaluate_review_ground_truth(
    data: &serde_json::Value,
    actor: &str,
) -> Vec<ProvenanceRecord> {
    let mut provenance = Vec::new();
    if let Some(label) = data.get("groundTruthLabel").and_then(|v| v.as_str()) {
        provenance.push(prov(
            ProvenanceKind::GroundTruthLabel,
            serde_json::json!({
                "label": label,
                "reviewer": actor,
            }),
        ));
    }
    provenance
}

fn prov(kind: ProvenanceKind, data: serde_json::Value) -> ProvenanceRecord {
    ProvenanceRecord {
        id: ProvenanceRecord::mint_id(),
        record_kind: kind,
        timestamp: String::new(),
        actor_id: None,
        from_state: None,
        to_state: None,
        event: None,
        data: Some(data),
        audit_layer: None,
        actor_type: None,
        lifecycle_state: None,
        definition_version: None,
        inputs: Vec::new(),
        outputs: Vec::new(),
        input_digest: None,
        output_digest: None,
        canonical_event_hash: None,
        transition_tags: Vec::new(),
        case_file_snapshot: None,
        outcome: None,
    }
}
