// Rust guideline compliant 2026-02-21

//! Rule coverage computation for the unified WOS rule registry.
//!
//! Walks both the `wos-lint` (T1/T2) and `wos-conformance` (T3) rule
//! registries, aggregates per-tier and per-category statistics, identifies
//! orphaned fixture files, and surfaces Draft rules that have discoverable
//! fixtures ready for promotion.
//!
//! Public entry point: [`compute_coverage`].

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use wos_lint::{Graduation, RuleMetadata};

/// Top-level result returned by [`compute_coverage`].
#[derive(Debug, Clone, Serialize)]
pub struct CoverageReport {
    /// Version string of the WOS spec (currently the crate version).
    pub version: String,
    /// Counts broken down by graduation tier.
    pub summary: GraduationSummary,
    /// Per-category counts.
    pub by_category: BTreeMap<String, CategoryCounts>,
    /// Fixture files under the fixture root that no rule references.
    pub orphaned_fixtures: Vec<String>,
    /// Draft rules that have at least one discoverable fixture.
    pub promotion_candidates: Vec<PromotionCandidate>,
    /// Flat list of all merged rules (for `--verbose`).
    pub rules: Vec<RuleEntry>,
    /// Duplicate rule ids found across the two registries.
    pub duplicate_ids: Vec<String>,
}

/// Breakdown across the four graduation tiers.
#[derive(Debug, Clone, Serialize)]
pub struct GraduationSummary {
    pub total: usize,
    pub draft: usize,
    pub tested: usize,
    pub stable: usize,
    pub load_bearing: usize,
}

/// Per-category counts by graduation tier.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CategoryCounts {
    pub draft: usize,
    pub tested: usize,
    pub stable: usize,
    pub load_bearing: usize,
}

impl CategoryCounts {
    fn total(&self) -> usize {
        self.draft + self.tested + self.stable + self.load_bearing
    }
}

/// A Draft rule that has a discoverable fixture — a candidate for promotion.
#[derive(Debug, Clone, Serialize)]
pub struct PromotionCandidate {
    pub rule_id: String,
    pub evidence: Vec<DiscoveredEvidence>,
}

/// One piece of evidence linking a Draft rule to a fixture file.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredEvidence {
    /// Relative path from workspace root to the fixture file.
    pub fixture_path: String,
    /// How the link was discovered.
    pub match_kind: EvidenceMatchKind,
}

/// How a fixture was discovered to relate to a rule.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceMatchKind {
    /// Fixture filename contains the rule id (case-insensitive, hyphens normalised).
    FilenameStem,
    /// Fixture JSON contains a top-level `"rule"` field equal to the rule id.
    RuleField,
}

/// One rule as it appears in the flattened merged registry.
#[derive(Debug, Clone, Serialize)]
pub struct RuleEntry {
    pub id: String,
    pub tier: String,
    pub graduation: String,
    pub category: String,
    pub summary: String,
    pub fixtures: Vec<String>,
}

/// A previously-recorded summary snapshot used in `--strict` mode.
#[derive(Debug, Deserialize)]
pub struct StoredSummary {
    pub total: usize,
    pub draft: usize,
    pub tested: usize,
    pub stable: usize,
    pub load_bearing: usize,
}

/// Compute a full coverage report.
///
/// `registries` is a slice of static slices — pass `&[lint_rules, conformance_rules]`
/// to merge both registries.
///
/// `fixtures_dir` should be the workspace-level `fixtures/` directory.
/// Pass `None` to skip orphan detection and promotion candidate discovery
/// (useful for hermetic unit tests that do not need filesystem access).
pub fn compute_coverage(
    registries: &[&'static [RuleMetadata]],
    fixtures_dir: Option<&Path>,
) -> CoverageReport {
    // Collect into a Vec<RuleMetadata> (RuleMetadata is Copy) so the rest of
    // the code is lifetime-free: it only works with owned values.
    let (merged_rules, duplicate_ids) = merge_registries(registries);

    let summary = build_graduation_summary(&merged_rules);
    let by_category = build_category_breakdown(&merged_rules);
    let rules = build_rule_entries(&merged_rules);

    let (orphaned_fixtures, promotion_candidates) = match fixtures_dir {
        Some(dir) => discover_fixtures(dir, &merged_rules),
        None => (Vec::new(), Vec::new()),
    };

    CoverageReport {
        version: env!("CARGO_PKG_VERSION").to_string(),
        summary,
        by_category,
        orphaned_fixtures,
        promotion_candidates,
        rules,
        duplicate_ids,
    }
}

/// Merge two or more rule registries into a de-duplicated `Vec<RuleMetadata>`.
///
/// Duplicate ids are collected separately. The first occurrence wins;
/// subsequent duplicates are flagged but not inserted.
fn merge_registries(
    registries: &[&'static [RuleMetadata]],
) -> (Vec<RuleMetadata>, Vec<String>) {
    let mut seen: HashSet<&'static str> = HashSet::new();
    let mut merged: Vec<RuleMetadata> = Vec::new();
    let mut duplicates: Vec<String> = Vec::new();

    for registry in registries {
        for rule in *registry {
            if seen.insert(rule.id) {
                merged.push(*rule);
            } else {
                duplicates.push(rule.id.to_string());
            }
        }
    }

    (merged, duplicates)
}

fn build_graduation_summary(rules: &[RuleMetadata]) -> GraduationSummary {
    let mut draft = 0usize;
    let mut tested = 0usize;
    let mut stable = 0usize;
    let mut load_bearing = 0usize;

    for rule in rules {
        match rule.graduation {
            Graduation::Draft => draft += 1,
            Graduation::Tested => tested += 1,
            Graduation::Stable => stable += 1,
            Graduation::LoadBearing => load_bearing += 1,
        }
    }

    GraduationSummary {
        total: rules.len(),
        draft,
        tested,
        stable,
        load_bearing,
    }
}

fn build_category_breakdown(rules: &[RuleMetadata]) -> BTreeMap<String, CategoryCounts> {
    let mut map: BTreeMap<String, CategoryCounts> = BTreeMap::new();

    for rule in rules {
        let category = rule_category(rule.id);
        let counts = map.entry(category).or_default();
        match rule.graduation {
            Graduation::Draft => counts.draft += 1,
            Graduation::Tested => counts.tested += 1,
            Graduation::Stable => counts.stable += 1,
            Graduation::LoadBearing => counts.load_bearing += 1,
        }
    }

    map
}

fn build_rule_entries(rules: &[RuleMetadata]) -> Vec<RuleEntry> {
    rules
        .iter()
        .map(|rule| RuleEntry {
            id: rule.id.to_string(),
            tier: format!("{:?}", rule.tier),
            graduation: graduation_label(rule.graduation),
            category: rule_category(rule.id),
            summary: rule.summary.to_string(),
            fixtures: rule.fixtures.iter().map(|f| f.to_string()).collect(),
        })
        .collect()
}

/// Derive the category from a rule id by taking the prefix up to (but not
/// including) the first segment that is purely numeric.
///
/// Examples:
/// - `"K-001"` → `"K"`
/// - `"G-051"` → `"G"`
/// - `"AI-041"` → `"AI"`
/// - `"K-EXT-002"` → `"K-EXT"`
/// - `"SCHEMA-DOC-001"` → `"SCHEMA-DOC"`
pub fn rule_category(id: &str) -> String {
    let parts: Vec<&str> = id.split('-').collect();
    let prefix_parts: Vec<&str> = parts
        .iter()
        .copied()
        .take_while(|part| !part.chars().all(|c| c.is_ascii_digit()))
        .collect();
    if prefix_parts.is_empty() {
        return id.to_string();
    }
    prefix_parts.join("-")
}

fn graduation_label(g: Graduation) -> String {
    match g {
        Graduation::Draft => "draft".to_string(),
        Graduation::Tested => "tested".to_string(),
        Graduation::Stable => "stable".to_string(),
        Graduation::LoadBearing => "load_bearing".to_string(),
    }
}

/// Walk `fixtures_dir` recursively, collect all `.json` files, and return
/// (orphans, promotion_candidates).
///
/// Orphans: files not referenced by any rule's `fixtures` list, excluding
/// the `conformance/expected-traces/` subtree.
///
/// Promotion candidates: Draft rules whose id matches a fixture file by
/// filename stem, or whose id appears as the value of a top-level `"rule"`
/// JSON field in a fixture file.
fn discover_fixtures(
    fixtures_dir: &Path,
    rules: &[RuleMetadata],
) -> (Vec<String>, Vec<PromotionCandidate>) {
    let all_fixtures = collect_fixture_paths(fixtures_dir);

    // Workspace root is the parent of the fixtures dir (e.g. `.../wos-spec`).
    let workspace_root = fixtures_dir
        .parent()
        .unwrap_or(fixtures_dir)
        .to_path_buf();

    // Build an index of all paths referenced by any rule (normalised to
    // forward-slashes for cross-platform suffix matching).
    let referenced_paths: HashSet<String> = rules
        .iter()
        .flat_map(|rule| rule.fixtures.iter().copied())
        .map(normalize_path)
        .collect();

    // Compute workspace-relative path for each fixture file.
    let fixture_rel_paths: Vec<String> = all_fixtures
        .iter()
        .filter_map(|abs| {
            abs.strip_prefix(&workspace_root)
                .ok()
                .map(|rel| normalize_path(rel.to_string_lossy().as_ref()))
        })
        .collect();

    // Orphan detection: fixture path not mentioned by any rule, and not
    // inside the expected-traces exclusion subtree.
    let mut orphaned_fixtures: Vec<String> = fixture_rel_paths
        .iter()
        .filter(|rel| {
            !is_expected_traces_path(rel)
                && !path_is_referenced(rel, &referenced_paths)
        })
        .cloned()
        .collect();
    orphaned_fixtures.sort();

    // Promotion candidates: Draft rules with a matching fixture.
    let draft_rules: Vec<&RuleMetadata> = rules
        .iter()
        .filter(|rule| matches!(rule.graduation, Graduation::Draft))
        .collect();

    let mut promotion_map: HashMap<String, Vec<DiscoveredEvidence>> = HashMap::new();

    for fixture_abs in &all_fixtures {
        if is_expected_traces_path_buf(fixture_abs) {
            continue;
        }
        let rel_path = fixture_abs
            .strip_prefix(&workspace_root)
            .ok()
            .map(|p| normalize_path(p.to_string_lossy().as_ref()))
            .unwrap_or_default();

        let stem = file_stem_normalized(fixture_abs);
        let rule_field = read_rule_field(fixture_abs);

        for rule in &draft_rules {
            let rule_id_norm = normalize_id_for_matching(rule.id);

            // Match 1: filename stem contains the normalised rule id.
            if !stem.is_empty() && stem.contains(&rule_id_norm) {
                promotion_map
                    .entry(rule.id.to_string())
                    .or_default()
                    .push(DiscoveredEvidence {
                        fixture_path: rel_path.clone(),
                        match_kind: EvidenceMatchKind::FilenameStem,
                    });
                continue;
            }

            // Match 2: fixture JSON has `"rule": "<rule-id>"` at the top level.
            if rule_field.as_deref() == Some(rule.id) {
                promotion_map
                    .entry(rule.id.to_string())
                    .or_default()
                    .push(DiscoveredEvidence {
                        fixture_path: rel_path.clone(),
                        match_kind: EvidenceMatchKind::RuleField,
                    });
            }
        }
    }

    let mut promotion_candidates: Vec<PromotionCandidate> = promotion_map
        .into_iter()
        .map(|(rule_id, evidence)| PromotionCandidate { rule_id, evidence })
        .collect();
    promotion_candidates.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));

    (orphaned_fixtures, promotion_candidates)
}

/// Walk a directory tree and return all `.json` file paths.
fn collect_fixture_paths(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            result.extend(collect_fixture_paths(&path));
        } else if path.extension().is_some_and(|ext| ext == "json") {
            result.push(path);
        }
    }
    result
}

/// Normalise a path string to forward-slashes, trimming a leading `./`.
fn normalize_path(path: &str) -> String {
    let normalised = path.replace('\\', "/");
    normalised
        .strip_prefix("./")
        .unwrap_or(&normalised)
        .to_string()
}

/// Return true when a relative path sits inside the expected-traces subtree.
fn is_expected_traces_path(rel: &str) -> bool {
    rel.contains("expected-traces")
}

fn is_expected_traces_path_buf(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("expected-traces")
    })
}

/// Return true when `rel` matches any path in `referenced` — exact or
/// suffix match (to handle paths stored with different leading segments).
fn path_is_referenced(rel: &str, referenced: &HashSet<String>) -> bool {
    if referenced.contains(rel) {
        return true;
    }
    // Allow a suffix match: a rule may store `fixtures/kernel/foo.json` and
    // the walked path may be `fixtures/kernel/foo.json` relative to workspace.
    referenced.iter().any(|r| r.ends_with(rel) || rel.ends_with(r))
}

/// Extract and normalise the filename stem (lower-case, hyphens only).
fn file_stem_normalized(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase().replace('_', "-"))
        .unwrap_or_default()
}

/// Normalise a rule id for filename-stem matching (lower-case, hyphens).
fn normalize_id_for_matching(id: &str) -> String {
    id.to_ascii_lowercase().replace('_', "-")
}

/// Read the top-level `"rule"` field from a JSON fixture, if present.
fn read_rule_field(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value
        .get("rule")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// ── Text rendering ────────────────────────────────────────────────────────────

/// Render a coverage report as human-readable text.
pub fn render_text(report: &CoverageReport, verbose: bool) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "Rule Coverage Report (WOS {})\n\n",
        report.version
    ));

    // Summary by graduation tier.
    out.push_str(&format!(
        "LoadBearing: {:>3} rules\n",
        report.summary.load_bearing
    ));
    out.push_str(&format!(
        "Stable:      {:>3} rules\n",
        report.summary.stable
    ));
    out.push_str(&format!(
        "Tested:      {:>3} rules\n",
        report.summary.tested
    ));
    out.push_str(&format!(
        "Draft:       {:>3} rules (no promotion gate)\n",
        report.summary.draft
    ));
    out.push_str(&format!("Total:       {:>3} rules\n", report.summary.total));

    if !report.duplicate_ids.is_empty() {
        out.push('\n');
        out.push_str(&format!(
            "WARNING: {} duplicate rule id(s) found (first wins): {}\n",
            report.duplicate_ids.len(),
            report.duplicate_ids.join(", ")
        ));
    }

    // Per-category breakdown.
    out.push_str("\nCategory breakdown:\n");
    for (cat, counts) in &report.by_category {
        out.push_str(&format!(
            "  {:>12}:  {:>3} Draft, {:>3} Tested, {:>3} Stable, {:>3} LoadBearing  (total: {})\n",
            cat,
            counts.draft,
            counts.tested,
            counts.stable,
            counts.load_bearing,
            counts.total()
        ));
    }

    // Orphaned fixtures.
    out.push_str(&format!(
        "\nOrphaned fixtures (not linked to any rule): {}\n",
        report.orphaned_fixtures.len()
    ));
    for path in &report.orphaned_fixtures {
        out.push_str(&format!("  {}\n", path));
    }

    // Promotion candidates.
    out.push_str(&format!(
        "\nPromotion candidates (Draft rules with ≥1 fixture link): {}\n",
        report.promotion_candidates.len()
    ));
    for candidate in &report.promotion_candidates {
        for ev in &candidate.evidence {
            let kind_label = match ev.match_kind {
                EvidenceMatchKind::FilenameStem => "filename-stem",
                EvidenceMatchKind::RuleField => "rule-field",
            };
            out.push_str(&format!(
                "  {}: {} [{}]; promote to Tested\n",
                candidate.rule_id, ev.fixture_path, kind_label
            ));
        }
    }

    // Verbose rule detail.
    if verbose {
        out.push_str("\nRule details:\n");
        for rule in &report.rules {
            out.push_str(&format!(
                "  {:>16} | {:>12} | {:>12} | {}\n",
                rule.id, rule.graduation, rule.tier, rule.summary
            ));
            if rule.fixtures.is_empty() {
                out.push_str("    fixtures: -\n");
            } else {
                for fx in &rule.fixtures {
                    out.push_str(&format!("    fixture: {}\n", fx));
                }
            }
        }
    }

    out
}

/// Render a coverage report as pretty-printed JSON.
pub fn render_json(report: &CoverageReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_lint::rules::Tier;

    fn make_rule(id: &'static str, graduation: Graduation) -> RuleMetadata {
        RuleMetadata {
            id,
            tier: Tier::T1,
            severity: wos_lint::Severity::Error,
            summary: "test rule",
            fixtures: &[],
            graduation,
            spec_ref: None,
            suggested_fix: None,
        }
    }

    // ── rule_category ─────────────────────────────────────────────────────

    #[test]
    fn category_simple_prefix() {
        assert_eq!(rule_category("K-001"), "K");
        assert_eq!(rule_category("G-037"), "G");
        assert_eq!(rule_category("AI-041"), "AI");
    }

    #[test]
    fn category_compound_prefix() {
        assert_eq!(rule_category("K-EXT-002"), "K-EXT");
        assert_eq!(rule_category("SCHEMA-DOC-001"), "SCHEMA-DOC");
    }

    #[test]
    fn category_single_segment_no_digits() {
        // Pathological: no numeric segment — whole id is the category.
        assert_eq!(rule_category("NOID"), "NOID");
    }

    // ── graduation summary ────────────────────────────────────────────────

    #[test]
    fn graduation_summary_counts_correctly() {
        let rules = vec![
            make_rule("A-001", Graduation::Draft),
            make_rule("A-002", Graduation::Draft),
            make_rule("A-003", Graduation::Tested),
            make_rule("A-004", Graduation::Stable),
            make_rule("A-005", Graduation::LoadBearing),
        ];
        let summary = build_graduation_summary(&rules);
        assert_eq!(summary.total, 5);
        assert_eq!(summary.draft, 2);
        assert_eq!(summary.tested, 1);
        assert_eq!(summary.stable, 1);
        assert_eq!(summary.load_bearing, 1);
    }

    // ── category breakdown ────────────────────────────────────────────────

    #[test]
    fn category_breakdown_groups_by_prefix() {
        let rules = vec![
            make_rule("K-001", Graduation::Tested),
            make_rule("K-002", Graduation::Draft),
            make_rule("G-001", Graduation::Draft),
        ];
        let breakdown = build_category_breakdown(&rules);

        assert_eq!(breakdown["K"].tested, 1);
        assert_eq!(breakdown["K"].draft, 1);
        assert_eq!(breakdown["G"].draft, 1);
        assert_eq!(breakdown.get("G").map(|c| c.tested).unwrap_or(0), 0);
    }

    // ── orphan detection ──────────────────────────────────────────────────

    #[test]
    fn orphan_detection_finds_unreferenced_fixtures() {
        let dir = tempfile::tempdir().unwrap();
        let fixtures_root = dir.path().join("fixtures");
        std::fs::create_dir_all(&fixtures_root).unwrap();

        // Create two fixture files.
        let referenced_abs = fixtures_root.join("k-001-test.json");
        let orphan_abs = fixtures_root.join("k-999-orphan.json");
        std::fs::write(&referenced_abs, r#"{"id": "test"}"#).unwrap();
        std::fs::write(&orphan_abs, r#"{"id": "orphan"}"#).unwrap();

        // Build a fixture path relative to the temp workspace root (dir.path()).
        // The rule must store the path in the same relative format that
        // discover_fixtures computes.
        let referenced_rel = format!(
            "fixtures/{}",
            referenced_abs.file_name().unwrap().to_str().unwrap()
        );

        // We cannot use a `&'static [&'static str]` here because
        // `referenced_rel` is runtime-computed. Instead we test through the
        // lower-level helpers that work on `&[RuleMetadata]` directly.
        static REFERENCED_FIXTURE: &[&str] = &["fixtures/k-001-test.json"];
        let rule_with_fixture = RuleMetadata {
            id: "K-001",
            tier: Tier::T1,
            severity: wos_lint::Severity::Error,
            summary: "test",
            fixtures: REFERENCED_FIXTURE,
            graduation: Graduation::Tested,
            spec_ref: None,
            suggested_fix: None,
        };
        let _ = referenced_rel; // the static path above covers the same file

        let rules = vec![rule_with_fixture];
        let (orphans, _candidates) = discover_fixtures(&fixtures_root, &rules);

        // The orphan file must be flagged.
        let orphan_rel = normalize_path(
            orphan_abs
                .strip_prefix(dir.path())
                .unwrap()
                .to_string_lossy()
                .as_ref(),
        );
        assert!(
            orphans.contains(&orphan_rel),
            "expected {orphan_rel} in orphans; got: {orphans:?}"
        );

        // The referenced file must not be flagged.
        let referenced_rel = normalize_path(
            referenced_abs
                .strip_prefix(dir.path())
                .unwrap()
                .to_string_lossy()
                .as_ref(),
        );
        assert!(
            !orphans.contains(&referenced_rel),
            "referenced fixture must not appear in orphans"
        );
    }

    #[test]
    fn expected_traces_excluded_from_orphan_detection() {
        let dir = tempfile::tempdir().unwrap();
        let traces_dir = dir.path().join("fixtures").join("expected-traces");
        std::fs::create_dir_all(&traces_dir).unwrap();
        let golden = traces_dir.join("golden-trace.json");
        std::fs::write(&golden, r#"{"id": "trace"}"#).unwrap();

        let rules: Vec<RuleMetadata> = vec![];
        let (orphans, _) =
            discover_fixtures(dir.path().join("fixtures").as_path(), &rules);

        assert!(
            orphans.is_empty(),
            "expected-traces files must not appear in orphan list; got: {orphans:?}"
        );
    }

    // ── promotion candidate detection ─────────────────────────────────────

    #[test]
    fn promotion_candidate_found_by_filename_stem() {
        let dir = tempfile::tempdir().unwrap();
        let fixtures_root = dir.path().join("fixtures");
        std::fs::create_dir_all(&fixtures_root).unwrap();

        // Fixture filename contains the rule id.
        let fixture = fixtures_root.join("k-030-extension-prefix-bad.json");
        std::fs::write(&fixture, r#"{"id": "test"}"#).unwrap();

        let rule = make_rule("K-030", Graduation::Draft);
        let rules = vec![rule];
        let (_orphans, candidates) = discover_fixtures(&fixtures_root, &rules);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].rule_id, "K-030");
        assert!(candidates[0]
            .evidence
            .iter()
            .any(|e| e.match_kind == EvidenceMatchKind::FilenameStem));
    }

    #[test]
    fn promotion_candidate_found_by_rule_field() {
        let dir = tempfile::tempdir().unwrap();
        let fixtures_root = dir.path().join("fixtures");
        std::fs::create_dir_all(&fixtures_root).unwrap();

        // Fixture has `"rule": "K-009"` in its JSON.
        let fixture = fixtures_root.join("actor-id-uniqueness.json");
        std::fs::write(&fixture, r#"{"rule": "K-009", "id": "test"}"#).unwrap();

        let rule = make_rule("K-009", Graduation::Draft);
        let rules = vec![rule];
        let (_orphans, candidates) = discover_fixtures(&fixtures_root, &rules);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].rule_id, "K-009");
        assert!(candidates[0]
            .evidence
            .iter()
            .any(|e| e.match_kind == EvidenceMatchKind::RuleField));
    }

    #[test]
    fn no_false_positive_for_promoted_rule() {
        // A Tested rule with a matching filename must not appear in
        // promotion candidates (it is already promoted).
        let dir = tempfile::tempdir().unwrap();
        let fixtures_root = dir.path().join("fixtures");
        std::fs::create_dir_all(&fixtures_root).unwrap();

        let fixture = fixtures_root.join("k-001-negative.json");
        std::fs::write(&fixture, r#"{"id": "test"}"#).unwrap();

        let rule = make_rule("K-001", Graduation::Tested);
        let rules = vec![rule];
        let (_orphans, candidates) = discover_fixtures(&fixtures_root, &rules);

        assert!(
            candidates.is_empty(),
            "Tested rules must not appear as promotion candidates"
        );
    }
}
