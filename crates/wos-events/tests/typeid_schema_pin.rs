// Rust guideline compliant 2026-02-21

//! Pins minted IDs to the published API patterns in
//! `schemas/api/_common.schema.json` (ADR 0092 D-1): `WosResourceUrn`,
//! `CaseLedgerPathId` and `ProcessPathId`. If the minter's byte format drifts
//! from the schema, this test fails.

use serde_json::{Value, json};
use stack_common_typeid::testing::fixture_minter;

const COMMON_SCHEMA: &str = include_str!("../../../schemas/api/_common.schema.json");

fn validator_for(def: &str) -> jsonschema::Validator {
    let common: Value = serde_json::from_str(COMMON_SCHEMA).expect("common schema parses");
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": format!("#/$defs/{def}"),
        "$defs": common["$defs"],
    });
    jsonschema::validator_for(&schema).expect("schema compiles")
}

#[test]
fn minted_ids_match_the_published_resource_urn_pattern() {
    let urn = validator_for("WosResourceUrn");
    for tenant in ["acme", "a", "tenant-test", "sba-poc", "a1-b2"] {
        let minter = stack_common_typeid::TypeIdMinter::fixture(tenant);
        let mut ids = vec![
            minter.mint_case_ledger_id(),
            minter.mint_process_id(),
            minter.mint_provenance_id(),
            minter.mint_governance_id(),
            minter.mint_ai_id(),
            minter.mint_assurance_id(),
            minter.mint_vendor("x-acme-vendor").expect("vendor family"),
        ];
        ids.sort();
        for id in ids {
            let resource = format!("urn:wos:{id}");
            assert!(urn.is_valid(&json!(resource)), "{resource}");
        }
    }
}

#[test]
fn case_and_process_ids_match_their_path_id_patterns() {
    let minter = fixture_minter();
    let case_path = validator_for("CaseLedgerPathId");
    let process_path = validator_for("ProcessPathId");
    let case = minter.mint_case_ledger_id();
    let process = minter.mint_process_id();
    assert!(case_path.is_valid(&json!(case)));
    assert!(case_path.is_valid(&json!(format!("urn:wos:{case}"))));
    assert!(process_path.is_valid(&json!(process)));
    assert!(process_path.is_valid(&json!(format!("urn:wos:{process}"))));
    assert!(!case_path.is_valid(&json!(process)));
    assert!(!process_path.is_valid(&json!(case)));
}

#[test]
fn schema_rejects_ids_the_minter_refuses_to_mint() {
    let urn = validator_for("WosResourceUrn");
    let minter = fixture_minter();
    // The minter has no way to build an empty or non-vendor custom family.
    assert!(minter.mint_vendor("").is_err());
    assert!(minter.mint_vendor("custom").is_err());
    let tail = minter.mint_case_ledger_id();
    let tail = tail.rsplit_once('_').expect("typeid shape").1;
    assert!(!urn.is_valid(&json!(format!("urn:wos:acme__{tail}"))));
    assert!(!urn.is_valid(&json!(format!("urn:wos:acme_custom_{tail}"))));
}
