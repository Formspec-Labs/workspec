//! Emit open string leaves (schema nodes with `type: string` and no enum/const/pattern).
//!
//! From `work-spec/`:
//! ```text
//! cargo run -p wos-lint --example schema_string_leaf_report -- schemas/wos-workflow.schema.json
//! cargo run -p wos-lint --example schema_string_leaf_report -- schemas/wos-workflow.schema.json --csv
//! ```
//!
//! Redirect to a file under `reports/` (gitignored) for triage archives.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

fn csv_escape(s: &str) -> String {
    if s.contains('"') || s.contains(',') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut csv = false;
    let path_args: Vec<&String> = args.iter().filter(|a| *a != "--csv").collect();
    if args.iter().any(|a| a == "--csv") {
        csv = true;
    }
    let Some(path_arg) = path_args.first() else {
        eprintln!("usage: schema_string_leaf_report <schema.json> [--csv]");
        std::process::exit(2);
    };

    let path = PathBuf::from(path_arg);
    let json = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("read {}: {e}", path.display());
        std::process::exit(1);
    });

    let root: serde_json::Value = serde_json::from_str(&json).unwrap_or_else(|e| {
        eprintln!("parse JSON: {e}");
        std::process::exit(1);
    });

    let inv = wos_lint::rules::schema_doc::inventory_string_leaves(&root);
    let rows = wos_lint::rules::schema_doc::collect_open_string_leaves(&root);

    eprintln!(
        "{}: string leaves {} (constrained {}, open {}) — listing {} open rows",
        path.display(),
        inv.string_leaves,
        inv.constrained_string_leaves,
        inv.open_string_leaves(),
        rows.len()
    );

    let stdout = io::stdout();
    let mut out = stdout.lock();
    if csv {
        writeln!(
            out,
            "pointer,def_context,has_format,has_min_length,has_max_length,description_snippet"
        )
        .ok();
        for row in &rows {
            writeln!(
                out,
                "{},{},{},{},{},{}",
                csv_escape(&row.pointer),
                csv_escape(&row.def_context),
                row.has_format,
                row.has_min_length,
                row.has_max_length,
                csv_escape(&row.description_snippet)
            )
            .ok();
        }
    } else {
        for row in &rows {
            writeln!(
                out,
                "{}\t{}\tformat={}\tminLen={}\tmaxLen={}\t{}",
                row.pointer,
                if row.def_context.is_empty() {
                    "(root)"
                } else {
                    &row.def_context
                },
                row.has_format,
                row.has_min_length,
                row.has_max_length,
                row.description_snippet
            )
            .ok();
        }
    }
}
