//! `lint_document` failure variants that downstream synth crates match on.

use wos_lint::LintError;

/// Well-formed JSON with no `$wos*` marker is the dominant non-convergence
/// mode of LLM-authored documents; it must surface as the dedicated
/// [`LintError::MissingMarker`] variant, never as a substring of `Parse`.
#[test]
fn well_formed_json_without_wos_marker_is_missing_marker() {
    let plausible_but_unmarked = r#"{
        "title": "Purchase order approval",
        "lifecycle": {"initialState": "draft", "states": {}}
    }"#;

    let err = wos_lint::lint_document(plausible_but_unmarked)
        .expect_err("lint must fail when the document has no $wos* marker");

    assert!(matches!(err, LintError::MissingMarker), "got: {err:?}");
}

/// Malformed JSON stays `Parse` so the missing-marker channel stays specific.
#[test]
fn malformed_json_is_parse_not_missing_marker() {
    let err = wos_lint::lint_document("not json at all")
        .expect_err("lint must fail on non-JSON input");

    assert!(matches!(err, LintError::Parse(_)), "got: {err:?}");
}
