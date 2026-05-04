// Rust guideline compliant 2026-05-02

//! Forbidden-import guard: every Studio crate's source MUST consume the
//! parent (`wos-core`, `wos-lint`, `wos-runtime`) ONLY through their
//! `studio_api` published modules.
//!
//! ## Why grep, not AST
//!
//! - Zero dependencies (no `syn` / `cargo-deny` / etc.)
//! - Cheap to run and trivial to reason about
//! - Catches the practical violation: someone writing `use
//!   wos_core::model::kernel::Action`. False positives (e.g., the string
//!   appearing inside a doc-comment) are filtered with a simple
//!   line-shape check.
//!
//! ## Forms recognized
//!
//! The walker calls [`is_forbidden_import_line`] on each line. That helper
//! handles every legal Rust import-statement form:
//!
//! - `use wos_core::studio_api::*;` — allowed (allowlist hit)
//! - `use wos_core::model::kernel::Action;` — forbidden
//! - `pub use wos_core::studio_api::Guard;` — allowed
//! - `pub use wos_core::model::kernel::*;` — forbidden (re-export laundering)
//! - `pub(crate) use wos_core::model::Foo;` — forbidden
//! - `pub(in path) use wos_core::Foo;` — forbidden
//! - `use wos_core::studio_api as api;` — allowed (rename)
//!
//! ## Known bypass surfaces (NOT currently caught)
//!
//! 1. **Re-export laundering inside `studio_api`.** A future contributor
//!    widening `wos_core::studio_api` to re-export internals would route
//!    around this guard. Solution lives at the source crate, not here.
//! 2. **`build.rs` macro-driven imports.** The walker reads `build.rs`
//!    files, but generated code via `quote!` / `include!()` of files
//!    outside `crates/*/src/` evades string match.
//! 3. **Type-alias inheritance.** If `studio_api` re-exports a type whose
//!    public methods return non-`studio_api` parent types, callers can
//!    hold those types transitively without writing a forbidden `use`
//!    line.
//!
//! These are documented gaps; tightening them requires `syn`-based AST
//! analysis that the per-crate model doesn't justify today.
//!
//! ## Topology
//!
//! Single workspace-wide guard test in this file. The walker scans every
//! `.rs` file under `studio/crates/*/` (including each crate's `tests/`
//! directory — Studio integration tests MUST honor the same boundary as
//! production code, since the boundary is a repo-extraction precondition,
//! not a test-style preference).

use std::path::{Path, PathBuf};

const PARENT_CRATES: &[&str] = &["wos_core", "wos_lint", "wos_runtime"];

#[test]
fn studio_crates_only_import_via_studio_api() {
    let workspace = studio_workspace_root();
    let crates_dir = workspace.join("crates");

    let mut violations: Vec<String> = Vec::new();
    for source_file in collect_rs_files(&crates_dir) {
        // The guard's own source can mention forbidden patterns in
        // comments / string literals (this very file does); skip it.
        if source_file.ends_with("api_surface.rs") {
            continue;
        }
        let content = std::fs::read_to_string(&source_file)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", source_file.display()));
        for (lineno, line) in content.lines().enumerate() {
            if let Some(parent) = is_forbidden_import_line(line) {
                let rel = source_file
                    .strip_prefix(&workspace)
                    .unwrap_or(&source_file)
                    .display()
                    .to_string();
                violations.push(format!(
                    "  {rel}:{}: [{parent}] {}",
                    lineno + 1,
                    line.trim_start(),
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Studio crate(s) reach into wos-core / wos-lint / wos-runtime \
         internals — must consume only via `studio_api`:\n{}\n\n\
         Add the type to the appropriate `studio_api` module if it's \
         genuinely needed at the Studio tier; do NOT widen the import.",
        violations.join("\n"),
    );
}

/// Return `Some(parent)` if `line` is an import-statement form that
/// reaches into a parent crate outside `studio_api`. Returns `None` for
/// allowed lines, non-import lines, and unrelated lines.
///
/// Public-to-the-crate so the red-team test below can exercise it
/// directly.
pub(crate) fn is_forbidden_import_line(line: &str) -> Option<&'static str> {
    // Strip leading whitespace.
    let mut s = line.trim_start();

    // Optional `pub` or `pub(...)` visibility prefix. Skip past it +
    // the mandatory whitespace that follows.
    if let Some(rest) = s.strip_prefix("pub") {
        let rest = match rest.strip_prefix('(') {
            Some(after_open) => {
                // `pub(crate)` / `pub(super)` / `pub(in path)` — skip to
                // matching `)`. Vis paren groups don't nest in Rust syntax.
                match after_open.find(')') {
                    Some(idx) => &after_open[idx + 1..],
                    None => return None, // malformed; can't be a use stmt
                }
            }
            None => rest,
        };
        // After `pub` or `pub(...)` we require whitespace then `use `.
        let rest_trimmed = rest.trim_start();
        if rest_trimmed.len() == rest.len() {
            // No whitespace between `pub` and the next token; not an
            // import statement.
            return None;
        }
        s = rest_trimmed;
    }

    // Now must begin with `use ` (with whitespace).
    let after_use = s.strip_prefix("use")?;
    let after_use = after_use.trim_start();
    // `use` must be followed by whitespace; `strip_prefix` already
    // consumed the keyword, and `trim_start` ensures we advanced.
    if after_use.len() == s.len().saturating_sub(3) {
        return None;
    }

    // Match against each parent crate root.
    for parent in PARENT_CRATES {
        let prefix = format!("{parent}::");
        if let Some(after_root) = after_use.strip_prefix(&prefix) {
            // Allowed iff the next path segment is `studio_api`. The
            // segment terminates at `::`, `;`, ` ` (e.g., ` as`), or end.
            let next_segment_end = after_root
                .find(|c: char| c == ':' || c == ';' || c.is_whitespace() || c == '{')
                .unwrap_or(after_root.len());
            let segment = &after_root[..next_segment_end];
            if segment == "studio_api" {
                return None;
            }
            return Some(*parent);
        }
        // Also catch `use wos_core;` (bare-crate import — equally bad).
        if let Some(after_bare) = after_use.strip_prefix(*parent) {
            let next = after_bare.chars().next();
            if matches!(next, Some(';') | Some(' ') | None) {
                return Some(*parent);
            }
        }
    }
    None
}

fn collect_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip only target/ and hidden dirs. Tests directories ARE
            // walked — Studio integration tests honor the same boundary
            // as production code (the boundary is a repo-extraction
            // precondition, not a test-style preference).
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name.starts_with('.') {
                continue;
            }
            walk(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == "rs")
        {
            out.push(path);
        }
    }
}

fn studio_workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = studio/crates/wos-studio-types
    // Two parents up = studio/
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("studio workspace root is two levels above CARGO_MANIFEST_DIR")
        .to_path_buf()
}

// ============================================================================
// Red-team coverage: lock the predicate against regression.
// ============================================================================

#[test]
fn predicate_allows_studio_api_imports() {
    for line in [
        "use wos_core::studio_api::*;",
        "use wos_core::studio_api;",
        "use wos_core::studio_api::Guard;",
        "use wos_lint::studio_api::LintDiagnostic;",
        "use wos_runtime::studio_api::DurableRuntime;",
        "    use wos_core::studio_api::{KernelDocument, Action};",
        "use wos_core::studio_api as api;",
        "pub use wos_core::studio_api::*;",
        "pub use wos_core::studio_api::Guard;",
        "pub(crate) use wos_lint::studio_api::LintDiagnostic;",
        "pub(super) use wos_runtime::studio_api;",
        "pub(in crate::foo) use wos_core::studio_api::Action;",
    ] {
        assert_eq!(
            is_forbidden_import_line(line),
            None,
            "should be allowed: {line:?}"
        );
    }
}

#[test]
fn predicate_rejects_known_bypass_patterns() {
    let cases: &[(&str, &str)] = &[
        // Plain `use` — already caught by old guard.
        ("use wos_core::model::kernel::Action;", "wos_core"),
        ("use wos_lint::rules::tier1;", "wos_lint"),
        ("use wos_runtime::Evaluator;", "wos_runtime"),
        ("    use wos_core::eval::EvalContext;", "wos_core"),
        // `pub use` — re-export laundering. Old guard MISSED these.
        ("pub use wos_core::model::kernel::*;", "wos_core"),
        ("pub use wos_core::model::kernel::Action;", "wos_core"),
        ("pub use wos_lint::rules::tier1::*;", "wos_lint"),
        ("pub use wos_runtime::Evaluator;", "wos_runtime"),
        // `pub(crate) use` / `pub(super) use` / `pub(in path) use`.
        ("pub(crate) use wos_core::model::Foo;", "wos_core"),
        ("pub(super) use wos_lint::diagnostic::Tier;", "wos_lint"),
        ("pub(in crate::foo) use wos_runtime::custody::Hook;", "wos_runtime"),
        // Group-imports that mix allowed + forbidden — the forbidden
        // form (`use wos_core::{...}`) lacks `studio_api::` so it fires
        // (correct: groups must each go through studio_api).
        ("use wos_core::{model::kernel::Action, studio_api::Guard};", "wos_core"),
        // Bare-crate import.
        ("use wos_core;", "wos_core"),
        // Indented forms.
        ("    pub use wos_core::model::kernel::*;", "wos_core"),
        ("\tuse wos_lint::rules::tier1;", "wos_lint"),
    ];
    for (line, expected_parent) in cases {
        assert_eq!(
            is_forbidden_import_line(line),
            Some(*expected_parent),
            "should be forbidden: {line:?}",
        );
    }
}

#[test]
fn predicate_ignores_non_imports() {
    for line in [
        // Comments.
        "// use wos_core::model::kernel::Action;",
        "/// Doc comment mentioning use wos_core::model.",
        "//! Module-level mention of use wos_core::Foo.",
        "/* use wos_core::Foo; */",
        // String literals.
        "    let s = \"use wos_core::model::Foo;\";",
        // Unrelated `use` lines.
        "use std::path::Path;",
        "use serde::Serialize;",
        "use wos_studio_types::kernel::*;",
        // `use` inside identifier (not at start).
        "    fn use_it() {}",
        // Empty / whitespace.
        "",
        "    ",
        // Macro that mentions but doesn't import.
        "    println!(\"use wos_core::Foo\");",
    ] {
        assert_eq!(
            is_forbidden_import_line(line),
            None,
            "should not flag: {line:?}"
        );
    }
}
