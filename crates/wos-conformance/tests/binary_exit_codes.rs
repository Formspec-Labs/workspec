// Rust guideline compliant 2026-04-18

//! Exit-code contract tests for `wos-conformance-explain`,
//! `wos-conformance-diff`, and `wos-rule-coverage` binaries.
//!
//! Pins the three defined exit codes for each binary:
//!   0  happy path (explain: pass fixture; diff: traces match)
//!   1  divergence (explain: fixture failed; diff: traces diverge)
//!   2  usage error (bad args / missing operands)
//!
//! Uses `assert_cmd` to spawn the real compiled binaries so the test
//! exercises the full exit-code mapping in main(), not just the library.

use assert_cmd::Command;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// T3 fixtures live in `crates/wos-conformance/fixtures/`, not `tests/fixtures/`.
fn t3_fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn passing_fixture() -> PathBuf {
    t3_fixtures_dir().join("K-011-determinism.json")
}

fn failing_fixture() -> PathBuf {
    // K-001 is a lint-negative fixture: 0 transitions, outcome = fail.
    t3_fixtures_dir().join("K-001-negative-final-transitions.json")
}

fn golden_trace(name: &str) -> PathBuf {
    workspace_root()
        .join("fixtures/conformance/expected-traces")
        .join(name)
}

// ── wos-conformance-explain ──────────────────────────────────────────────────

/// Passing fixture → exit 0.
#[test]
fn explain_passing_fixture_exits_0() {
    Command::cargo_bin("wos-conformance-explain")
        .unwrap()
        .arg(passing_fixture())
        .arg("--base-dir")
        .arg(workspace_root())
        .assert()
        .code(0);
}

/// Failing fixture → exit 1 (trace still rendered).
#[test]
fn explain_failing_fixture_exits_1() {
    Command::cargo_bin("wos-conformance-explain")
        .unwrap()
        .arg(failing_fixture())
        .arg("--base-dir")
        .arg(workspace_root())
        .assert()
        .code(1);
}

/// No arguments → exit 2 (usage error).
#[test]
fn explain_no_args_exits_2() {
    Command::cargo_bin("wos-conformance-explain")
        .unwrap()
        .assert()
        .code(2);
}

// ── wos-conformance-diff ─────────────────────────────────────────────────────

/// Golden trace matches fresh run → exit 0 and stdout "OK".
#[test]
fn diff_matching_traces_exits_0_and_prints_ok() {
    let golden = golden_trace("K-011-determinism.json");
    Command::cargo_bin("wos-conformance-diff")
        .unwrap()
        .arg(golden)
        .arg(passing_fixture())
        .arg("--base-dir")
        .arg(workspace_root())
        .assert()
        .code(0)
        .stdout("OK\n");
}

/// Mutated golden (wrong state) vs fresh run → exit 1 and stdout contains "DIVERGENCE".
#[test]
fn diff_diverging_traces_exits_1_and_prints_divergence() {
    // Write a mutated golden to a tempfile: flip state_after of step 0 to "rejected".
    let golden_path = golden_trace("K-011-determinism.json");
    let golden_json = std::fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("cannot read golden trace: {e}"));
    let mut golden: serde_json::Value =
        serde_json::from_str(&golden_json).expect("parse golden");
    golden["steps"][0]["stateAfter"] = serde_json::json!("rejected");
    golden["steps"][0]["expectedStateAfter"] = serde_json::json!("rejected");

    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    std::fs::write(tmp.path(), serde_json::to_string_pretty(&golden).unwrap())
        .expect("write mutated golden");

    let output = Command::cargo_bin("wos-conformance-diff")
        .unwrap()
        .arg(tmp.path())
        .arg(passing_fixture())
        .arg("--base-dir")
        .arg(workspace_root())
        .assert()
        .code(1)
        .get_output()
        .clone();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("DIVERGENCE"),
        "stdout must contain DIVERGENCE: {stdout}"
    );
}

/// No arguments → exit 2 (usage error).
#[test]
fn diff_no_args_exits_2() {
    Command::cargo_bin("wos-conformance-diff")
        .unwrap()
        .assert()
        .code(2);
}

// ── wos-rule-coverage ─────────────────────────────────────────────────────────

/// --help → exit 0 (user-requested, not a usage error).
#[test]
fn rule_coverage_help_exits_0() {
    Command::cargo_bin("wos-rule-coverage")
        .unwrap()
        .arg("--help")
        .assert()
        .code(0);
}

/// -h → exit 0 (short alias for --help).
#[test]
fn rule_coverage_help_short_exits_0() {
    Command::cargo_bin("wos-rule-coverage")
        .unwrap()
        .arg("-h")
        .assert()
        .code(0);
}

/// Unknown flag → exit 2 (usage error).
#[test]
fn rule_coverage_unknown_flag_exits_2() {
    Command::cargo_bin("wos-rule-coverage")
        .unwrap()
        .arg("--not-a-real-flag")
        .assert()
        .code(2);
}
