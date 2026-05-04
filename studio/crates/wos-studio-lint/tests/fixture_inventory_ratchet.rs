//! Ratchet: every `load_workspace("...")` call site across the lint
//! crate's `src/` AND `tests/` trees references a fixture file that
//! exists on disk. Catches typos / dangling fixture refs early,
//! before the test suite reports them as a panic in the loader.
//!
//! Mirror of `tests/raw_access_ratchet.rs` (boundary guard) — same
//! "walk the file, scan for the call shape, validate" idiom.

use std::fs;
use std::path::{Path, PathBuf};

/// Hard floor on the number of resolved `load_workspace(...)` call
/// sites across the crate. The ratchet's intent is to catch
/// regressions in the *fixture-externalization* posture introduced
/// by D-wave: if someone reverts a swath of fixtures back inline,
/// the call count drops below this floor and the ratchet fails.
/// The original `found > 0` only caught a full revert of every
/// fixture; this floor surfaces partial reverts. Bump downward only
/// with explicit justification.
const FOUND_FLOOR: usize = 36;

#[test]
fn every_load_workspace_call_resolves_to_an_existing_fixture() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures_root = manifest_dir.join("fixtures");

    let mut sources: Vec<PathBuf> = Vec::new();
    collect_rs_files(&manifest_dir.join("src"), &mut sources);
    collect_rs_files(&manifest_dir.join("tests"), &mut sources);

    let mut missing: Vec<String> = Vec::new();
    let mut found: usize = 0;
    for src in &sources {
        let text = match fs::read_to_string(src) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let needle = "load_workspace(\"";
        let mut cursor = 0;
        while let Some(start) = text[cursor..].find(needle) {
            let abs_start = cursor + start + needle.len();
            let Some(end_rel) = text[abs_start..].find('"') else {
                break;
            };
            let rel = &text[abs_start..abs_start + end_rel];
            let path = fixtures_root.join(rel);
            if !path.is_file() {
                missing.push(format!("{} → {rel}", src.display()));
            } else {
                found += 1;
            }
            cursor = abs_start + end_rel;
        }
    }

    assert!(
        missing.is_empty(),
        "load_workspace() call sites reference {} missing fixture(s): {missing:?} \
         (resolved {found} OK; fixtures_root={})",
        missing.len(),
        fixtures_root.display(),
    );
    assert!(
        found >= FOUND_FLOOR,
        "load_workspace() call count {found} < FOUND_FLOOR {FOUND_FLOOR}; \
         this signals partial revert of the fixture-externalization posture \
         (D-wave). Investigate before lowering the floor."
    );
}

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_rs_files(&p, out);
        } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            // The ratchet itself mentions the literal call shape in
            // its doc-comment; skip it to avoid self-scan.
            if p.file_name().and_then(|s| s.to_str())
                == Some("fixture_inventory_ratchet.rs")
            {
                continue;
            }
            out.push(p);
        }
    }
}
