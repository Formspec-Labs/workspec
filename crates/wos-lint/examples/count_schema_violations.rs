//! Count SCHEMA-DOC-001 violations in a schema file. Example helper for
//! iterative backfill work — lists violations grouped by pointer.

use std::{env, fs};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <schema-path> [--list]", args[0]);
        std::process::exit(1);
    }
    let path = &args[1];
    let list = args.iter().any(|a| a == "--list");
    let json = fs::read_to_string(path).expect("read schema");
    let diags = wos_lint::lint_schema(&json).expect("lint");
    println!("{}: {} violations", path, diags.len());
    if list {
        for d in &diags {
            println!("  {} -- {}", d.path, d.message);
        }
    }
}
