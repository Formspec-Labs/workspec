// Rust guideline compliant 2026-02-21

//! Executable drift tests verifying that `docs/wos-creation-guide.md` and the
//! reconciled `thoughts/plans/2026-04-17-wos-authoring-crate.md` remain aligned
//! with `wos-authoring/src/project.rs`, `wos-mcp/tool-catalog.json`, and
//! `wos-mcp/WORKFLOW-COVERAGE.json`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_text(rel_path: &str) -> String {
    let full_path = manifest_dir().join(rel_path);
    fs::read_to_string(&full_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", full_path.display()))
}

fn extract_prefixed_identifiers(text: &str, prefix: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let bytes = text.as_bytes();
    let prefix_bytes = prefix.as_bytes();
    let mut i = 0;

    while i + prefix_bytes.len() <= bytes.len() {
        if &bytes[i..i + prefix_bytes.len()] == prefix_bytes {
            // Ensure prefix starts at an identifier boundary.
            let prev_is_ident = i > 0
                && (bytes[i - 1].is_ascii_alphanumeric()
                    || bytes[i - 1] == b'_'
                    || bytes[i - 1] == b'$');
            if !prev_is_ident {
                let mut end = i + prefix_bytes.len();
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                if end > i + prefix_bytes.len() {
                    let followed_by_path_sep =
                        end + 1 < bytes.len() && bytes[end] == b':' && bytes[end + 1] == b':';
                    if !followed_by_path_sep {
                        found.insert(text[i..end].to_string());
                    }
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }

    found
}

fn extract_pub_fn_names(rust_src: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in rust_src.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("pub fn ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                names.insert(name);
            }
        }
    }
    names
}

fn extract_relative_markdown_links(markdown: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = markdown;
    while let Some(idx) = rest.find("](") {
        let after = &rest[idx + 2..];
        if let Some(close) = after.find(')') {
            let target = after[..close].trim();
            if !target.starts_with("http://")
                && !target.starts_with("https://")
                && !target.starts_with('#')
            {
                let path_part = target.split('#').next().unwrap_or(target).trim();
                if !path_part.is_empty() {
                    links.push(path_part.to_string());
                }
            }
            rest = &after[close + 1..];
        } else {
            break;
        }
    }
    links
}

#[test]
fn creation_guide_mcp_tools_match_tool_catalog() {
    let guide = read_text("../../docs/wos-creation-guide.md");
    let catalog_raw = read_text("../wos-mcp/tool-catalog.json");
    let catalog: Value = serde_json::from_str(&catalog_raw).expect("valid tool-catalog.json");

    let catalog_tools: BTreeSet<String> = catalog["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().expect("tool name").to_string())
        .collect();

    let guide_tools = extract_prefixed_identifiers(&guide, "wos_");

    for mentioned in &guide_tools {
        assert!(
            catalog_tools.contains(mentioned),
            "docs/wos-creation-guide.md mentions MCP tool `{mentioned}` which is not in crates/wos-mcp/tool-catalog.json"
        );
    }

    for tool in &catalog_tools {
        assert!(
            guide_tools.contains(tool),
            "crates/wos-mcp/tool-catalog.json tool `{tool}` is missing from docs/wos-creation-guide.md"
        );
    }
}

#[test]
fn creation_guide_documents_all_edit_workflow_actions_and_coverage() {
    let guide = read_text("../../docs/wos-creation-guide.md");
    let catalog_raw = read_text("../wos-mcp/tool-catalog.json");
    let coverage_raw = read_text("../wos-mcp/WORKFLOW-COVERAGE.json");

    let catalog: Value = serde_json::from_str(&catalog_raw).expect("valid tool-catalog.json");
    let coverage: Value = serde_json::from_str(&coverage_raw).expect("valid WORKFLOW-COVERAGE.json");

    let edit_tool = catalog["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|t| t["name"].as_str() == Some("wos_edit_workflow"))
        .expect("wos_edit_workflow must exist in tool-catalog.json");

    let actions: BTreeSet<String> = edit_tool["x-wos-actions"]
        .as_array()
        .expect("x-wos-actions array")
        .iter()
        .map(|a| a["name"].as_str().expect("action name").to_string())
        .collect();

    assert_eq!(
        actions.len(),
        10,
        "wos_edit_workflow must declare all 10 actions"
    );

    for action in &actions {
        let backticked = format!("`{action}`");
        assert!(
            guide.contains(&backticked),
            "docs/wos-creation-guide.md must document wos_edit_workflow action {backticked}"
        );
    }

    // Also verify every `wos_edit_workflow.<action>` in WORKFLOW-COVERAGE.json is a valid action.
    let fields = coverage["fields"].as_object().expect("coverage fields");
    for (field_name, entry) in fields {
        for editor in entry["editors"].as_array().expect("editors array") {
            let editor_str = editor.as_str().expect("editor string");
            if let Some(action_name) = editor_str.strip_prefix("wos_edit_workflow.") {
                assert!(
                    actions.contains(action_name),
                    "WORKFLOW-COVERAGE.json field `{field_name}` references unknown action `{action_name}`"
                );
            }
        }
    }
}

#[test]
fn creation_guide_wos_project_methods_exist_in_project_rs() {
    let guide = read_text("../../docs/wos-creation-guide.md");
    let project_rs = read_text("src/project.rs");

    let pub_fns = extract_pub_fn_names(&project_rs);
    let referenced = extract_prefixed_identifiers(&guide, "WosProject::");

    assert!(
        !referenced.is_empty(),
        "docs/wos-creation-guide.md must reference WosProject::<method> symbols"
    );

    for symbol in &referenced {
        let method = symbol
            .strip_prefix("WosProject::")
            .expect("prefixed with WosProject::");
        assert!(
            pub_fns.contains(method),
            "docs/wos-creation-guide.md references `{symbol}`, which is not a `pub fn` in crates/wos-authoring/src/project.rs"
        );
    }
}

#[test]
fn creation_guide_relative_links_resolve_on_disk() {
    let guide_path = manifest_dir().join("../../docs/wos-creation-guide.md");
    let guide_dir = guide_path
        .parent()
        .expect("docs/wos-creation-guide.md has parent directory");
    let guide = read_text("../../docs/wos-creation-guide.md");

    let links = extract_relative_markdown_links(&guide);
    assert!(
        !links.is_empty(),
        "docs/wos-creation-guide.md must contain relative links to canonical artifacts"
    );

    for required in [
        "../crates/wos-mcp/README.md",
        "../crates/wos-mcp/tool-catalog.json",
        "../crates/wos-mcp/WORKFLOW-COVERAGE.json",
        "../thoughts/plans/2026-04-17-wos-authoring-crate.md",
    ] {
        assert!(
            links.iter().any(|l| l == required),
            "docs/wos-creation-guide.md must link to `{required}`"
        );
    }

    for link in links {
        let resolved = guide_dir.join(&link);
        assert!(
            Path::new(&resolved).exists(),
            "relative link `{link}` in docs/wos-creation-guide.md does not exist at {}",
            resolved.display()
        );
    }
}

#[test]
fn historical_plan_contains_reconciliation_section_and_actual_sources() {
    let plan = read_text("../../thoughts/plans/2026-04-17-wos-authoring-crate.md");

    assert!(
        plan.contains("RECONCILED WITH SHIPPED IMPLEMENTATION"),
        "2026-04-17-wos-authoring-crate.md must carry the reconciliation status header"
    );
    assert!(
        plan.contains("docs/wos-creation-guide.md"),
        "2026-04-17-wos-authoring-crate.md must link to docs/wos-creation-guide.md"
    );

    for src_file in [
        "src/lib.rs",
        "src/command.rs",
        "src/diagnostics.rs",
        "src/native_schema.rs",
        "src/project.rs",
        "src/raw.rs",
    ] {
        assert!(
            plan.contains(src_file),
            "reconciled plan must reference shipped module `{src_file}`"
        );
        let full_path = manifest_dir().join(src_file);
        assert!(
            full_path.exists(),
            "shipped module `{src_file}` must exist on disk at {}",
            full_path.display()
        );
    }
}
