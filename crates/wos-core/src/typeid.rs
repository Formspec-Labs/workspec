// Rust guideline compliant 2026-02-21

//! Stack-local TypeID minting and validation helpers.
//!
//! WOS adopts the ADR-0061 identifier format
//! `{tenant}_{type}_{uuidv7_base32}` for case-ledger, process, and
//! authored-record identities. This module keeps the implementation local to
//! `wos-core` until the stack decides whether a shared utility crate is worth
//! the coordination cost.

use uuid::{Uuid, Version};

/// Default deployment tenant for local development and tests.
pub const DEFAULT_TENANT: &str = "default";

/// Reserved WOS TypeID prefix for durable case ledgers.
pub const CASE_PREFIX: &str = "case";

/// Reserved WOS TypeID prefix for workflow runtime processes.
pub const PROCESS_PREFIX: &str = "process";

/// Reserved WOS TypeID prefix for Kernel provenance records.
pub const PROVENANCE_PREFIX: &str = "prov";

/// Reserved WOS TypeID prefix for governance records.
pub const GOVERNANCE_PREFIX: &str = "gov";

/// Reserved WOS TypeID prefix for AI records.
pub const AI_PREFIX: &str = "ai";

/// Reserved WOS TypeID prefix for assurance records.
pub const ASSURANCE_PREFIX: &str = "assurance";

const CROCKFORD_LOWER: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// Resolves the tenant string from a raw environment value without reading
/// process environment. Invalid or missing values fall back to
/// [`DEFAULT_TENANT`].
#[must_use]
pub fn tenant_from_env_value(raw: Option<&str>) -> String {
    match raw {
        Some(value) if is_valid_tenant(value) => value.to_string(),
        _ => DEFAULT_TENANT.to_string(),
    }
}

/// Returns the active TypeID tenant.
///
/// The runtime can override the default tenant with `WOS_TYPEID_TENANT`.
#[must_use]
pub fn tenant() -> String {
    tenant_from_env_value(std::env::var("WOS_TYPEID_TENANT").ok().as_deref())
}

/// Mints a new case ledger identifier.
#[must_use]
pub fn mint_case_ledger_id() -> String {
    mint_type_id(&tenant(), CASE_PREFIX)
}

/// Mints a new workflow runtime process identifier.
#[must_use]
pub fn mint_process_id() -> String {
    mint_type_id(&tenant(), PROCESS_PREFIX)
}

/// Returns whether `value` is a raw case-ledger TypeID.
#[must_use]
pub fn is_case_ledger_id(value: &str) -> bool {
    is_valid_type_id(value, Some(CASE_PREFIX))
}

/// Returns whether `value` is a raw process TypeID.
#[must_use]
pub fn is_process_id(value: &str) -> bool {
    is_valid_type_id(value, Some(PROCESS_PREFIX))
}

/// Parses a case-ledger TypeID from a raw TypeID or WOS URN.
#[must_use]
pub fn parse_case_ledger_id(value: &str) -> Option<&str> {
    parse_prefixed_type_id(value, CASE_PREFIX)
}

/// Parses a process TypeID from a raw TypeID or WOS URN.
#[must_use]
pub fn parse_process_id(value: &str) -> Option<&str> {
    parse_prefixed_type_id(value, PROCESS_PREFIX)
}

/// Mints a new Kernel provenance identifier.
#[must_use]
pub fn mint_provenance_id() -> String {
    mint_type_id(&tenant(), PROVENANCE_PREFIX)
}

/// Mints a new identifier for the given family prefix.
#[must_use]
pub fn mint_type_id(tenant: &str, family_prefix: &str) -> String {
    let encoded_uuid = encode_uuid_v7(Uuid::now_v7());
    format!("{tenant}_{family_prefix}_{encoded_uuid}")
}

/// Mints a new governance-record identifier.
#[must_use]
pub fn mint_governance_id() -> String {
    mint_type_id(&tenant(), GOVERNANCE_PREFIX)
}

/// Mints a new AI-record identifier.
#[must_use]
pub fn mint_ai_id() -> String {
    mint_type_id(&tenant(), AI_PREFIX)
}

/// Mints a new assurance-record identifier.
#[must_use]
pub fn mint_assurance_id() -> String {
    mint_type_id(&tenant(), ASSURANCE_PREFIX)
}

/// Returns whether `value` is a valid **custody `recordId`** per Kernel
/// custody-hook-encoding (ADR-0061 wire); `RecordTypeId` rules were formerly
/// in `wos-custody-hook-encoding.schema.json` and now live in author-time
/// `schemas/wos-workflow.schema.json` / runtime `wos-runtime::custody` only:
/// reserved families `prov`, `gov`, `ai`, `assurance`, or a vendor family
/// `x-{label}(?:-{label})+` with the same tenant and UUIDv7 tail rules as
/// [`is_valid_type_id`].
#[must_use]
pub fn is_valid_record_type_id(value: &str) -> bool {
    if !is_valid_type_id(value, None) {
        return false;
    }
    let mut parts = value.split('_');
    let _tenant = parts.next();
    let Some(family) = parts.next() else {
        return false;
    };
    matches!(family, "prov" | "gov" | "ai" | "assurance") || is_valid_vendor_record_family(family)
}

/// Vendor `recordId` middle segment: `x-` then at least two hyphen-separated
/// labels, each starting with a lowercase ASCII letter.
fn is_valid_vendor_record_family(family: &str) -> bool {
    let Some(after_x) = family.strip_prefix("x-") else {
        return false;
    };
    if after_x.is_empty() {
        return false;
    }
    let labels: Vec<&str> = after_x.split('-').collect();
    if labels.len() < 2 {
        return false;
    }
    labels.iter().all(|label| {
        let mut chars = label.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !first.is_ascii_lowercase() {
            return false;
        }
        chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    })
}

/// Returns whether `value` is a valid WOS TypeID for the optional prefix.
#[must_use]
pub fn is_valid_type_id(value: &str, expected_prefix: Option<&str>) -> bool {
    let mut parts = value.split('_');
    let Some(tenant) = parts.next() else {
        return false;
    };
    let Some(prefix) = parts.next() else {
        return false;
    };
    let Some(tail) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    if !is_valid_tenant(tenant) {
        return false;
    }
    if expected_prefix.is_some_and(|expected| prefix != expected) {
        return false;
    }
    decode_uuid_v7_tail(tail).is_some()
}

fn parse_prefixed_type_id<'a>(value: &'a str, expected_prefix: &str) -> Option<&'a str> {
    let type_id = value.strip_prefix("urn:wos:").unwrap_or(value);
    is_valid_type_id(type_id, Some(expected_prefix)).then_some(type_id)
}

fn encode_uuid_v7(uuid: Uuid) -> String {
    let mut value = u128::from_be_bytes(*uuid.as_bytes());
    let mut output = [0u8; 26];
    for slot in output.iter_mut().rev() {
        *slot = CROCKFORD_LOWER[(value & 0x1f) as usize];
        value >>= 5;
    }
    String::from_utf8(output.to_vec()).expect("crockford alphabet is valid UTF-8")
}

/// Decodes the TypeID Crockford tail; **lowercase only** per TypeID
/// normalization (see <https://typeid.io/>).
fn decode_uuid_v7_tail(tail: &str) -> Option<Uuid> {
    if tail.len() != 26 {
        return None;
    }
    let mut value = 0u128;
    for byte in tail.bytes() {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'h' => 10 + (byte - b'a'),
            b'j'..=b'k' => 18 + (byte - b'j'),
            b'm'..=b'n' => 20 + (byte - b'm'),
            b'p'..=b't' => 22 + (byte - b'p'),
            b'v'..=b'z' => 27 + (byte - b'v'),
            _ => return None,
        };
        value = (value << 5) | u128::from(digit);
    }
    let bytes = value.to_be_bytes();
    let uuid = Uuid::from_bytes(bytes);
    if uuid.get_version() != Some(Version::SortRand) {
        return None;
    }
    if !matches!(uuid.get_variant(), uuid::Variant::RFC4122) {
        return None;
    }
    Some(uuid)
}

/// Validate a tenant identifier per ADR 0068 D-1.1 grammar
/// `^[a-z][a-z0-9-]{0,62}$` (RFC 1035 DNS-label-compatible: ≤63 chars,
/// lowercase-kebab; tenants map 1:1 to DNS subdomains for free).
pub fn is_valid_tenant(value: &str) -> bool {
    // RFC 1035 DNS-label-compatible upper bound (ADR 0068 D-1.1).
    if value.len() > 63 {
        return false;
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

/// Extracts the tenant prefix from a TypeID string.
///
/// Returns `None` when the value is not a valid TypeID (fewer than three
/// `_`-separated segments, invalid tenant grammar, or invalid UUIDv7 tail).
#[must_use]
pub fn extract_tenant(type_id: &str) -> Option<&str> {
    let mut parts = type_id.split('_');
    let tenant = parts.next()?;
    let _prefix = parts.next()?;
    let _tail = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if is_valid_tenant(tenant) {
        Some(tenant)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minted_case_ledger_ids_match_reserved_shape() {
        let minted = mint_case_ledger_id();
        assert!(is_valid_type_id(&minted, Some(CASE_PREFIX)));
        assert!(is_case_ledger_id(&minted));
        assert_eq!(parse_case_ledger_id(&minted), Some(minted.as_str()));
        let urn = format!("urn:wos:{minted}");
        assert_eq!(parse_case_ledger_id(&urn), Some(minted.as_str()));
    }

    #[test]
    fn minted_process_ids_match_reserved_shape() {
        let minted = mint_process_id();
        assert!(is_valid_type_id(&minted, Some(PROCESS_PREFIX)));
        assert!(is_process_id(&minted));
        assert_eq!(parse_process_id(&minted), Some(minted.as_str()));
        let urn = format!("urn:wos:{minted}");
        assert_eq!(parse_process_id(&urn), Some(minted.as_str()));
        assert_eq!(parse_case_ledger_id(&minted), None);
    }

    #[test]
    fn minted_provenance_ids_match_reserved_shape() {
        let minted = mint_provenance_id();
        assert!(is_valid_type_id(&minted, Some(PROVENANCE_PREFIX)));
    }

    #[test]
    fn minted_other_reserved_ids_match_reserved_shapes() {
        assert!(is_valid_type_id(
            &mint_governance_id(),
            Some(GOVERNANCE_PREFIX)
        ));
        assert!(is_valid_type_id(&mint_ai_id(), Some(AI_PREFIX)));
        assert!(is_valid_type_id(
            &mint_assurance_id(),
            Some(ASSURANCE_PREFIX)
        ));
    }

    #[test]
    fn validator_rejects_wrong_prefix() {
        let minted = mint_type_id("tenant", GOVERNANCE_PREFIX);
        assert!(!is_valid_type_id(&minted, Some(PROVENANCE_PREFIX)));
    }

    #[test]
    fn record_type_id_accepts_reserved_and_vendor_families() {
        assert!(is_valid_record_type_id(&mint_provenance_id()));
        assert!(is_valid_record_type_id(&mint_governance_id()));
        let prov = mint_provenance_id();
        let tail = prov.rsplit_once('_').expect("typeid shape").1;
        let vendor = format!("default_x-acme-vendor-test_{tail}");
        assert!(is_valid_record_type_id(&vendor));
    }

    #[test]
    fn record_type_id_rejects_case_family_and_unknown_middle() {
        assert!(!is_valid_record_type_id(&mint_case_ledger_id()));
        let prov = mint_provenance_id();
        let tail = prov.rsplit_once('_').expect("typeid shape").1;
        assert!(!is_valid_record_type_id(&format!("default_custom_{tail}")));
        assert!(!is_valid_record_type_id(&format!("default_x-short_{tail}")));
    }

    #[test]
    fn valid_tenant_from_env_value_is_used_for_minting() {
        let tenant = tenant_from_env_value(Some("acme-corp"));
        let id = mint_type_id(&tenant, CASE_PREFIX);
        assert!(
            id.starts_with("acme-corp_case_"),
            "expected acme-corp tenant, got {id}"
        );
        assert!(is_valid_type_id(&id, Some(CASE_PREFIX)));
        let process_id = mint_type_id(&tenant, PROCESS_PREFIX);
        assert!(
            process_id.starts_with("acme-corp_process_"),
            "expected acme-corp tenant, got {process_id}"
        );
        assert!(is_valid_type_id(&process_id, Some(PROCESS_PREFIX)));
    }

    #[test]
    fn invalid_tenant_env_value_falls_back_to_default() {
        assert_eq!(tenant_from_env_value(Some("INVALID")), DEFAULT_TENANT);
    }

    #[test]
    fn missing_tenant_env_value_falls_back_to_default() {
        assert_eq!(tenant_from_env_value(None), DEFAULT_TENANT);
    }

    #[test]
    fn is_valid_tenant_accepts_lowercase_and_digits() {
        assert!(is_valid_tenant("abc"));
        assert!(is_valid_tenant("a1b2"));
        assert!(is_valid_tenant("sba-poc"));
        assert!(!is_valid_tenant(""));
        assert!(!is_valid_tenant("ABC"));
        assert!(!is_valid_tenant("1abc"));
        assert!(!is_valid_tenant("a_b"));
    }

    #[test]
    fn is_valid_tenant_enforces_dns_label_length_cap() {
        // ADR 0068 D-1.1: ≤63 chars (RFC 1035 DNS-label limit).
        let max_len_tenant = "a".to_string() + &"b".repeat(62);
        assert_eq!(max_len_tenant.len(), 63);
        assert!(
            is_valid_tenant(&max_len_tenant),
            "63 chars MUST be accepted (the boundary)"
        );

        let too_long = "a".to_string() + &"b".repeat(63);
        assert_eq!(too_long.len(), 64);
        assert!(
            !is_valid_tenant(&too_long),
            "64 chars MUST be rejected (RFC 1035 boundary; ADR 0068 D-1.1)"
        );
    }
}
