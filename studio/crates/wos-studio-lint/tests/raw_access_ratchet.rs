//! Ratchet: count of `.raw` access call sites in the lint engine's
//! `src/` tree (`wos-studio-lint/src/`) does NOT grow.
//!
//! Scope, load-bearing: this ratchet ONLY scans
//! `wos-studio-lint/src/`. `wos-studio-compiler/` is intentionally
//! out of scope (compiler-tier raw use is documented in-line in
//! that crate). When a new `.raw` site appears under
//! `wos-studio-lint/src/`, the right move is one of:
//!
//! 1. Re-check whether a typed accessor on `wos-studio-model`
//!    already exposes the field (most do).
//! 2. If no typed accessor exists, add one in `wos-studio-model`
//!    first, then consume it from the lint rule. The ratchet does
//!    not permit growth — STUDIO-DEFER-001 is now Closed; new
//!    untyped reaches are not part of any active deferral.
//!
//! Mirror of `tests/api_surface.rs` (boundary guard) — same
//! "walk-the-tree, count, fail-on-grow" shape.

use std::fs;
use std::path::Path;

const BASELINE: usize = 8;

#[test]
fn workspace_document_raw_access_does_not_grow() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let count = count_raw_access(&root);
    assert!(
        count <= BASELINE,
        "raw access count {count} > baseline {BASELINE}; \
         add typed accessor on wos-studio-model OR amend STUDIO-DEFER-001 \
         with justification, then bump BASELINE here."
    );
}

fn count_raw_access(dir: &Path) -> usize {
    let mut total = 0;
    for entry in fs::read_dir(dir).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            total += count_raw_access(&path);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            total += count_in_file(&path);
        }
    }
    total
}

fn count_in_file(path: &Path) -> usize {
    let text = fs::read_to_string(path).expect("read source file");
    let mut n = 0;
    let bytes = text.as_bytes();
    let pat = b".raw";
    let mut i = 0;
    while i + pat.len() <= bytes.len() {
        if &bytes[i..i + pat.len()] == pat {
            // Word-boundary on the right: next byte must NOT be an
            // ident char (matches \b semantics for `.raw\b`).
            let next = bytes.get(i + pat.len()).copied();
            let is_word_char = matches!(next, Some(c) if c.is_ascii_alphanumeric() || c == b'_');
            if !is_word_char {
                n += 1;
                i += pat.len();
                continue;
            }
        }
        i += 1;
    }
    n
}
