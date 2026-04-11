# Technical Reference Implementation: Formspec + WOS + Temporal

**Status:** Reference architecture. No production code exists. This document defines the integration seams and concrete patterns for the first engine binding.

**Stack:**
- **Formspec** (TypeScript/WASM) -- intake forms, validation, FEL evaluation
- **wos-core** (Rust) -- governance evaluation, typed models, provenance
- **Temporal** (Rust SDK) -- durable execution, state persistence, timers, crash recovery
- **SaaS layer** (TypeScript) -- reviewer dashboard, respondent portal, notifications

---

## 1. Architecture

```text
                    ┌──────────────────────────────────────────┐
                    │            SaaS API Layer (TS)           │
                    │  Reviewer dashboard · Respondent portal  │
                    │  Notification service · Analytics        │
                    └─────────────┬───────────────┬────────────┘
                                  │               │
                    ┌─────────────▼───┐   ┌───────▼────────────┐
                    │  Formspec Engine │   │  Temporal Client    │
                    │  (TS/WASM)       │   │  (signals, queries) │
                    │  Form render,    │   │                     │
                    │  validation, FEL │   │  start_workflow()   │
                    └─────────────┬───┘   │  signal()           │
                                  │       │  query()            │
                                  │       └───────┬─────────────┘
                                  │               │
                    ┌─────────────▼───────────────▼─────────────┐
                    │         Temporal Worker (Rust)             │
                    │                                           │
                    │  ┌─────────────────────────────────────┐  │
                    │  │  WOS Case Workflow                   │  │
                    │  │                                     │  │
                    │  │  wos_core::Evaluator                │  │
                    │  │    - process_event()                │  │
                    │  │    - governance checks              │  │
                    │  │    - provenance recording           │  │
                    │  │                                     │  │
                    │  │  Coprocessor                        │  │
                    │  │    - formspec_submission_to_case()  │  │
                    │  │    - response_to_case_file()        │  │
                    │  └─────────────────────────────────────┘  │
                    │                                           │
                    │  Activities:                               │
                    │    invoke_external_service()               │
                    │    send_notification()                     │
                    │    validate_contract()                     │
                    │    record_provenance()                     │
                    │    create_human_task()                     │
                    │    invoke_ai_agent()                       │
                    └──────────────┬────────────────────────────┘
                                   │
                    ┌──────────────▼────────────────────────────┐
                    │         External Services                  │
                    │  AI extraction agent · OPA/Cedar           │
                    │  Notification delivery · Document storage  │
                    │  Employer verification · State mainframe   │
                    └───────────────────────────────────────────┘
```

### What runs where

| Component | Language | Runs as | Owns |
|-----------|----------|---------|------|
| Temporal Server | Go | Managed service or self-hosted | Workflow state, timer scheduling, event history, crash recovery |
| Temporal Worker | Rust | Long-running process (1+ replicas) | Workflow logic, governance evaluation, activity execution |
| wos-core | Rust | Library linked into the worker | Typed models, evaluation algorithm, governance rules, provenance construction |
| Formspec Engine | TS/WASM | Browser (respondent) + server (validation) | Form rendering, FEL evaluation, contract validation |
| SaaS API | TypeScript | HTTP server | Dashboard, portal, Temporal client calls, notification dispatch |
| Provenance Store | Postgres | Database | Append-only provenance log (separate from Temporal's event history) |
| Task Store | Postgres | Database | Human task state, queue, assignment |

---

## 2. Crate Structure

```text
wos-temporal/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── workflow.rs          # Temporal workflow: WOS case lifecycle
│   ├── coprocessor.rs       # Formspec → WOS handoff
│   ├── activities/
│   │   ├── mod.rs
│   │   ├── human_task.rs    # create_human_task, complete_task, claim_task
│   │   ├── service.rs       # invoke_external_service (AI agents, APIs)
│   │   ├── notification.rs  # send_notification (email, SMS)
│   │   ├── provenance.rs    # append_provenance (to Postgres)
│   │   ├── contract.rs      # validate_contract (calls Formspec WASM)
│   │   └── governance.rs    # evaluate_governance (deontic, delegation, etc.)
│   ├── store/
│   │   ├── mod.rs
│   │   ├── provenance.rs    # ProvenanceStore trait impl (Postgres)
│   │   └── task.rs          # TaskStore trait impl (Postgres)
│   ├── queries.rs           # Temporal query handlers (case status, provenance)
│   └── signals.rs           # Temporal signal definitions (WOS events)
└── tests/
    ├── integration/
    │   ├── happy_path.rs
    │   ├── adverse_with_appeal.rs
    │   ├── ai_agent_fallback.rs
    │   └── drift_demotion.rs
    └── fixtures/             # WOS kernel + governance docs for tests
```

---

## 3. The Temporal Workflow

The core loop: receive events, evaluate governance, execute actions, persist provenance. Temporal provides durability. wos-core provides governance decisions.

```rust
// workflow.rs

use temporal_sdk::{WfContext, Signal, ActivityOptions};
use wos_core::{
    Evaluator, CaseInstance, KernelDocument, GovernanceDocument,
    AIIntegrationDocument, EvalResult, Action, ProvenanceRecord,
};

/// Temporal signals map 1:1 to WOS events.
#[derive(Signal)]
pub enum WosSignal {
    /// External event (human action, service callback, timer expiry)
    Event(WosEvent),
    /// Formspec submission completed (triggers Coprocessor)
    SubmissionCompleted(SubmissionPayload),
    /// Human task completed (reviewer submitted their work)
    TaskCompleted(TaskCompletionPayload),
}

/// Temporal queries expose case state without mutating it.
#[derive(Query)]
pub enum WosQuery {
    /// Current case status and active states
    CaseStatus,
    /// Full provenance log for this case
    ProvenanceLog { since: Option<u64> },
    /// Current case file data
    CaseFile,
    /// Governance state (active holds, delegations, volume counters)
    GovernanceState,
}

/// One Temporal workflow instance = one WOS case instance.
///
/// Temporal handles: crash recovery, timer durability, signal queuing,
/// deterministic replay, event history.
///
/// wos-core handles: transition evaluation, governance enforcement,
/// provenance construction, explanation assembly.
#[workflow]
pub async fn wos_case_workflow(
    ctx: WfContext,
    input: CaseWorkflowInput,
) -> Result<CaseOutcome, WorkflowError> {

    // ── Phase 1: Load documents ──────────────────────────────
    // Temporal activity ensures this survives replay without re-fetching.
    let docs = ctx.execute_activity(
        load_documents,
        LoadDocsInput {
            kernel_url: input.kernel_url,
            governance_url: input.governance_url,
            ai_url: input.ai_url,
            sidecar_urls: input.sidecar_urls,
        },
        activity_options(Duration::from_secs(30)),
    ).await?;

    // ── Phase 2: Initialize evaluator and case instance ──────
    let mut evaluator = Evaluator::new(&docs.kernel);
    if let Some(ref gov) = docs.governance {
        evaluator.attach_governance(gov);
    }
    if let Some(ref ai) = docs.ai_integration {
        evaluator.attach_ai_integration(ai);
    }

    let mut instance = CaseInstance::create(
        &docs.kernel,
        input.instance_id,
        input.initial_case_file,
    );

    // ── Phase 3: If triggered by a Formspec submission, run Coprocessor ──
    if let Some(submission) = input.submission {
        let case_file = coprocessor::map_submission_to_case_file(
            &submission,
            &docs.kernel,
        )?;
        instance.apply_case_file(case_file);

        ctx.execute_activity(
            append_provenance,
            ProvenanceRecord::case_created(&instance),
            activity_options(Duration::from_secs(5)),
        ).await?;
    }

    // ── Phase 4: Event loop ──────────────────────────────────
    // This loop runs for the lifetime of the case -- potentially months.
    // Temporal persists state between events. The worker can restart.
    loop {
        let signal = ctx.wait_for_signal::<WosSignal>().await;

        match signal {
            WosSignal::Event(event) => {
                let result = process_wos_event(
                    &ctx, &mut evaluator, &mut instance, &docs, event
                ).await?;

                if instance.is_terminal() {
                    // Record final provenance and return outcome
                    ctx.execute_activity(
                        append_provenance,
                        ProvenanceRecord::workflow_completed(&instance),
                        activity_options(Duration::from_secs(5)),
                    ).await?;

                    return Ok(CaseOutcome {
                        final_state: instance.current_state().to_string(),
                        case_file: instance.case_file().clone(),
                    });
                }
            }

            WosSignal::SubmissionCompleted(submission) => {
                // A linked Formspec form was completed (e.g., RFI response,
                // appeal submission). Run Coprocessor to merge data.
                let merged = coprocessor::merge_submission(
                    &submission,
                    &instance,
                    &docs.kernel,
                )?;
                instance.apply_case_file_update(merged);

                // Fire the corresponding WOS event
                let event = coprocessor::submission_to_event(&submission);
                let result = process_wos_event(
                    &ctx, &mut evaluator, &mut instance, &docs, event
                ).await?;
            }

            WosSignal::TaskCompleted(completion) => {
                // A human task was completed. Map to WOS event.
                let event = task_completion_to_event(&completion);
                let result = process_wos_event(
                    &ctx, &mut evaluator, &mut instance, &docs, event
                ).await?;
            }
        }
    }
}
```

---

## 4. Event Processing with Governance

The heart of the integration: wos-core evaluates, Temporal executes durably.

```rust
// workflow.rs (continued)

/// Process a single WOS event: evaluate governance, fire transitions,
/// execute actions, record provenance.
async fn process_wos_event(
    ctx: &WfContext,
    evaluator: &mut Evaluator,
    instance: &mut CaseInstance,
    docs: &LoadedDocuments,
    event: WosEvent,
) -> Result<(), WorkflowError> {

    // ── Step 1: wos-core evaluates the event ─────────────────
    // This is pure computation -- no I/O. Deterministic.
    // Returns: transitions to fire, actions to execute, provenance to record.
    let eval_result = evaluator.process_event(instance, &event);

    // ── Step 2: Record transition provenance ─────────────────
    // Temporal activity: durable write to provenance store.
    // If the worker crashes here, Temporal replays up to this point.
    for record in &eval_result.provenance {
        ctx.execute_activity(
            append_provenance,
            record.clone(),
            activity_options(Duration::from_secs(5)),
        ).await?;
    }

    // ── Step 3: Execute actions ──────────────────────────────
    // Each action is a Temporal activity -- durable, retryable,
    // with idempotency keys from the WOS action definition.
    for action in &eval_result.actions {
        execute_action(ctx, evaluator, instance, docs, action).await?;
    }

    // ── Step 4: Handle timers ────────────────────────────────
    // WOS startTimer → Temporal timer. WOS cancelTimer → cancel.
    for timer_op in &eval_result.timer_operations {
        match timer_op {
            TimerOp::Start { timer_id, duration, event } => {
                // Temporal timer: survives restarts, fires on schedule.
                // When it fires, it sends a signal back to this workflow.
                ctx.timer_with_signal(
                    *duration,
                    WosSignal::Event(WosEvent::timeout(timer_id, event)),
                );
            }
            TimerOp::Cancel { timer_id } => {
                ctx.cancel_timer(timer_id);
            }
        }
    }

    // ── Step 5: Check for SLA breach ─────────────────────────
    // Governance S10.3: if task SLA expired, fire breach policy.
    if let Some(breach) = evaluator.check_sla_breach(instance) {
        execute_breach_policy(ctx, instance, &breach).await?;
    }

    Ok(())
}
```

---

## 5. Action Execution

Each WOS action type maps to a Temporal activity.

```rust
// workflow.rs (continued)

async fn execute_action(
    ctx: &WfContext,
    evaluator: &mut Evaluator,
    instance: &mut CaseInstance,
    docs: &LoadedDocuments,
    action: &Action,
) -> Result<(), WorkflowError> {

    match action.action_type.as_str() {

        // ── createTask: human work ───────────────────────────
        "createTask" => {
            let task_input = CreateTaskInput {
                case_id: instance.id().to_string(),
                task_ref: action.task_ref().unwrap().to_string(),
                assign_to: action.assign_to().map(String::from),
                // WOS Governance S10.2: assignment roles
                potential_owners: evaluator.resolve_potential_owners(
                    instance, action
                ),
                excluded_owners: evaluator.resolve_excluded_owners(
                    instance, action
                ),
                sla: evaluator.resolve_task_sla(instance, action),
            };

            // Temporal activity: writes to task store.
            // The reviewer dashboard polls the task store.
            ctx.execute_activity(
                create_human_task,
                task_input,
                activity_options(Duration::from_secs(10)),
            ).await?;

            // Start SLA timer if configured.
            if let Some(sla) = &task_input.sla {
                ctx.timer_with_signal(
                    sla.target_duration,
                    WosSignal::Event(WosEvent::sla_breach(
                        &task_input.task_ref,
                        &sla.breach_policy,
                    )),
                );
            }
        }

        // ── invokeService: external call with governance ─────
        "invokeService" => {
            let service_ref = action.service_ref().unwrap();
            let idempotency_key = action.idempotency_key();

            // If this is an AI agent invocation, run governance pipeline.
            if evaluator.is_agent_invocation(service_ref) {
                invoke_governed_agent(
                    ctx, evaluator, instance, docs, action
                ).await?;
            } else {
                // Standard service invocation.
                // Temporal activity handles retry, timeout, idempotency.
                let output = ctx.execute_activity(
                    invoke_external_service,
                    InvokeServiceInput {
                        service_ref: service_ref.to_string(),
                        input: action.data().cloned().unwrap_or_default(),
                        idempotency_key: idempotency_key.map(String::from),
                        timeout: action.timeout(),
                    },
                    ActivityOptions {
                        // Temporal retries with backoff on transient failures.
                        retry_policy: Some(RetryPolicy {
                            max_attempts: 3,
                            initial_interval: Duration::from_secs(1),
                            backoff_coefficient: 2.0,
                            ..Default::default()
                        }),
                        start_to_close_timeout: action.timeout()
                            .unwrap_or(Duration::from_secs(30)),
                        ..Default::default()
                    },
                ).await?;

                // Persist output BEFORE advancing state (WOS G3).
                // Temporal does this implicitly -- activity result is
                // in the event history before the workflow continues.
                instance.set_data(action.output_path(), output);
            }
        }

        // ── setData: case file mutation ──────────────────────
        "setData" => {
            let path = action.path().unwrap();
            let value = action.value().unwrap();
            instance.set_data(path, value.clone());
            // Mutation history recorded automatically by CaseInstance.
        }

        // ── emitEvent: fire another event into this workflow ─
        "emitEvent" => {
            let event_type = action.event_type().unwrap();
            let data = action.data().cloned();
            // Self-signal: Temporal delivers this back to our event loop.
            ctx.signal_self(WosSignal::Event(WosEvent {
                event: event_type.to_string(),
                actor_id: None,
                data,
                timestamp: now(),
            }));
        }

        // ── startTimer / cancelTimer: handled in timer section
        "startTimer" | "cancelTimer" => {
            // Handled in process_wos_event step 4.
        }

        // ── log: provenance entry ────────────────────────────
        "log" => {
            ctx.execute_activity(
                append_provenance,
                ProvenanceRecord::log_action(instance, action),
                activity_options(Duration::from_secs(5)),
            ).await?;
        }

        _ => {
            return Err(WorkflowError::UnknownActionType(
                action.action_type.clone()
            ));
        }
    }

    // Record action execution provenance.
    ctx.execute_activity(
        append_provenance,
        ProvenanceRecord::action_executed(instance, action),
        activity_options(Duration::from_secs(5)),
    ).await?;

    Ok(())
}
```

---

## 6. AI Agent Invocation with Governance

The full governance pipeline for an AI agent call: deontic evaluation, contract validation, confidence check, provenance.

```rust
// activities/governance.rs

/// Invoke an AI agent with full WOS governance.
///
/// Pipeline (AI Integration S4.6 enforcement ordering):
///   1. Check permissions (allowed fields, bounds)
///   2. Check prohibitions (forbidden outputs)
///   3. Invoke the agent
///   4. Validate output against Formspec contract (S6)
///   5. Check obligations (required content)
///   6. Check confidence floor (S7.4)
///   7. Check volume constraints (S11.1)
///   8. Apply review sampling (S11.2)
///   9. Record provenance with agent metadata
///   10. On failure: execute fallback chain (S8)
async fn invoke_governed_agent(
    ctx: &WfContext,
    evaluator: &mut Evaluator,
    instance: &mut CaseInstance,
    docs: &LoadedDocuments,
    action: &Action,
) -> Result<(), WorkflowError> {

    let agent_id = evaluator.resolve_agent(action);
    let agent_config = evaluator.agent_config(agent_id);

    // ── Pre-invocation governance ────────────────────────────

    // Resolve effective autonomy (AI Integration S5.3):
    // min(document default, agent max, action override, impact cap)
    let autonomy = evaluator.effective_autonomy(instance, agent_id, action);

    // Check volume constraints (S11.1).
    if evaluator.volume_limit_reached(instance, agent_id) {
        // Escalate to human regardless of confidence.
        return execute_fallback_chain(
            ctx, evaluator, instance, action,
            FallbackReason::VolumeLimitReached,
        ).await;
    }

    // ── Invoke the agent ─────────────────────────────────────

    let agent_output = ctx.execute_activity(
        invoke_ai_agent,
        AgentInvocationInput {
            agent_id: agent_id.to_string(),
            endpoint: agent_config.endpoint.url.clone(),
            input: evaluator.build_agent_input(instance, action),
            timeout: agent_config.endpoint.timeout,
            idempotency_key: action.idempotency_key().map(String::from),
        },
        ActivityOptions {
            start_to_close_timeout: agent_config.endpoint.timeout
                .unwrap_or(Duration::from_secs(60)),
            retry_policy: Some(RetryPolicy {
                max_attempts: 1, // Fallback chain handles retries, not Temporal.
                ..Default::default()
            }),
            ..Default::default()
        },
    ).await;

    // Agent invocation failed -- run fallback chain.
    let agent_output = match agent_output {
        Ok(output) => output,
        Err(_) => {
            return execute_fallback_chain(
                ctx, evaluator, instance, action,
                FallbackReason::InvocationFailed,
            ).await;
        }
    };

    // ── Post-invocation governance (S4.6 ordering) ───────────

    // 1. Permissions: check allowed fields and bounds.
    let permission_result = evaluator.check_permissions(
        instance, agent_id, &agent_output
    );
    if let Some(violation) = permission_result.violation() {
        return handle_deontic_violation(
            ctx, evaluator, instance, action, violation
        ).await;
    }

    // 2. Prohibitions: check forbidden outputs.
    let prohibition_result = evaluator.check_prohibitions(
        instance, agent_id, &agent_output
    );
    if let Some(violation) = prohibition_result.violation() {
        return handle_deontic_violation(
            ctx, evaluator, instance, action, violation
        ).await;
    }

    // 3. Validate against Formspec contract (S6).
    // Calls Formspec WASM for contract validation.
    let validation = ctx.execute_activity(
        validate_contract,
        ContractValidationInput {
            contract_ref: action.output_contract_ref().unwrap().to_string(),
            data: agent_output.data.clone(),
        },
        activity_options(Duration::from_secs(10)),
    ).await?;

    if !validation.valid {
        // Contract validation failed -- fallback.
        return execute_fallback_chain(
            ctx, evaluator, instance, action,
            FallbackReason::ContractValidationFailed(validation.errors),
        ).await;
    }

    // 4. Obligations: check required content.
    let obligation_result = evaluator.check_obligations(
        instance, agent_id, &agent_output
    );
    if let Some(violation) = obligation_result.violation() {
        return handle_deontic_violation(
            ctx, evaluator, instance, action, violation
        ).await;
    }

    // 5. Confidence floor (S7.4).
    if agent_output.confidence.overall < evaluator.confidence_floor(instance) {
        return handle_low_confidence(
            ctx, evaluator, instance, action, &agent_output
        ).await;
    }

    // 6. Review sampling (S11.2).
    if evaluator.should_sample_for_review(instance, agent_id) {
        // Route to human review regardless of autonomy.
        return create_review_task_for_agent_output(
            ctx, instance, action, &agent_output
        ).await;
    }

    // ── All governance passed ────────────────────────────────

    // Stamp agent provenance on each output field (S6.3).
    let annotated = evaluator.annotate_agent_provenance(
        &agent_output, agent_id,
    );

    match autonomy {
        Autonomy::Autonomous => {
            // Commit directly.
            instance.apply_agent_output(action.output_path(), &annotated);
        }
        Autonomy::Supervisory => {
            // Provisionally commit. Start review window timer.
            instance.apply_agent_output_provisional(
                action.output_path(), &annotated
            );
            let review_window = agent_config.review_window();
            ctx.timer_with_signal(
                review_window,
                WosSignal::Event(WosEvent::review_window_expired(action)),
            );
        }
        Autonomy::Assistive => {
            // Create human confirmation task. Do not commit.
            create_confirmation_task(
                ctx, instance, action, &annotated
            ).await?;
        }
        Autonomy::Manual => {
            // Agent output available as context only. No commit, no task.
            instance.store_agent_context(action.output_path(), &annotated);
        }
    }

    // Record provenance: agent invocation with full metadata.
    ctx.execute_activity(
        append_provenance,
        ProvenanceRecord::agent_invocation(
            instance, action, agent_id,
            &agent_output.confidence,
            &annotated,
        ),
        activity_options(Duration::from_secs(5)),
    ).await?;

    Ok(())
}
```

---

## 7. Fallback Chains

WOS S8: every agent must have a fallback chain that terminates in a human task or failure.

```rust
// activities/governance.rs (continued)

/// Execute the fallback chain for a failed agent invocation.
/// Chain validated at document load time -- guaranteed to terminate.
async fn execute_fallback_chain(
    ctx: &WfContext,
    evaluator: &mut Evaluator,
    instance: &mut CaseInstance,
    action: &Action,
    reason: FallbackReason,
) -> Result<(), WorkflowError> {

    let chain = evaluator.fallback_chain(action);

    for (level_idx, level) in chain.levels.iter().enumerate() {
        // Record each fallback attempt.
        ctx.execute_activity(
            append_provenance,
            ProvenanceRecord::fallback_attempt(
                instance, action, level_idx, &reason
            ),
            activity_options(Duration::from_secs(5)),
        ).await?;

        match &level.action {
            FallbackAction::Retry { max_retries, backoff, initial_interval } => {
                for attempt in 0..*max_retries {
                    let delay = calculate_backoff(
                        *backoff, *initial_interval, attempt
                    );
                    ctx.timer(delay).await;

                    match ctx.execute_activity(
                        invoke_ai_agent,
                        /* same input as original */
                        rebuild_agent_input(evaluator, instance, action),
                        activity_options(Duration::from_secs(60)),
                    ).await {
                        Ok(output) => {
                            // Re-run governance on retry output.
                            // (Recursive call with the new output.)
                            return Ok(());
                        }
                        Err(_) => continue,
                    }
                }
                // All retries exhausted. Fall through to next level.
            }

            FallbackAction::AlternateAgent { agent_ref } => {
                // Try a different agent with the same governance pipeline.
                // (Invoke governed agent with alternate agent_ref.)
                // On success: return. On failure: fall through.
            }

            FallbackAction::EscalateToHuman { task_ref } => {
                // Terminal: create a human task.
                ctx.execute_activity(
                    create_human_task,
                    CreateTaskInput {
                        case_id: instance.id().to_string(),
                        task_ref: task_ref.to_string(),
                        // Include the failure context so the human
                        // knows what the agent couldn't do.
                        context: Some(FallbackContext {
                            original_action: action.clone(),
                            reason: reason.clone(),
                            attempts: level_idx + 1,
                        }),
                        ..Default::default()
                    },
                    activity_options(Duration::from_secs(10)),
                ).await?;
                return Ok(());
            }

            FallbackAction::Fail => {
                return Err(WorkflowError::AgentFallbackExhausted(
                    reason.to_string()
                ));
            }
        }
    }

    unreachable!("Fallback chain validated at load time to terminate")
}
```

---

## 8. The Coprocessor

The handoff between Formspec submissions and WOS case instances. This is the critical gap identified in TODO.md.

```rust
// coprocessor.rs

use formspec_engine::Response;  // Formspec response struct
use wos_core::{KernelDocument, CaseInstance, CaseFile};

/// Map a completed Formspec Response to WOS case file fields.
///
/// This is the Formspec Coprocessor -- the bridge between intake and
/// governance. The mapping uses the Formspec Mapping DSL when a mapping
/// document is referenced, or direct field-name matching as fallback.
pub fn map_submission_to_case_file(
    submission: &SubmissionPayload,
    kernel: &KernelDocument,
) -> Result<CaseFile, CoprocessorError> {

    let mut case_file = CaseFile::new();

    // If the kernel's contract references a Mapping DSL document,
    // use it for bidirectional field mapping.
    if let Some(mapping_ref) = kernel.contract_mapping_ref() {
        let mapping = load_mapping(mapping_ref)?;
        case_file = mapping.apply_to_case_file(
            &submission.response_data,
            &kernel.case_file_schema(),
        )?;
    } else {
        // Direct mapping: Formspec field names match case file field names.
        for (field_name, field_def) in kernel.case_file_fields() {
            if let Some(value) = submission.response_data.get(field_name) {
                case_file.set(field_name, coerce_value(value, field_def)?);
            }
        }
    }

    Ok(case_file)
}

/// Validate the Formspec Response against its contract before firing
/// the WOS workflow event.
///
/// WOS requires: "Response is validated before the workflow event fires."
pub async fn validate_and_fire(
    ctx: &WfContext,
    submission: &SubmissionPayload,
    kernel: &KernelDocument,
    instance: &mut CaseInstance,
) -> Result<WosEvent, CoprocessorError> {

    // Validate response against the Formspec Definition contract.
    let validation = ctx.execute_activity(
        validate_contract,
        ContractValidationInput {
            contract_ref: submission.definition_ref.clone(),
            data: submission.response_data.clone(),
        },
        activity_options(Duration::from_secs(10)),
    ).await?;

    if !validation.valid {
        return Err(CoprocessorError::ResponseValidationFailed(
            validation.errors
        ));
    }

    // Map response data to case file.
    let case_file = map_submission_to_case_file(submission, kernel)?;
    instance.apply_case_file(case_file);

    // Fire the corresponding WOS event.
    Ok(WosEvent {
        event: submission.event_name.clone(), // e.g., "submitted", "rfi_response"
        actor_id: Some(submission.respondent_id.clone()),
        data: Some(submission.metadata.clone()),
        timestamp: submission.submitted_at,
    })
}

/// For rights-impacting workflows: link the Formspec Respondent Ledger
/// to WOS provenance.
///
/// The Ledger's checkpoint hashes are recorded in WOS provenance,
/// creating a tamper-evidence chain from form to workflow.
pub fn link_respondent_ledger(
    submission: &SubmissionPayload,
    instance: &CaseInstance,
) -> ProvenanceRecord {
    ProvenanceRecord::ledger_link(
        instance,
        &submission.ledger_checkpoint_hash,
        &submission.ledger_url,
    )
}
```

---

## 9. Human Task Integration

WOS tasks live in a separate store. The reviewer dashboard reads from it. Task completion signals the Temporal workflow.

```rust
// activities/human_task.rs

/// Create a human task in the task store.
/// The reviewer dashboard polls this store for the caseworker's queue.
///
/// WOS Governance S10: 8-state lifecycle, 5 assignment roles.
#[activity]
pub async fn create_human_task(
    input: CreateTaskInput,
) -> Result<TaskId, ActivityError> {

    let task = Task {
        id: TaskId::generate(),
        case_id: input.case_id,
        task_ref: input.task_ref,
        status: TaskStatus::Created,
        // Assignment per WOS Governance S10.2.
        owner: input.assign_to,
        potential_owners: input.potential_owners,
        excluded_owners: input.excluded_owners,
        sla: input.sla,
        context: input.context,
        created_at: now(),
    };

    task_store().insert(&task).await?;

    Ok(task.id)
}

// ── Called by the SaaS API when a reviewer acts ──────────────

/// Reviewer claims a task. Transitions Created/Assigned → Claimed.
/// The SaaS API calls this, which signals the Temporal workflow.
pub async fn claim_task(
    task_id: TaskId,
    actor_id: String,
    temporal_client: &TemporalClient,
) -> Result<(), ApiError> {

    let task = task_store().get(&task_id).await?;

    // WOS Governance S10.2: excludedOwner overrides all other roles.
    if task.excluded_owners.contains(&actor_id) {
        return Err(ApiError::ExcludedFromTask);
    }

    // WOS Governance S7.2: separation of duties.
    // Cannot review your own prior determination on this case.
    if task_store().actor_determined_case(&actor_id, &task.case_id).await? {
        return Err(ApiError::SeparationOfDuties);
    }

    task_store().update_status(&task_id, TaskStatus::Claimed, &actor_id).await?;

    Ok(())
}

/// Reviewer completes a task. Signals the Temporal workflow.
pub async fn complete_task(
    task_id: TaskId,
    actor_id: String,
    result: TaskResult,
    temporal_client: &TemporalClient,
) -> Result<(), ApiError> {

    let task = task_store().get(&task_id).await?;
    task_store().update_status(&task_id, TaskStatus::Completed, &actor_id).await?;

    // Signal the Temporal workflow that the task is done.
    // The workflow's event loop receives this and fires the WOS transition.
    temporal_client.signal_workflow(
        &task.case_id, // workflow ID = case ID
        WosSignal::TaskCompleted(TaskCompletionPayload {
            task_id,
            task_ref: task.task_ref,
            actor_id,
            result,
            completed_at: now(),
        }),
    ).await?;

    Ok(())
}
```

---

## 10. Temporal Query Handlers

Read-only access to case state for the dashboard and API.

```rust
// queries.rs

/// The reviewer dashboard and API query case state through Temporal.
/// Queries are read-only and do not advance the workflow.

#[query_handler(WosQuery::CaseStatus)]
fn case_status(instance: &CaseInstance) -> CaseStatusResponse {
    CaseStatusResponse {
        instance_id: instance.id().to_string(),
        status: instance.status(),
        active_states: instance.configuration().to_vec(),
        created_at: instance.created_at(),
        updated_at: instance.updated_at(),
    }
}

#[query_handler(WosQuery::CaseFile)]
fn case_file(instance: &CaseInstance) -> serde_json::Value {
    instance.case_file().to_json()
}

#[query_handler(WosQuery::GovernanceState)]
fn governance_state(instance: &CaseInstance) -> GovernanceStateResponse {
    GovernanceStateResponse {
        active_holds: instance.governance_state().active_holds.clone(),
        active_delegations: instance.governance_state().active_delegations.clone(),
        volume_counters: instance.governance_state().volume_counters.clone(),
    }
}

// Provenance is queried from the provenance store directly (not Temporal),
// because the append-only log may be large and is stored separately.
```

---

## 11. Starting a Case (SaaS API → Temporal)

How the SaaS layer starts a workflow when a Formspec submission arrives.

```typescript
// SaaS API layer (TypeScript)

import { Client } from '@temporalio/client';

// POST /api/submissions — called when a respondent submits a form
async function handleSubmission(req: Request): Promise<Response> {
  const submission = await req.json();
  const temporal = await getTemporalClient();

  // Resolve which WOS documents govern this workflow.
  const workflowConfig = await resolveWorkflowConfig(submission.definitionRef);

  // Start a Temporal workflow = create a WOS case instance.
  const handle = await temporal.workflow.start('wos_case_workflow', {
    taskQueue: 'wos-worker',
    workflowId: `case-${submission.id}`,  // Case ID = Workflow ID
    args: [{
      kernel_url: workflowConfig.kernelUrl,
      governance_url: workflowConfig.governanceUrl,
      ai_url: workflowConfig.aiUrl,
      sidecar_urls: workflowConfig.sidecarUrls,
      instance_id: `case-${submission.id}`,
      initial_case_file: null,  // Coprocessor maps this from submission
      submission: {
        response_data: submission.data,
        definition_ref: submission.definitionRef,
        respondent_id: submission.respondentId,
        submitted_at: new Date().toISOString(),
        event_name: 'submitted',
        ledger_checkpoint_hash: submission.ledgerCheckpoint,
        ledger_url: submission.ledgerUrl,
      },
    }],
  });

  return Response.json({
    caseId: handle.workflowId,
    status: 'submitted',
    message: 'Your submission has been received.',
  });
}

// GET /api/cases/:id — reviewer dashboard queries case state
async function getCaseStatus(caseId: string): Promise<CaseStatus> {
  const temporal = await getTemporalClient();
  const handle = temporal.workflow.getHandle(caseId);
  return await handle.query('CaseStatus');
}

// POST /api/tasks/:id/complete — reviewer completes a task
async function completeTask(
  taskId: string,
  actorId: string,
  result: TaskResult,
): Promise<void> {
  // This calls the Rust function from Section 9 which:
  // 1. Updates task store
  // 2. Signals the Temporal workflow
  await taskService.completeTask(taskId, actorId, result);
}
```

---

## 12. What Temporal Provides (That We Don't Build)

| Capability | Temporal mechanism | WOS equivalent if we built our own |
|-----------|-------------------|-----------------------------------|
| Crash recovery | Event sourcing + replay | InstanceStore.save() after every event, custom replay logic |
| Durable timers | Timer service, survives restarts | Separate timer database, polling, fire-on-startup sweep |
| Signal queuing | Signal buffer per workflow | EventQueue.enqueue() with durable message store |
| Exactly-once activities | Activity deduplication by ID | Idempotency key management per external service |
| Workflow visibility | ListWorkflows, DescribeWorkflow | Custom case index with status tracking |
| Multi-tenancy | Namespace isolation | Custom tenant routing layer |
| Observability | Metrics, tracing, OpenTelemetry | Custom instrumentation |

Building these from scratch: 6-12 months of infrastructure work. Using Temporal: a dependency and a `Cargo.toml` entry.

---

## 13. Testing Strategy

```text
Unit tests (wos-core, no Temporal):
  - Evaluator.process_event() with fixture documents
  - Governance checks (deontic, delegation, separation of duties)
  - Coprocessor mapping logic
  - Provenance record construction

Integration tests (wos-temporal, Temporal test server):
  - Happy path: submission → review → determination → notice
  - Adverse path: determination → notice → appeal → independent review
  - AI path: agent invocation → governance pipeline → fallback on failure
  - Timer path: SLA breach → escalation
  - Crash recovery: kill worker mid-event, restart, verify state consistency

Conformance tests (wos-conformance fixtures):
  - Run existing 194 conformance tests through the Temporal workflow
  - Verify provenance output matches expected records

End-to-end tests (full stack):
  - Formspec form submission → Temporal workflow → case in dashboard
  - Reviewer claims task → completes review → workflow advances
  - AI extraction → governance pipeline → human confirmation
```

---

## 14. Deployment

```text
Production:
  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
  │  SaaS API    │  │  Temporal    │  │  WOS Worker  │
  │  (TS, N pods)│  │  Server      │  │  (Rust, N)   │
  │              │  │  (managed or │  │              │
  │  Dashboard   │  │   self-host) │  │  wos-core    │
  │  Portal      │  │              │  │  wos-temporal │
  │  Notifier    │  │  Cassandra/  │  │  activities  │
  └──────┬───────┘  │  Postgres    │  └──────┬───────┘
         │          └──────┬───────┘         │
         │                 │                 │
  ┌──────▼─────────────────▼─────────────────▼───────┐
  │                  Postgres                         │
  │  provenance (append-only) │ tasks │ case index   │
  └──────────────────────────────────────────────────┘
```

WOS Worker replicas scale horizontally. Each processes events for any case. Temporal's task queue distributes work. No worker holds state -- all state is in Temporal's event history + the provenance store.

---

## 15. Open Questions

| Question | Options | Decision needed by |
|----------|---------|-------------------|
| Temporal Rust SDK maturity | Rust SDK is newer than Go/TS. Evaluate stability. Alternative: Go worker calling wos-core via C FFI. | Before implementation starts |
| Provenance store location | Same Postgres as tasks? Separate database? Temporal's visibility store? | Phase 1 architecture |
| Formspec WASM in Temporal worker | Load formspec-engine WASM for contract validation in Rust worker, or call SaaS API for validation? | Coprocessor design |
| Multi-tenancy model | Temporal namespace per tenant, or shared namespace with tenant ID in workflow metadata? | Phase 1 architecture |

### Resolved: Case ID Scheme

TypeID-style with UUIDv7 and tenant prefix: `{tenant}_{type}_{uuidv7_base32}`

```
linc_case_01j5e8g7k3pqx9vnm084sn02q    -- Lincoln County, case
linc_task_01j5e8h2m7rwy4bpnk0a3t05r    -- Lincoln County, task
linc_prov_01j5e8h2m7rwy4bpnk0a3t05s    -- Lincoln County, provenance record
```

**Why UUIDv7:** Time-ordered (RFC 9562). Cases sort by creation time within a tenant without a secondary index. Timestamp is extractable from the ID -- no `created_at` column needed for coarse ordering.

**Why TypeID prefix:** Self-describing. A human or log parser can distinguish `case`, `task`, `prov`, `hold`, `deleg` at a glance. Parseable: split on `_`, segment 1 = tenant, segment 2 = type, segment 3 = UUIDv7.

**Why tenant prefix:** Enables routing and isolation without a lookup. A shared Temporal namespace can partition work by tenant using workflow ID prefix. Provenance queries filter by tenant without joining a separate table. Multi-tenant Postgres can route by prefix if sharding becomes necessary.

**Temporal workflow ID = case ID.** `linc_case_01j5e8g7k3...` is both the WOS case instance identifier and the Temporal workflow ID. One identifier, zero mapping.
