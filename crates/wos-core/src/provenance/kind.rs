// Rust guideline compliant 2026-02-21

use serde::{Deserialize, Serialize};

/// Provenance record type discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProvenanceKind {
    /// Lifecycle state transition.
    StateTransition,
    /// Event that matched no transition (Kernel S4.9).
    UnmatchedEvent,
    /// Case state field mutation (Kernel S5.4).
    CaseStateMutation,
    /// Governed case identity was created.
    ///
    /// `wos-core` owns the shared provenance kind, but binding crates or host
    /// runtime code own payload assembly until WOS specifies a canonical
    /// binding-agnostic `caseCreated` data shape in spec and schema.
    CaseCreated,
    /// Intake handoff was accepted by host runtime policy.
    IntakeAccepted,
    /// Intake handoff was rejected by host runtime policy.
    IntakeRejected,
    /// Intake handoff was deferred by host runtime policy.
    IntakeDeferred,
    /// Timer created (Lifecycle Detail S6.7).
    TimerCreated,
    /// Timer fired (Lifecycle Detail S6.7).
    TimerFired,
    /// Timer cancelled (Lifecycle Detail S6.7).
    TimerCancelled,
    /// An `onEntry` lifecycle hook executed.
    OnEntry,
    /// An `onExit` lifecycle hook executed.
    OnExit,
    /// Action executed during onEntry, onExit, or transition.
    ActionExecuted,
    /// Duration string could not be parsed; timer deadline set to zero.
    InvalidDuration,
    /// Timer fired beyond its tolerance window (LCD S6.6, Runtime S7.2).
    ToleranceViolation,
    /// Continuous-mode re-evaluation hit the 100-cycle convergence cap for
    /// a single triggering mutation (Runtime §10.3). The record's `outcome`
    /// field carries the reserved literal `"convergenceCapReached"` (kernel
    /// `$defs/ProvenanceOutcome`); `data` carries `triggeringMutation` and
    /// `cyclesUsed` so downstream tooling can locate the cycle.
    ConvergenceCapReached,

    // ── Capability preconditions (AI S3.3.1) ───────────────────────
    /// A capability precondition was evaluated.
    ///
    /// Required `data` shape: `{"capabilityId": "<id>", "invocationBlocked":
    /// <bool>, ...}`. When `invocationBlocked` is `true` the record's
    /// `outcome` MUST be the reserved kernel literal
    /// `"preconditionNotSatisfied"` (Kernel §8.2.2). Schema-validated by
    /// `$defs/CapabilityInvocationRecord` in `wos-workflow.schema.json`,
    /// composed onto `FactsTierRecord` via `allOf` so every conformant
    /// provenance log participates in the MUST.
    CapabilityInvocation,

    // ── Deontic enforcement (AI S4) ────────────────────────────────
    /// A deontic constraint was violated (AI S4.2–S4.4).
    DeonticViolation,
    /// Deontic evaluation order record (AI S4.6).
    DeonticEvaluation,
    /// Resolved effective action from multiple violations (AI S4.6).
    DeonticResolution,
    /// Deontic constraint bypass with rationale (AI S4.7).
    DeonticBypass,
    /// Rights violation not attributed to agent (AI S4.5).
    RightsViolation,
    /// Consistency check contradiction (AI S4.7).
    ConsistencyViolation,

    // ── Autonomy (AI S5) ───────────────────────────────────────────
    /// Agent attempted to override a human decision (AI S3.7).
    AutonomyViolation,
    /// Autonomy level was capped by impact level or calibration (AI S5.3).
    AutonomyCapped,
    /// Effective autonomy computed from multiple sources (AI S5.3).
    AutonomyComputed,
    /// Assistive agent required human confirmation task (AI S5.3).
    HumanTaskCreated,
    /// Tool governance violation (AdvGov S6.1).
    ToolViolation,
    /// Escalation pending human approval (AI S5.4).
    EscalationPending,
    /// Autonomy demotion applied (AI S5.5).
    AutonomyDemotion,

    // ── Confidence (AI S7) ─────────────────────────────────────────
    /// Confidence violation — missing, uncalibrated, or below floor (AI S7).
    ConfidenceViolation,
    /// Confidence decay applied (AI S7.5).
    ConfidenceDecay,
    /// Cumulative confidence below threshold (AI S7.7).
    CumulativeConfidenceViolation,
    /// Session paused due to confidence threshold (AdvGov S5.4).
    SessionPaused,
    /// Ground truth label recorded from human review (AdvGov S9.3).
    GroundTruthLabel,

    // ── Agent lifecycle (AI S3, S6) ────────────────────────────────
    AgentOutput,
    ActorTypeViolation,
    AgentProvenanceAnnotation,
    AgentVersionChange,
    NarrativeTierRecorded,
    ConstraintTamperBlocked,
    DriftReclassification,
    AgentStateTransition,
    ProxyInvocation,
    DispositiveViolation,

    // ── Fallback (AI S8) ───────────────────────────────────────────
    FallbackTriggered,
    FallbackAttempt,
    FallbackTerminal,

    // ── Due process (WG S4, S6, S7) ────────────────────────────────
    NoticeSent,
    SeparationViolation,
    AppealFiled,
    ProtocolViolation,
    IndependentFirstEnforced,
    SamplingDecision,
    OverrideViolation,
    OverrideRecorded,

    // ── Pipeline (WG S8) ───────────────────────────────────────────
    PipelineStageCompleted,
    PipelineRiskProfile,
    PipelineRejection,
    TaskCreated,
    TaskPresented,
    TaskDismissed,
    TaskDraftPersisted,
    TaskResponseSubmitted,
    TaskResponseRejected,
    DataMapping,
    TaskCompleted,
    TaskFailed,
    TaskSkipped,
    ParameterResolved,

    // ── Compensation (Kernel S9.8) ─────────────────────────────────
    CompensationLogEntry,
    CompensationExecuted,
    CompensationScopeBoundary,

    // ── Delegation (WG S9) ─────────────────────────────────────────
    DelegationViolation,

    // ── Durability (Kernel S10) ────────────────────────────────────
    InstanceResumed,
    StepResultPersisted,
    IdempotencyDedup,
    InstanceMigrated,
    ContractValidation,
    HistoryCleared,

    // ── DCR (Advanced Governance) ──────────────────────────────────
    DcrActivityExecuted,
    DcrRelationEvaluated,
    DcrResolutionError,
    ZoneSatisfied,
    EquityAlert,

    // ── Verification (Advanced Governance) ─────────────────────────
    VerificationReportProduced,
    ImmutabilityViolation,
    ActivationBlocked,

    // ── Sidecar (Business Calendar, Notification) ──────────────────
    CalendarIgnored,
    NotificationSuppressed,

    // ── Configuration warnings (cross-cutting) ──────────────────────
    /// A configuration reference failed to resolve, or a configured
    /// operation (e.g. notification render) failed at runtime.
    ///
    /// Generic carrier for four spec MUSTs that require provenance for
    /// declarative-config failures without binding a more specific
    /// `recordKind`:
    /// - `specs/ai/drift-monitor.md:77` — unresolvable `policyRef`.
    /// - `specs/governance/workflow-governance.md:154` — unresolvable
    ///   `continuationPolicyRef`.
    /// - `specs/sidecars/notification-template.md:199` — template key not
    ///   found.
    /// - `specs/sidecars/notification-template.md:222` — notification
    ///   rendering failure.
    ///
    /// Required `data.subject` discriminator selects the failure site;
    /// reserved literals are `drift-monitor.policyRef`,
    /// `governance.continuationPolicyRef`, `notification-template.key`,
    /// `notification-template.render`. Vendor extensions use an `x-`
    /// prefix. Distinct from `CalendarIgnored` / `NotificationSuppressed`,
    /// which are sidecar-fallback semantics, not config-resolution
    /// failures.
    ConfigurationWarning,

    // ── Relationship provenance (Kernel S7) ────────────────────────
    RelationshipChanged,

    // ── Milestones (Kernel S4.13) ──────────────────────────────────
    /// A milestone condition became true for the first time (Kernel S4.13).
    ///
    /// `data` carries `{"milestoneId": "<id>"}`.
    MilestoneFired,

    // ── CloudEvents bindings (Integration Profile NB.3) ───────────
    /// An outbound CloudEvent was emitted by an `event-emit` binding.
    ///
    /// `data` carries the full CloudEvent envelope (all CE attributes + `data`).
    EventEmitted,

    /// An inbound CloudEvent was successfully consumed by an `event-consume` binding.
    ///
    /// `data` carries the full CloudEvent envelope (all CE attributes + `data`).
    EventConsumed,

    /// An inbound CloudEvent resolved a pending callback registered by a `callback` binding.
    ///
    /// `data` carries the full CloudEvent envelope and the `subject` used for correlation.
    CallbackReceived,

    /// A `callback` binding fired and is waiting for a matching inbound CloudEvent.
    ///
    /// `data` carries `{"subject": "<subject>", "bindingId": "<id>", "expectedUntil": "<iso>"}`.
    CallbackPending,

    // ── Arazzo / Tool / Policy-engine bindings (Integration Profile NB.4) ─
    /// A single step of an Arazzo multi-step sequence completed (or failed).
    ///
    /// `data` carries `{"stepId": "<id>", "outcome": "ok"|"failed", "durationMs": <n>, ...}`.
    ArazzoStep,

    /// A non-HTTP tool binding was invoked and produced a result.
    ///
    /// `data` carries `{"toolId": "<id>", "outcome": "ok"|"failed", ...}`.
    ToolInvoked,

    /// An external policy engine evaluated a request and returned a decision.
    ///
    /// `data` carries `{"decision": "allow"|"deny"|"indeterminate",
    /// "reasonsCount": <n>, "obligationsCount": <n>, ...}`.
    PolicyDecision,

    // ── Signature Profile (WOS-T4) ─────────────────────────────────
    /// A signer affirmed a document under a Signature Profile.
    ///
    /// `data` carries signer, role, document, identity-binding, consent,
    /// ceremony, profile, Formspec response, and custody eligibility fields.
    SignatureAffirmation,

    // ── Amendment & supersession (ADR 0066) ─────────────────────────
    /// A correction to a non-determination event was authorized (ADR 0066 §1).
    ///
    /// Mode 1 of the closed five-mode supersession taxonomy
    /// (Correction / Amendment / Supersession / Rescission / Reinstatement).
    /// `data` carries `correctionTargetEventHash`, `correctedFieldSet`
    /// (JSON-pointer strings), `reason`, `authorizingActorId`, and
    /// `authorityBasis` (discriminated union: `{kind, value}` where
    /// `kind` is `"uri"` or `"actorPolicyRef"`).
    CorrectionAuthorized,
    /// An amendment to a prior determination was authorized (ADR 0066 §2).
    ///
    /// Mode 2 of the five-mode supersession taxonomy. Pairs with
    /// `DeterminationAmended`, which carries the new value. `data` carries
    /// `amendmentTargetEventHash`, `priorDeterminationHash`, `reason`,
    /// `authorizingActorId`, `authorityBasis`.
    AmendmentAuthorized,
    /// A determination was amended; new value supersedes the prior (ADR 0066 §2).
    ///
    /// `data` carries `priorDeterminationHash`, `newDeterminationValue`,
    /// `amendmentAuthorizationEventHash` (back-reference to the authorizing
    /// `AmendmentAuthorized` record).
    DeterminationAmended,
    /// A rescission of a prior determination was authorized (ADR 0066 §3).
    ///
    /// Mode 4 of the five-mode supersession taxonomy. `data` carries
    /// `rescissionTargetEventHash`, `priorDeterminationHash`, `reason`,
    /// `authorizingActorId`, `authorityBasis`, and an optional
    /// `migrationPinChange` payload (Q32 cross-chain hash linkage —
    /// supersession that also changes a version pin carries
    /// `{newChainPinEventHash, priorPinSet, newPinSet}`).
    RescissionAuthorized,
    /// A determination was rescinded (ADR 0066 §3).
    ///
    /// `data` carries `priorDeterminationHash` and
    /// `rescissionAuthorizationEventHash` (back-reference to the authorizing
    /// `RescissionAuthorized` record).
    DeterminationRescinded,
    /// A previously rescinded determination was reinstated (ADR 0066 §4).
    ///
    /// Mode 5 of the closed five-mode supersession taxonomy
    /// (owner directive Q26). Re-activates a determination from a
    /// non-operative (post-rescission) state and is distinct from amendment:
    /// the substantive value is unchanged; only the operative status flips
    /// back. `data` carries `priorRescissionEventHash`,
    /// `reactivationAuthorizationEventHash`, and `reason`.
    Reinstated,
    /// A standalone authorization attestation supporting an amendment,
    /// rescission, or reinstatement chain (ADR 0066 §5).
    ///
    /// Records the authorizing actor's policy basis for the supersession
    /// act. `data` carries `authorizingActorId`, `authorityBasis`,
    /// `policyPredicate` (e.g. `"amendment-authority"`,
    /// `"rescission-authority"`, `"reinstatement-authority"`), and an
    /// optional `assuranceLevel` (e.g. `"high"`, `"standard"`).
    AuthorizationAttestation,

    // ── Statutory clocks (ADR 0067) ──────────────────────────────
    /// A statutory or SLA clock was started (ADR 0067 §2).
    ///
    /// `data` carries `clockId`, `clockKind` (open enum:
    /// `"AppealClock"` | `"ProcessingSLA"` | `"GrantExpiry"` |
    /// `"StatuteClock"` | `x-*`), `originEventHash`, `duration`
    /// (ISO 8601 duration), `computedDeadline` (RFC 3339), and
    /// optional `calendarRef` and `statuteReference`.
    ClockStarted,
    /// A statutory or SLA clock resolved (ADR 0067 §3).
    ///
    /// `data` carries `clockId`, `originClockHash`, `resolution`
    /// (closed enum: `"satisfied"` | `"elapsed"` | `"paused"` |
    /// `"cancelled"` — see [`ClockResolvedResolution`], `resolvedAt`
    /// (RFC 3339), and `resolvingEventHash` (required when `resolution` is
    /// `"paused"`; otherwise optional).
    ClockResolved,

    // ── Identity attestation (ADR 0068) ──────────────────────────
    /// A cross-tenant identity attestation was recorded (ADR 0068 §4, Q15).
    ///
    /// Pulled inline as a first-class kind per maximalist Q15. `data`
    /// carries `subjectGlobalId`, `assuranceLevel` (open enum, e.g.
    /// `"low"` | `"standard"` | `"high"` | `"very-high"`),
    /// `attestationProvider`, `providerAttestationId`, `attestedAt`
    /// (RFC 3339), optional `validUntil` (RFC 3339), and
    /// `attestedPredicates` (open list, e.g. `["legal-name-verified",
    /// "age-of-majority"]`).
    IdentityAttestation,

    // ── Clock skew (ADR 0069) ────────────────────────────────────
    /// Clock skew between processor and substrate was observed (ADR 0069 §3).
    ///
    /// Emitted when processor and substrate timestamps diverge beyond the
    /// deployment-configured threshold (default: 1000ms). `data` carries
    /// `processorAuthoredAt` (RFC 3339), `substrateCreatedAt` (RFC 3339),
    /// `skewMilliseconds` (signed: positive = processor ahead),
    /// `thresholdMilliseconds`, and `eventHash`.
    ClockSkewObserved,

    // ── Failure & compensation (ADR 0070) ────────────────────────
    /// A commit attempt against the substrate failed (ADR 0070 §2).
    ///
    /// `data` carries `targetEventHash`, `failureKind` (closed enum
    /// via [`CommitFailureKind`]: `networkTimeout` | `substrateDown` |
    /// `hashConflict` | `other`), `attemptCount`,
    /// `retryBudgetRemainingMs`, and optional `errorPayload`.
    CommitAttemptFailure,
    /// An authorization attempt was rejected by policy (ADR 0070 §4).
    ///
    /// `data` carries `attemptedActorId`, `attemptedAction` (e.g.
    /// `"transition:approve"`, `"submit:taskResponse"`),
    /// `targetResourceId`, `rejectionReason`, and optional
    /// `policyDecisionRef`.
    AuthorizationRejected,

    // ── Migration & version pins (ADR 0071) ──────────────────────
    /// A migration pin set changed (ADR 0071 §3).
    ///
    /// `data` carries `priorPinSet` and `newPinSet` (4-field pin trees
    /// per maximalist Q33: `formspec.definitionVersion`,
    /// `wos.$wosWorkflowVersion`, `trellis.envelopeVersion`,
    /// `trellis.conformanceClass`), `authorizingActorId`,
    /// `authorityBasis`, and `migrationRationale`.
    MigrationPinChanged,
}

impl ProvenanceKind {
    /// Whether this kind represents a governance / AI policy or rule that
    /// applied during event processing.
    ///
    /// Used by the runtime to decide which records should have their `event`
    /// field stamped with the drain's processed event (for records whose
    /// constructors left `event = None` — the governance layer does this
    /// uniformly today), and by the conformance trace builder to decide
    /// which records contribute a `PolicyApplication` entry on a trace step.
    ///
    /// Semantics are "applied" not "violated". Violation-shaped kinds
    /// (`DeonticViolation`, `AutonomyViolation`, `ConfidenceViolation`, ...)
    /// signal that a rule FAILED, not that one applied, so they are
    /// intentionally excluded. `DeonticBypass` and `AutonomyDemotion` are
    /// semantically "policy overridden / demoted", not accept-and-fire —
    /// they are included because downstream teaching-signal consumers want
    /// to see that an override/demotion DID happen (with its rationale)
    /// when reasoning about a workflow's actual behaviour. Consumers can
    /// filter them out if they specifically want accept-and-fire semantics.
    pub fn is_policy_application(&self) -> bool {
        matches!(
            self,
            ProvenanceKind::DeonticEvaluation
                | ProvenanceKind::DeonticResolution
                | ProvenanceKind::DeonticBypass
                | ProvenanceKind::AutonomyComputed
                | ProvenanceKind::AutonomyDemotion
                | ProvenanceKind::OverrideRecorded
                | ProvenanceKind::PolicyDecision
                | ProvenanceKind::PipelineRiskProfile
        )
    }
}
