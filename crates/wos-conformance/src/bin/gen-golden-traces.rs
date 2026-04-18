// @filedesc One-time binary to generate golden traces for T3 fixtures.
// Generate golden expected traces for the T3 conformance fixture set.
//
// Run from the workspace root:
//   cargo run -p wos-conformance --bin gen-golden-traces
//
// Reads every JSON fixture from `crates/wos-conformance/fixtures/`,
// runs it through `run_fixture_with_trace`, and writes the resulting
// trace to `fixtures/conformance/expected-traces/<slug>.json`.

use std::path::Path;

fn main() {
    // CARGO_MANIFEST_DIR points to the crate root; workspace root is two levels up.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = Path::new(manifest_dir)
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap();

    let fixtures_dir = workspace_root.join("crates/wos-conformance/fixtures");
    let output_dir = workspace_root.join("fixtures/conformance/expected-traces");

    std::fs::create_dir_all(&output_dir)
        .expect("failed to create expected-traces directory");

    let mut generated = 0usize;
    let mut failed = 0usize;

    let mut entries: Vec<_> = std::fs::read_dir(&fixtures_dir)
        .expect("fixtures directory not found")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "json")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let name = path.file_name().unwrap().to_str().unwrap();
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {name}: {e}"));

        // T3 fixtures reference documents relative to the workspace root.
        let base_dir = workspace_root.to_str().unwrap();

        print!("  {name} ... ");
        match wos_conformance::run_fixture_with_trace(&json, base_dir) {
            Ok((_result, trace)) => {
                let slug = wos_conformance::slugify(&trace.fixture_id);
                let out_path = output_dir.join(format!("{slug}.json"));
                let pretty = serde_json::to_string_pretty(&trace)
                    .expect("trace serialization failed");
                std::fs::write(&out_path, pretty)
                    .unwrap_or_else(|e| panic!("write {}: {e}", out_path.display()));
                println!("ok  ({})", out_path.file_name().unwrap().to_str().unwrap());
                generated += 1;
            }
            Err(e) => {
                println!("FAILED: {e}");
                failed += 1;
            }
        }
    }

    println!();
    println!("generated: {generated}  failed: {failed}");
    if failed > 0 {
        std::process::exit(1);
    }
}
