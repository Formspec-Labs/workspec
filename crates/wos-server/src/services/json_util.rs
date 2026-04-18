//! Shared JSON traversal helpers used by multiple services.

/// Walk `value` by a dotted path (`a.b.c`) and stringify the leaf.
///
/// Returns `None` when a segment is missing or the leaf is `null`.
/// Non-string primitives are stringified; objects and arrays yield their
/// debug serialization via `to_string` so callers can compare.
pub fn lookup_dotted(value: &serde_json::Value, path: &str) -> Option<String> {
    let mut current = value;
    for seg in path.split('.') {
        current = current.get(seg)?;
    }
    match current {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Null => None,
        other => Some(other.to_string()),
    }
}
