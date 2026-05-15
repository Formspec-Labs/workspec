// Rust guideline compliant 2026-02-21

//! Compatibility re-exports for shared stack TypeID helpers.
//!
//! TypeID grammar and minting now live in `stack-common-typeid`. This module
//! keeps the existing `wos_core::typeid::*` path stable while later refactor
//! steps move consumers to the shared crate directly.

#[doc(inline)]
pub use stack_common_typeid::{
    AI_PREFIX, ASSURANCE_PREFIX, CASE_PREFIX, DEFAULT_TENANT, GOVERNANCE_PREFIX, PROCESS_PREFIX,
    PROVENANCE_PREFIX, ParsedTypeId, TenantResolutionError, TypeIdScope, extract_tenant,
    is_case_ledger_id, is_process_id, is_valid_record_type_id, is_valid_tenant, is_valid_type_id,
    mint_ai_id, mint_assurance_id, mint_case_ledger_id, mint_governance_id, mint_process_id,
    mint_provenance_id, mint_type_id, parse_case_ledger_id, parse_process_id, parse_type_id,
    tenant, tenant_from_env_value, tenant_from_env_value_strict, tenant_strict,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_module_exposes_shared_typeid_helpers() {
        let case_id = mint_case_ledger_id();
        assert!(is_case_ledger_id(&case_id));
        assert_eq!(parse_case_ledger_id(&case_id), Some(case_id.as_str()));
        assert_eq!(extract_tenant(&case_id), Some(DEFAULT_TENANT));

        let case_urn = format!("urn:wos:{case_id}");
        assert_eq!(parse_case_ledger_id(&case_urn), Some(case_id.as_str()));
        assert!(!is_valid_type_id(&case_urn, Some(CASE_PREFIX)));
        assert!(!is_case_ledger_id(&case_urn));
        assert_eq!(extract_tenant(&case_urn), None);

        let process_id = mint_process_id();
        let process_urn = format!("urn:wos:{process_id}");
        assert_eq!(parse_process_id(&process_urn), Some(process_id.as_str()));
        assert!(!is_valid_type_id(&process_urn, Some(PROCESS_PREFIX)));
        assert!(!is_process_id(&process_urn));

        let record_id = mint_provenance_id();
        assert!(is_valid_record_type_id(&record_id));
        assert!(!is_valid_record_type_id(&format!("urn:wos:{record_id}")));
    }
}
