# wos-synth

The CLI uses the same catalog declarations, argument/result validation, canonical
static checks, and exact-candidate replay handlers as MCP. The provider-independent
loop receives complete evidence through `ToolContext::check_candidate`.

```sh
wos-synth generate --problem problem.md --checks checks.json --output workflow.json
wos-synth check-candidate --candidate workflow.json --checks checks.json
wos-synth dry-run --problem problem.md --checks checks.json --output workflow.json
wos-synth dry-run --problem problem.md --draft-only --output draft.json
wos-synth explain workflow.synth-trace.json
```

`generate` reads `ANTHROPIC_API_KEY` at the host boundary. `dry-run` uses a
deterministic schema-valid typed-event workflow through the same host and tools.
Checked generation requires independently selected scenarios before any provider
call. Saved-candidate checking reports absent scenarios as required replay not-run.

A checks file contains native tool arguments without `project_id`:

```json
{
  "validation": {"scope": "project", "resolved_dependencies": []},
  "replays": [
    {
      "id": "reviewed-decision",
      "arguments": {
        "fixture_json": "{\"id\":\"decision-control\",\"rule\":\"control\",\"description\":\"Reviewed decision\",\"documents\":{},\"event_sequence\":[{\"event\":\"decide\",\"actor\":\"reviewer\"}],\"expected_transitions\":[{\"from\":\"review\",\"target\":\"done\",\"event\":\"decide\"}]}",
        "clock_start_ms": 0,
        "resolved_dependencies": [],
        "required_observations": ["configuration"],
        "expected_observations": []
      }
    }
  ]
}
```

Scenario IDs must be nonempty and unique. Their arguments use the maintained
`wos_run_conformance` declaration; static arguments use `wos_lint`. Unknown or
invalid fields fail through those declarations. Static and runtime dependencies
remain separate selections: static `document_json` is a native JSON value;
runtime `document_json` is a string whose exact bytes remain selected.

Every candidate uses `WosProject::from_native_document` and a fresh local
`ProjectRegistry`. Incomplete object drafts receive canonical validation; malformed
text/nonobjects retain their original intake error and explicit not-run checks.
Every configured scenario gets a result slot, even after another scenario fails.
Preparation errors retain their available detail; completed native refusal reports
retain identity, attempts, differences, initialization and trace without recoding.

Checked convergence consumes the shared native decision from `wos_check_authoring`.
The assessment retains that exact decision and its reasons as `authoring_decision`,
alongside the unchanged owner reports. Stored runtime event forms can replay
successfully while failing the authoring schema; both results remain visible.
`--draft-only` requests static checks alone and produces an explicit `Draft` outcome.
Its checks file must omit replay selections. These outcomes describe requested
synthesis checks; publication readiness and hosted execution need their own evidence.

Generation always saves the full trace beside the document as
`<stem>.synth-trace.json`; `--trace` selects another path. Failed/exhausted runs save
the last candidate and full evidence too. `check-candidate` prints the assessment
as JSON. A provider failure during repair produces `Interrupted`, saves the last
checked candidate and existing evidence, and records the original error separately.
Empty model output remains empty in evidence and fails native intake.
Exit 0 means requested checks passed or an explicitly labeled static-only
draft completed; exit 2 means candidate checks failed or remain incomplete; exit 1
means an input, provider, or host error. Argument parsing uses clap's error status.

`generate_with` accepts actual parsed `GenerateArgs` and an injected `Prompter` for
embedded callers. It uses the same concrete tool adapter as normal generation.
Replay evidence reaches the saved trace; replay-driven repair prompting belongs
to fs-563r.
