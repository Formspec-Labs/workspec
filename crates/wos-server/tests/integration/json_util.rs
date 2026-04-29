//! Unit tests for [`wos_server::services::json_util`].

use serde_json::json;
use wos_server::services::json_util::lookup_dotted;

#[test]
fn lookup_dotted_string_leaf() {
    let v = json!({ "a": { "b": "leaf" } });
    assert_eq!(lookup_dotted(&v, "a.b"), Some("leaf".into()));
}

#[test]
fn lookup_dotted_null_is_none() {
    let v = json!({ "a": { "b": null } });
    assert_eq!(lookup_dotted(&v, "a.b"), None);
}

#[test]
fn lookup_dotted_missing_segment_is_none() {
    let v = json!({ "a": {} });
    assert_eq!(lookup_dotted(&v, "a.b.c"), None);
}

#[test]
fn lookup_dotted_number_stringifies() {
    let v = json!({ "n": 42 });
    assert_eq!(lookup_dotted(&v, "n"), Some("42".into()));
}
