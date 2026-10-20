# Current generic governance owner controls

`governance_policy.rs` executes the maintained native runtime using the existing
controlled task adapter and service ports. It binds retained accepted Responses,
actual evaluator Facts IDs and snapshots, original Mapping captures, actor/task
associations and callback provenance. It does not qualify a live host, recipient
receipt, document-signing trust configuration or provider delivery.

Run the ordinary controls with `cargo nextest run --offline --locked -p
wos-runtime --test governance_policy`. The authored D1 artifact qualification is
ignored by default and is **not default CI coverage**. Select it explicitly:

```sh
export WOS_GOVERNANCE_ARTIFACT_DIR=/absolute/path/to/authored/d1-full/v0.4.6
cargo nextest run --offline --locked -p wos-runtime --test governance_policy \
  -E 'test(generated_d1_native_policy_qualifies_exact_workflow_delivery_and_mapping_selection)' \
  --run-ignored only
```

Explicit selection fails without its directory. This check qualifies the exact
policy/Workflow/Delivery declaration through the owner; it is not an executed D1
journey. Full signed D1 journeys have their own server evidence.

Set `WOS_RUNTIME_GOVERNANCE_PROBE_DIR` to retain actual before/after native records
and effect counts from the drain and task-Mapping refusal controls. Guard-removal
experiments must preserve these unchanged assertions and restore exact source
bytes before final verification.
