//! Print SCHEMA-DOC-001 findings and string-leaf inventory for a JSON Schema file.
//!
//! Exit status: `0` when there are no SCHEMA-DOC-001 violations, or when
//! `--no-fail` is passed (violations are still printed). `1` when violations
//! exist without `--no-fail`, or on read/parse/`lint_schema` failure. `2` for
//! usage errors (missing path or unknown flags).
//!
//! Pass `--no-fail` to exit `0` even when SCHEMA-DOC-001 reports violations
//! (useful for local triage while schemas are still being fixed).
//!
//! From the `work-spec/` directory:
//! ```text
//! cargo run -p wos-lint --example count_schema_violations -- schemas/wos-workflow.schema.json
//! cargo run -p wos-lint --example count_schema_violations -- schemas/wos-workflow.schema.json --list
//! cargo run -p wos-lint --example count_schema_violations -- schemas/wos-workflow.schema.json --list --no-fail
//! ```

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let mut args = env::args().skip(1);
    let Some(path_arg) = args.next() else {
        eprintln!(
            "usage: count_schema_violations <schema.json> [--list] [--no-fail]"
        );
        std::process::exit(2);
    };
    let mut list = false;
    let mut no_fail = false;
    for a in args {
        match a.as_str() {
            "--list" => list = true,
            "--no-fail" => no_fail = true,
            _ => {
                eprintln!("unknown argument: {a}");
                std::process::exit(2);
            }
        }
    }

    let path = PathBuf::from(&path_arg);
    let json = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("read {}: {e}", path.display());
        std::process::exit(1);
    });

    let root: serde_json::Value = serde_json::from_str(&json).unwrap_or_else(|e| {
        eprintln!("parse JSON: {e}");
        std::process::exit(1);
    });

    let inv = wos_lint::rules::schema_doc::inventory_string_leaves(&root);
    println!(
        "{}: string leaves {} (enum/const/pattern: {}, open: {})",
        path.display(),
        inv.string_leaves,
        inv.constrained_string_leaves,
        inv.open_string_leaves()
    );

    let diagnostics = wos_lint::lint_schema(&json).unwrap_or_else(|e| {
        eprintln!("lint_schema: {e}");
        std::process::exit(1);
    });
    println!("SCHEMA-DOC-001 violations: {}", diagnostics.len());
    if list {
        for diagnostic in &diagnostics {
            println!("{} — {}", diagnostic.path, diagnostic.message);
        }
    }
    if !diagnostics.is_empty() && !no_fail {
        std::process::exit(1);
    }
}
