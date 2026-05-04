// Rust guideline compliant 2026-05-02

//! Per-marker document envelopes — permissive shape.
//!
//! Each document carries one of the 14 `$wosStudio*` marker keys plus a
//! free-form body of remaining fields. The marker key serves as the
//! `#[serde(untagged)]` discriminator on [`StudioDocument`]; everything
//! else is captured in `body: IndexMap<String, Value>` for downstream
//! tightening.
//!
//! This is intentionally a wide front door:
//! - The schemas themselves enforce per-kind shapes (Stage-3 work).
//! - Stage-4 readiness lint (Wave 1.3) tightens shape against business
//!   rules.
//! - Stage-5 compiler (Wave 2) needs typed access to specific fields per
//!   document kind; those promote into named struct fields here as
//!   compiler work demands them.
//!
//! Round-trip: `serde_json::from_value::<StudioDocument>(v)` followed by
//! `serde_json::to_value(parsed)` is bit-identical for any well-formed
//! Studio document (tested below).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Top-level untagged enum dispatching on `$wosStudio*` marker.
///
/// Use [`crate::classify`] for cheap discrimination first when you only
/// need to know the document's kind without paying full deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StudioDocument {
    Approval(ApprovalDocument),
    Binding(BindingDocument),
    Effectiveness(EffectivenessDocument),
    IdentitySubject(IdentitySubjectDocument),
    Mapping(MappingDocument),
    MigrationPath(MigrationPathDocument),
    PolicyObject(PolicyObjectDocument),
    Provenance(ProvenanceDocument),
    Readiness(ReadinessDocument),
    Scenario(ScenarioDocument),
    Source(SourceDocument),
    TerminologyMap(TerminologyMapDocument),
    WorkflowIntent(WorkflowIntentDocument),
    Workspace(WorkspaceDocument),
}

/// Generates a permissive per-marker document type. `marker_key` is the
/// `$wosStudio*` JSON key the document carries; the type name is appended
/// with `Document`.
macro_rules! studio_doc {
    ($name:ident, $marker_key:literal) => {
        #[doc = concat!("Document carrying the `", $marker_key, "` marker.")]
        ///
        /// Shape is intentionally permissive: only the marker is typed;
        /// every other field flows through [`body`](Self::body) as raw
        /// JSON. Downstream consumers tighten as needed.
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct $name {
            #[doc = concat!("The `", $marker_key, "` marker value (typically `\"1.0\"`).")]
            #[serde(rename = $marker_key)]
            pub marker: String,

            /// Every other top-level field, preserved in document order.
            #[serde(flatten)]
            pub body: IndexMap<String, Value>,
        }

        impl $name {
            pub const MARKER_KEY: &'static str = $marker_key;

            /// Read a typed body field, deserializing the underlying
            /// `serde_json::Value` into `T`. Returns `None` if the field
            /// is absent.
            pub fn field<T: serde::de::DeserializeOwned>(
                &self,
                key: &str,
            ) -> Option<Result<T, serde_json::Error>> {
                self.body
                    .get(key)
                    .map(|v| serde_json::from_value::<T>(v.clone()))
            }

            /// Borrow a body field as raw `&Value` without deserializing.
            pub fn raw(&self, key: &str) -> Option<&Value> {
                self.body.get(key)
            }
        }
    };
}

studio_doc!(ApprovalDocument, "$wosStudioApproval");
studio_doc!(BindingDocument, "$wosStudioBinding");
studio_doc!(EffectivenessDocument, "$wosStudioEffectiveness");
studio_doc!(IdentitySubjectDocument, "$wosStudioIdentitySubject");
studio_doc!(MappingDocument, "$wosStudioMapping");
studio_doc!(MigrationPathDocument, "$wosStudioMigrationPath");
studio_doc!(PolicyObjectDocument, "$wosStudioPolicyObject");
studio_doc!(ProvenanceDocument, "$wosStudioProvenance");
studio_doc!(ReadinessDocument, "$wosStudioReadiness");
studio_doc!(ScenarioDocument, "$wosStudioScenario");
studio_doc!(SourceDocument, "$wosStudioSource");
studio_doc!(TerminologyMapDocument, "$wosStudioTerminologyMap");
studio_doc!(WorkflowIntentDocument, "$wosStudioWorkflowIntent");
studio_doc!(WorkspaceDocument, "$wosStudioWorkspace");

impl StudioDocument {
    /// Borrow the body map of whatever variant this is. All studio
    /// documents share the same `(marker, body: IndexMap<String, Value>)`
    /// envelope; this is the dispatch helper that lets consumers read
    /// fields without matching on every variant. Migration handle for
    /// retiring `WorkspaceDocument.raw` access in the lint engine.
    pub fn body(&self) -> &IndexMap<String, Value> {
        match self {
            StudioDocument::Approval(d) => &d.body,
            StudioDocument::Binding(d) => &d.body,
            StudioDocument::Effectiveness(d) => &d.body,
            StudioDocument::IdentitySubject(d) => &d.body,
            StudioDocument::Mapping(d) => &d.body,
            StudioDocument::MigrationPath(d) => &d.body,
            StudioDocument::PolicyObject(d) => &d.body,
            StudioDocument::Provenance(d) => &d.body,
            StudioDocument::Readiness(d) => &d.body,
            StudioDocument::Scenario(d) => &d.body,
            StudioDocument::Source(d) => &d.body,
            StudioDocument::TerminologyMap(d) => &d.body,
            StudioDocument::WorkflowIntent(d) => &d.body,
            StudioDocument::Workspace(d) => &d.body,
        }
    }
}

// ============================================================================
// Typed accessors for the four heavy-consumer documents (R6 — type-driven
// discovery). Each accessor returns the spec-declared field with a typed
// signature so a schema rename produces compile errors at consumer call
// sites rather than silent runtime misses.
//
// The underlying body shape stays permissive (IndexMap<String, Value>),
// so these accessors compose with `field<T>()` / `raw()` for fields that
// the consumer hasn't promoted yet.
// ============================================================================

use crate::common::{
    MappingState, OriginClass, PolicyObjectLifecycleState,
    ScenarioLifecycleState, WorkflowIntentLifecycleState,
};

/// Typed accessor delegate that returns the parsed enum value of a body
/// field, or None if the field is absent / malformed.
fn typed_str_field<T: serde::de::DeserializeOwned>(
    body: &IndexMap<String, Value>,
    key: &str,
) -> Option<T> {
    body.get(key)
        .and_then(|v| serde_json::from_value::<T>(v.clone()).ok())
}

impl PolicyObjectDocument {
    /// `id` — always present on a well-formed PolicyObject.
    pub fn id(&self) -> Option<&str> {
        self.body.get("id").and_then(Value::as_str)
    }

    /// `workspaceId` — owning workspace.
    pub fn workspace_id(&self) -> Option<&str> {
        self.body.get("workspaceId").and_then(Value::as_str)
    }

    /// PolicyObject discriminator (`Outcome`, `NoticeRequirement`,
    /// `AppealRight`, etc.).
    pub fn kind(&self) -> Option<&str> {
        self.body.get("kind").and_then(Value::as_str)
    }

    pub fn lifecycle_state(&self) -> Option<PolicyObjectLifecycleState> {
        typed_str_field(&self.body, "lifecycleState")
    }

    pub fn origin_class(&self) -> Option<OriginClass> {
        typed_str_field(&self.body, "originClass")
    }

    pub fn mapping_state(&self) -> Option<MappingState> {
        typed_str_field(&self.body, "mappingState")
    }

    /// `triggersDueProcess` — load-bearing for WF-LINT-001 / SC-LINT-001.
    pub fn triggers_due_process(&self) -> Option<bool> {
        self.body.get("triggersDueProcess").and_then(Value::as_bool)
    }

    /// Citations array — borrows raw `Value` items; the spec doesn't
    /// fix a shape, so a typed `Citation` struct would be premature.
    pub fn citations(&self) -> Option<&Vec<Value>> {
        self.body.get("citations").and_then(Value::as_array)
    }

    /// `applicabilityScope` body — the `ApplicabilityScope` PolicyObject
    /// kind carries this as its primary payload. Returns the raw object
    /// so consumers can $ref against the kernel `$defs/ApplicabilityScope`
    /// shape (added F1.1).
    pub fn applicability_scope(&self) -> Option<&Value> {
        self.body.get("applicabilityScope")
    }

    /// `effectivePeriod` body — the `EffectivePeriod` PolicyObject
    /// kind carries this as its primary payload. Returns the raw
    /// object; kernel `$defs/EffectivePeriod` defines the shape
    /// (added F1.2).
    pub fn effective_period(&self) -> Option<&Value> {
        self.body.get("effectivePeriod")
    }

    /// `effectivenessRef` — IRI to an Effectiveness PolicyObject;
    /// load-bearing for EFF-LINT-* rules.
    pub fn effectiveness_ref(&self) -> Option<&str> {
        self.body.get("effectivenessRef").and_then(Value::as_str)
    }

    /// `canonicalTermRef` — DataElement vocabulary alignment
    /// (added F1.5; kernel FieldDeclaration mirrors this).
    pub fn canonical_term_ref(&self) -> Option<&str> {
        self.body.get("canonicalTermRef").and_then(Value::as_str)
    }

    /// `dpvSensitivity` — DataElement DPV classification (added F1.5;
    /// kernel FieldDeclaration mirrors this).
    pub fn dpv_sensitivity(&self) -> Option<&str> {
        self.body.get("dpvSensitivity").and_then(Value::as_str)
    }

    /// `extensionRecordRef` — when this PolicyObject's mapping carries
    /// an external ExtensionRecord, this is the IRI handle. Returns
    /// None for inline-extension-record mappings.
    pub fn extension_record_ref(&self) -> Option<&str> {
        self.body
            .get("extensionRecordRef")
            .and_then(Value::as_str)
    }

    /// `polarity` — Outcome PolicyObjects mark themselves `favorable`
    /// (the workflow concluded in the subject's favor) or `adverse`
    /// (against the subject; triggers due-process surfaces when paired
    /// with `triggersDueProcess: true`). Other PolicyObject kinds
    /// carry None.
    pub fn polarity(&self) -> Option<&str> {
        self.body.get("polarity").and_then(Value::as_str)
    }

    /// `linkedNoticeRequirementRef` — adverse Outcomes link to a
    /// NoticeRequirement PolicyObject; load-bearing for WF-LINT-001.
    pub fn linked_notice_requirement_ref(&self) -> Option<&str> {
        self.body
            .get("linkedNoticeRequirementRef")
            .and_then(Value::as_str)
    }

    /// `linkedAppealRightRef` — adverse Outcomes that trigger due
    /// process link to an AppealRight PolicyObject; load-bearing for
    /// WF-LINT-001 + SC-LINT-001.
    pub fn linked_appeal_right_ref(&self) -> Option<&str> {
        self.body
            .get("linkedAppealRightRef")
            .and_then(Value::as_str)
    }

    /// `decisionRuleRef` — present on PolicyObjects that bind to a
    /// DecisionRule (Decision elements + downstream consumers).
    pub fn decision_rule_ref(&self) -> Option<&str> {
        self.body.get("decisionRuleRef").and_then(Value::as_str)
    }

    /// `actor` — AuthorityGrant PolicyObjects name the actor whose
    /// authority they document; load-bearing for WF-LINT-005.
    pub fn actor(&self) -> Option<&str> {
        self.body.get("actor").and_then(Value::as_str)
    }

    /// `retentionPolicy` — EvidenceRequirement (and historically
    /// DataElement) PolicyObjects carry a typed
    /// [`crate::policy::RetentionPolicy`] per ADR-0083 r2.
    ///
    /// Returns `None` when the field is absent. Returns
    /// `Some(Err(...))` when the field is present but malformed
    /// (caller decides whether that is a hard error or a parse
    /// advisory; `WF-LINT-006` treats it as a hard error). Returns
    /// `Some(Ok(policy))` on success — at which point
    /// `policy.shape_violations()` is the next gate (semantic
    /// validation that schema couldn't fully capture).
    pub fn retention_policy(
        &self,
    ) -> Option<Result<crate::policy::RetentionPolicy, serde_json::Error>> {
        self.body
            .get("retentionPolicy")
            .map(|v| serde_json::from_value::<crate::policy::RetentionPolicy>(v.clone()))
    }

    /// `retentionPolicy` raw access — escape hatch for consumers that
    /// need the original JSON (e.g., to detect `$comment` keys or
    /// vendor extensions the typed struct elides). Prefer
    /// [`Self::retention_policy`] for everything else.
    pub fn retention_policy_raw(&self) -> Option<&Value> {
        self.body.get("retentionPolicy")
    }

    /// Legacy `retentionPeriod` — present on documents authored against
    /// the pre-ADR-0083 spec. Lint surfaces
    /// `SA-WARN-pom-MIGRATE-RETENTION` when this returns `Some`. The
    /// field is removed from the spec; this accessor exists solely so
    /// the migration advisory can fire.
    pub fn legacy_retention_period(&self) -> Option<&str> {
        self.body.get("retentionPeriod").and_then(Value::as_str)
    }

    /// `linkedPolicyObjects` — cross-cutting reference field used by
    /// Scenario / Conflict / Supersession PolicyObjects.
    pub fn linked_policy_objects(&self) -> Vec<&str> {
        self.body
            .get("linkedPolicyObjects")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }
}

impl MappingDocument {
    pub fn id(&self) -> Option<&str> {
        self.body.get("id").and_then(Value::as_str)
    }

    pub fn policy_object_ref(&self) -> Option<&str> {
        self.body.get("policyObjectRef").and_then(Value::as_str)
    }

    pub fn mapping_state(&self) -> Option<MappingState> {
        typed_str_field(&self.body, "mappingState")
    }

    pub fn targets(&self) -> Option<&Vec<Value>> {
        self.body.get("targets").and_then(Value::as_array)
    }

    pub fn unmapped_rationale(&self) -> Option<&str> {
        self.body.get("unmappedRationale").and_then(Value::as_str)
    }

    /// `mappings[]` — collection-form accessor. Returns the array if
    /// the document is the wrapper shape, otherwise None.
    pub fn collection(&self) -> Option<&Vec<Value>> {
        self.body.get("mappings").and_then(Value::as_array)
    }

    /// `extensionRecord` — embedded extension proposal. Mappings with
    /// `mappingState = requiresSpecExtension` carry one. Returns the
    /// raw object; consumers can read its `lifecycleState` to gate
    /// emission per SA-MUST-cmp-021.
    pub fn extension_record(&self) -> Option<&Value> {
        self.body.get("extensionRecord")
    }

    /// `extensionRecordRef` — alternative external-reference form
    /// (vs the inline `extensionRecord`). Some workspaces share an
    /// ExtensionRecord across multiple Mappings.
    pub fn extension_record_ref(&self) -> Option<&str> {
        self.body
            .get("extensionRecordRef")
            .and_then(Value::as_str)
    }

    /// `wosTarget` — when present at the Mapping body level
    /// (vs. inside `targets[*]`), surfaces the projection target
    /// JSON-pointer for tooling consumption.
    pub fn wos_target(&self) -> Option<&str> {
        self.body.get("wosTarget").and_then(Value::as_str)
    }

    /// `lifecycleState` — Mapping lifecycle (per
    /// `studio-to-wos-mapping.md` lifecycle enum: draft / approved /
    /// retracted / superseded).
    pub fn lifecycle_state(&self) -> Option<&str> {
        self.body.get("lifecycleState").and_then(Value::as_str)
    }
}

impl WorkflowIntentDocument {
    pub fn id(&self) -> Option<&str> {
        self.body.get("id").and_then(Value::as_str)
    }

    pub fn version(&self) -> Option<&str> {
        self.body.get("version").and_then(Value::as_str)
    }

    pub fn title(&self) -> Option<&str> {
        self.body.get("title").and_then(Value::as_str)
    }

    pub fn impact_level(&self) -> Option<&str> {
        self.body.get("impactLevel").and_then(Value::as_str)
    }

    pub fn lifecycle_state(&self) -> Option<WorkflowIntentLifecycleState> {
        typed_str_field(&self.body, "lifecycleState")
    }

    pub fn elements(&self) -> Option<&Vec<Value>> {
        self.body.get("elements").and_then(Value::as_array)
    }

    pub fn actors(&self) -> Option<&Vec<Value>> {
        self.body.get("actors").and_then(Value::as_array)
    }

    pub fn protected_category_refs(&self) -> Vec<&str> {
        self.body
            .get("protectedCategoryRefs")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// `wosVersionPin` (string-form) — Studio docs carry the pin as
    /// a free-form claims string (`"kernel@1.0, governance@1.0"`).
    /// Returns None when the pin is set as a typed object (use
    /// [`Self::wos_version_pin_typed`]).
    pub fn wos_version_pin(&self) -> Option<&str> {
        self.body.get("wosVersionPin").and_then(Value::as_str)
    }

    /// `wosVersionPin` (typed-object form) — kernel
    /// `$defs/WosVersionPin` shape (`{envelopeVersion, includedBlocks}`).
    /// Returns None when the pin is absent or set as a string (use
    /// [`Self::wos_version_pin`]).
    pub fn wos_version_pin_typed(&self) -> Option<&Value> {
        let v = self.body.get("wosVersionPin")?;
        if v.is_object() { Some(v) } else { None }
    }

    /// `effectivenessRef` — workflow-level Effectiveness scope IRI.
    /// Mirrors the per-PolicyObject `effectivenessRef` accessor but
    /// at the WorkflowIntent root (per `effectiveness-and-applicability.md`).
    pub fn effectiveness_ref(&self) -> Option<&str> {
        self.body.get("effectivenessRef").and_then(Value::as_str)
    }
}

impl ScenarioDocument {
    pub fn id(&self) -> Option<&str> {
        self.body.get("id").and_then(Value::as_str)
    }

    pub fn version(&self) -> Option<&str> {
        self.body.get("version").and_then(Value::as_str)
    }

    pub fn scenario_type(&self) -> Option<&str> {
        self.body.get("scenarioType").and_then(Value::as_str)
    }

    pub fn lifecycle_state(&self) -> Option<ScenarioLifecycleState> {
        typed_str_field(&self.body, "lifecycleState")
    }

    pub fn collection(&self) -> Option<&Vec<Value>> {
        self.body.get("scenarios").and_then(Value::as_array)
    }

    pub fn exercises_outcomes(&self) -> Vec<&str> {
        self.body
            .get("exercisesOutcomes")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    pub fn exercises_appeals(&self) -> Vec<&str> {
        self.body
            .get("exercisesAppeals")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// `expectedOutcome` — terminal-outcome assertion under
    /// `expectedTrace.expectedTerminalOutcome` per the spec, OR the
    /// shorter top-level form some Scenarios use. Returns whichever
    /// is present, with the typed-trace form preferred.
    pub fn expected_terminal_outcome(&self) -> Option<&str> {
        self.body
            .get("expectedTrace")
            .and_then(|t| t.get("expectedTerminalOutcome"))
            .and_then(Value::as_str)
            .or_else(|| {
                self.body
                    .get("expectedOutcome")
                    .and_then(Value::as_str)
            })
    }

    /// `expectedNotices` — Scenario MUST surface every NoticeRequirement
    /// that fires on its expected path. Sourced from the typed trace.
    pub fn expected_notices(&self) -> Vec<&str> {
        self.body
            .get("expectedTrace")
            .and_then(|t| t.get("expectedNotices"))
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// `expectedAppealBranch` — Scenario indicates which AppealRight
    /// the expected path triggers (load-bearing for AdverseDetermination
    /// + AppealFiled scenarios).
    pub fn expected_appeal_branch(&self) -> Option<&str> {
        self.body
            .get("expectedTrace")
            .and_then(|t| t.get("expectedAppealBranch"))
            .and_then(Value::as_str)
    }

    /// `linkedPolicyObjects` — Scenarios cite the PolicyObjects they
    /// exercise. Used by SC-LINT-005 (supersession-affected scenarios
    /// re-run check).
    pub fn linked_policy_objects(&self) -> Vec<&str> {
        self.body
            .get("linkedPolicyObjects")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// `lastRanAt` — ISO-8601 timestamp of the last simulation run.
    /// Used by SC-LINT-005 for supersession freshness checks.
    pub fn last_ran_at(&self) -> Option<&str> {
        self.body.get("lastRanAt").and_then(Value::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn policy_object_round_trips_with_arbitrary_body() {
        let doc = json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-test",
            "workspaceId": "ws-test",
            "kind": "NoticeRequirement",
            "body": {"text": "Sample"},
            "x-extra": 42
        });
        let parsed: PolicyObjectDocument =
            serde_json::from_value(doc.clone()).expect("parse");
        assert_eq!(parsed.marker, "1.0");
        assert_eq!(parsed.body.get("id"), Some(&json!("pol-test")));
        assert_eq!(parsed.body.get("kind"), Some(&json!("NoticeRequirement")));
        assert_eq!(parsed.body.get("x-extra"), Some(&json!(42)));

        let back = serde_json::to_value(&parsed).expect("write");
        assert_eq!(back, doc);
    }

    #[test]
    fn studio_document_dispatches_on_marker() {
        let doc = json!({
            "$wosStudioWorkspace": "1.0",
            "id": "ws-test",
            "title": "Test workspace"
        });
        let parsed: StudioDocument = serde_json::from_value(doc).expect("parse");
        match parsed {
            StudioDocument::Workspace(ws) => {
                assert_eq!(ws.body.get("id"), Some(&json!("ws-test")));
                assert_eq!(ws.body.get("title"), Some(&json!("Test workspace")));
            }
            other => panic!("expected workspace variant, got {other:?}"),
        }
    }

    #[test]
    fn field_helper_decodes_typed_value() {
        let doc = json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-x",
            "workspaceId": "ws-x",
            "kind": "Outcome"
        });
        let parsed: PolicyObjectDocument =
            serde_json::from_value(doc).expect("parse");
        let id: String = parsed
            .field::<String>("id")
            .expect("present")
            .expect("decodes");
        assert_eq!(id, "pol-x");

        // Absent field → None.
        assert!(parsed.field::<String>("missing").is_none());
    }

    #[test]
    fn marker_const_matches_serde_rename() {
        assert_eq!(PolicyObjectDocument::MARKER_KEY, "$wosStudioPolicyObject");
        assert_eq!(WorkspaceDocument::MARKER_KEY, "$wosStudioWorkspace");
    }

    #[test]
    fn studio_document_body_dispatches_on_every_variant() {
        // Regression guard for the 14-arm exhaustive match in
        // `StudioDocument::body()`. If a new StudioDocument variant
        // is added without a corresponding arm, this test won't
        // compile (because the match is now non-exhaustive in the
        // implementation, surfacing as a build error). The runtime
        // assertions confirm every existing arm dispatches to the
        // correct underlying body.
        let cases: Vec<(&str, serde_json::Value, &str)> = vec![
            (
                "$wosStudioApproval",
                json!({"$wosStudioApproval": "1.0", "kind": "ApprovalDecision",
                       "decision": {"id": "ad-1"}}),
                "kind",
            ),
            (
                "$wosStudioBinding",
                json!({"$wosStudioBinding": "1.0", "id": "b-1", "kind": "ServiceBinding",
                       "body": {"servicePort": "x"}}),
                "id",
            ),
            (
                "$wosStudioEffectiveness",
                json!({"$wosStudioEffectiveness": "1.0", "id": "eff-1",
                       "kind": "EffectivePeriod", "body": {}}),
                "id",
            ),
            (
                "$wosStudioIdentitySubject",
                json!({"$wosStudioIdentitySubject": "1.0", "id": "subj-1",
                       "displayName": "X", "actorType": "human",
                       "validUntil": null, "attestationLevel": "session"}),
                "id",
            ),
            (
                "$wosStudioMapping",
                json!({"$wosStudioMapping": "1.0",
                       "mapping": {"id": "m-1",
                                   "subjectPolicyObjectRef": "pol-x",
                                   "mappingState": "authoringOnly"}}),
                "mapping",
            ),
            (
                "$wosStudioMigrationPath",
                json!({"$wosStudioMigrationPath": "1.0", "id": "mp-1"}),
                "id",
            ),
            (
                "$wosStudioPolicyObject",
                json!({"$wosStudioPolicyObject": "1.0", "id": "pol-1",
                       "kind": "Assumption", "lifecycleState": "draft",
                       "originClass": "assumption"}),
                "id",
            ),
            (
                "$wosStudioProvenance",
                json!({"$wosStudioProvenance": "1.0",
                       "kind": "AuthoringProvenanceRecord"}),
                "kind",
            ),
            (
                "$wosStudioReadiness",
                json!({"$wosStudioReadiness": "1.0", "kind": "RuleRegistry",
                       "rules": []}),
                "kind",
            ),
            (
                "$wosStudioScenario",
                json!({"$wosStudioScenario": "1.0", "id": "sc-1"}),
                "id",
            ),
            (
                "$wosStudioSource",
                json!({"$wosStudioSource": "1.0", "id": "src-1"}),
                "id",
            ),
            (
                "$wosStudioTerminologyMap",
                json!({"$wosStudioTerminologyMap": "1.0", "id": "tm-1"}),
                "id",
            ),
            (
                "$wosStudioWorkflowIntent",
                json!({"$wosStudioWorkflowIntent": "1.0", "id": "wfi-1"}),
                "id",
            ),
            (
                "$wosStudioWorkspace",
                json!({"$wosStudioWorkspace": "1.0", "id": "ws-1",
                       "title": "T", "reviewerRoles": []}),
                "id",
            ),
        ];
        assert_eq!(
            cases.len(),
            14,
            "body() dispatches over 14 StudioDocument variants; \
             update this test if a variant is added or removed",
        );
        for (marker, doc, sentinel_key) in cases {
            let parsed: StudioDocument = serde_json::from_value(doc.clone())
                .unwrap_or_else(|e| panic!("parse {marker}: {e}"));
            let body = parsed.body();
            assert!(
                body.contains_key(sentinel_key),
                "{marker}: body() dispatched to wrong arm — sentinel key \
                 {sentinel_key:?} not present in {body:?}"
            );
        }
    }

    #[test]
    fn policy_object_typed_accessors() {
        let doc: PolicyObjectDocument = serde_json::from_value(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-x",
            "workspaceId": "ws-x",
            "kind": "Outcome",
            "lifecycleState": "approved",
            "originClass": "source",
            "mappingState": "mapsToWos",
            "triggersDueProcess": true,
            "citations": [{"sourceCitationRef": "c-1"}],
        }))
        .expect("parse");
        assert_eq!(doc.id(), Some("pol-x"));
        assert_eq!(doc.workspace_id(), Some("ws-x"));
        assert_eq!(doc.kind(), Some("Outcome"));
        assert_eq!(
            doc.lifecycle_state(),
            Some(PolicyObjectLifecycleState::Approved)
        );
        assert_eq!(doc.origin_class(), Some(OriginClass::Source));
        assert_eq!(doc.mapping_state(), Some(MappingState::MapsToWos));
        assert_eq!(doc.triggers_due_process(), Some(true));
        assert_eq!(doc.citations().map(|c| c.len()), Some(1));
    }

    #[test]
    fn mapping_typed_accessors_collection_form() {
        let doc: MappingDocument = serde_json::from_value(json!({
            "$wosStudioMapping": "1.0",
            "mappings": [{"id": "m-1", "policyObjectRef": "pol-x"}]
        }))
        .expect("parse");
        // Single-form fields absent on collection-shape doc.
        assert_eq!(doc.id(), None);
        assert_eq!(doc.collection().map(|c| c.len()), Some(1));
    }

    #[test]
    fn workflow_intent_typed_accessors() {
        let doc: WorkflowIntentDocument = serde_json::from_value(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-1",
            "version": "0.2.0",
            "title": "T",
            "impactLevel": "rights-impacting",
            "lifecycleState": "scenarioTested",
            "protectedCategoryRefs": ["pc-race", "pc-disability", "pc-age"],
            "wosVersionPin": "kernel@1.0",
            "elements": [],
            "actors": [],
        }))
        .expect("parse");
        assert_eq!(doc.id(), Some("wfi-1"));
        assert_eq!(doc.version(), Some("0.2.0"));
        assert_eq!(doc.impact_level(), Some("rights-impacting"));
        assert_eq!(
            doc.lifecycle_state(),
            Some(WorkflowIntentLifecycleState::ScenarioTested)
        );
        assert_eq!(doc.protected_category_refs().len(), 3);
        assert_eq!(doc.wos_version_pin(), Some("kernel@1.0"));
    }

    #[test]
    fn scenario_typed_accessors() {
        let doc: ScenarioDocument = serde_json::from_value(json!({
            "$wosStudioScenario": "1.0",
            "id": "sc-1",
            "version": "1.0.0",
            "scenarioType": "appeal-filed",
            "lifecycleState": "passing",
            "exercisesOutcomes": ["out-deny"],
            "exercisesAppeals": ["ar-1"]
        }))
        .expect("parse");
        assert_eq!(doc.id(), Some("sc-1"));
        assert_eq!(doc.version(), Some("1.0.0"));
        assert_eq!(doc.scenario_type(), Some("appeal-filed"));
        assert_eq!(doc.lifecycle_state(), Some(ScenarioLifecycleState::Passing));
        assert_eq!(doc.exercises_outcomes(), vec!["out-deny"]);
        assert_eq!(doc.exercises_appeals(), vec!["ar-1"]);
    }

    // ── F5.0: typed accessors for newly load-bearing fields ─────────

    #[test]
    fn policy_object_outcome_accessors() {
        let doc: PolicyObjectDocument = serde_json::from_value(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-out-deny",
            "kind": "Outcome",
            "polarity": "adverse",
            "triggersDueProcess": true,
            "linkedNoticeRequirementRef": "pol-notice-1",
            "linkedAppealRightRef": "pol-appeal-1",
            "linkedPolicyObjects": ["pol-x", "pol-y"]
        }))
        .expect("parse");
        assert_eq!(doc.polarity(), Some("adverse"));
        assert_eq!(doc.linked_notice_requirement_ref(), Some("pol-notice-1"));
        assert_eq!(doc.linked_appeal_right_ref(), Some("pol-appeal-1"));
        assert_eq!(doc.linked_policy_objects(), vec!["pol-x", "pol-y"]);
    }

    #[test]
    fn policy_object_authority_grant_accessors() {
        let doc: PolicyObjectDocument = serde_json::from_value(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-auth-1",
            "kind": "AuthorityGrant",
            "actor": "caseworker"
        }))
        .expect("parse");
        assert_eq!(doc.actor(), Some("caseworker"));
        // Polarity / notice / appeal absent on AuthorityGrant.
        assert_eq!(doc.polarity(), None);
        assert_eq!(doc.linked_notice_requirement_ref(), None);
    }

    #[test]
    fn policy_object_data_element_accessors() {
        let doc: PolicyObjectDocument = serde_json::from_value(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-data-ssn",
            "kind": "DataElement",
            "dpvSensitivity": "dpv:SpecialCategoryData",
            "canonicalTermRef": "urn:wos:vocab:identity:ssn",
            "retentionPolicy": {
                "duration": "P7Y",
                "disposalAction": "purge"
            }
        }))
        .expect("parse");
        assert_eq!(doc.dpv_sensitivity(), Some("dpv:SpecialCategoryData"));
        assert_eq!(
            doc.canonical_term_ref(),
            Some("urn:wos:vocab:identity:ssn")
        );
        let policy = doc
            .retention_policy()
            .expect("present")
            .expect("parses post-E8.3");
        assert_eq!(policy.duration.as_deref(), Some("P7Y"));
        assert_eq!(
            policy.disposal_action,
            crate::policy::DisposalAction::Purge
        );
        assert!(policy.shape_violations().is_empty());
    }

    #[test]
    fn policy_object_legacy_retention_period_surfaces() {
        let doc: PolicyObjectDocument = serde_json::from_value(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-data-old",
            "kind": "EvidenceRequirement",
            "retentionPeriod": "P5Y"
        }))
        .expect("parse");
        // Migration advisory hook: typed accessor returns None
        // (no `retentionPolicy` field), legacy accessor returns
        // Some so WF-LINT-006 can emit SA-WARN-pom-MIGRATE-RETENTION.
        assert!(doc.retention_policy().is_none());
        assert_eq!(doc.legacy_retention_period(), Some("P5Y"));
    }

    #[test]
    fn policy_object_retention_policy_parse_error_surfaces() {
        let doc: PolicyObjectDocument = serde_json::from_value(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-data-bad",
            "kind": "EvidenceRequirement",
            "retentionPolicy": {"duration": "P7Y"}
        }))
        .expect("parse");
        // Missing `disposalAction` — schema would catch it; the typed
        // accessor surfaces as a parse error so consumers can
        // distinguish "absent" from "present but malformed".
        let result = doc.retention_policy().expect("present");
        assert!(result.is_err(), "expected parse error, got {result:?}");
    }

    #[test]
    fn policy_object_decision_rule_ref() {
        let doc: PolicyObjectDocument = serde_json::from_value(json!({
            "$wosStudioPolicyObject": "1.0",
            "id": "pol-decision-1",
            "kind": "DecisionRule",
            "decisionRuleRef": "tab-eligibility"
        }))
        .expect("parse");
        assert_eq!(doc.decision_rule_ref(), Some("tab-eligibility"));
    }

    #[test]
    fn mapping_extension_record_accessors() {
        let doc: MappingDocument = serde_json::from_value(json!({
            "$wosStudioMapping": "1.0",
            "id": "m-1",
            "policyObjectRef": "pol-x",
            "mappingState": "requiresSpecExtension",
            "lifecycleState": "approved",
            "wosTarget": "$.governance.policyObjects[0]",
            "extensionRecord": {
                "id": "ext-1",
                "lifecycleState": "open"
            }
        }))
        .expect("parse");
        assert!(doc.extension_record().is_some());
        assert_eq!(
            doc.extension_record().unwrap()["lifecycleState"],
            json!("open")
        );
        assert_eq!(doc.wos_target(), Some("$.governance.policyObjects[0]"));
        assert_eq!(doc.lifecycle_state(), Some("approved"));
    }

    #[test]
    fn workflow_intent_typed_pin_accessors() {
        let string_pin: WorkflowIntentDocument = serde_json::from_value(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-1",
            "wosVersionPin": "kernel@1.0, governance@1.0"
        }))
        .expect("parse");
        assert_eq!(
            string_pin.wos_version_pin(),
            Some("kernel@1.0, governance@1.0")
        );
        assert!(string_pin.wos_version_pin_typed().is_none());

        let typed_pin: WorkflowIntentDocument = serde_json::from_value(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-2",
            "wosVersionPin": {
                "envelopeVersion": "1.0",
                "includedBlocks": ["governance"]
            }
        }))
        .expect("parse");
        assert!(typed_pin.wos_version_pin_typed().is_some());
        assert_eq!(
            typed_pin.wos_version_pin_typed().unwrap()["envelopeVersion"],
            json!("1.0")
        );
        // String accessor returns None when pin is the typed object form.
        assert!(typed_pin.wos_version_pin().is_none());
    }

    #[test]
    fn workflow_intent_effectiveness_ref() {
        let doc: WorkflowIntentDocument = serde_json::from_value(json!({
            "$wosStudioWorkflowIntent": "1.0",
            "id": "wfi-1",
            "effectivenessRef": "eff-fed-cfr-273-current"
        }))
        .expect("parse");
        assert_eq!(
            doc.effectiveness_ref(),
            Some("eff-fed-cfr-273-current")
        );
    }

    #[test]
    fn scenario_expected_trace_accessors() {
        let doc: ScenarioDocument = serde_json::from_value(json!({
            "$wosStudioScenario": "1.0",
            "id": "sc-1",
            "expectedTrace": {
                "expectedTerminalOutcome": "pol-out-deny",
                "expectedNotices": ["pol-notice-1", "pol-notice-2"],
                "expectedAppealBranch": "pol-appeal-fair-hearing"
            },
            "linkedPolicyObjects": ["pol-x"],
            "lastRanAt": "2026-04-15T10:00:00Z"
        }))
        .expect("parse");
        assert_eq!(doc.expected_terminal_outcome(), Some("pol-out-deny"));
        assert_eq!(
            doc.expected_notices(),
            vec!["pol-notice-1", "pol-notice-2"]
        );
        assert_eq!(
            doc.expected_appeal_branch(),
            Some("pol-appeal-fair-hearing")
        );
        assert_eq!(doc.linked_policy_objects(), vec!["pol-x"]);
        assert_eq!(doc.last_ran_at(), Some("2026-04-15T10:00:00Z"));
    }

    #[test]
    fn scenario_expected_terminal_falls_back_to_top_level() {
        // Some Scenarios use the shorter top-level `expectedOutcome`
        // form; the typed accessor falls back to it.
        let doc: ScenarioDocument = serde_json::from_value(json!({
            "$wosStudioScenario": "1.0",
            "id": "sc-1",
            "expectedOutcome": "pol-out-approved"
        }))
        .expect("parse");
        assert_eq!(
            doc.expected_terminal_outcome(),
            Some("pol-out-approved")
        );
    }
}
