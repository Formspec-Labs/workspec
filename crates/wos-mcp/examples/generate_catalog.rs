//! Rebuild or check committed discovery and native-root coverage artifacts.
use std::{env, fs, path::PathBuf};
use wos_mcp::catalog::{descriptors, workflow_coverage};
fn main() {
    let write = env::args().nth(1).as_deref() == Some("--write");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (name, value) in [
        ("tool-catalog.json", descriptors()),
        ("WORKFLOW-COVERAGE.json", workflow_coverage()),
    ] {
        let bytes = format!(
            "{}\n",
            serde_json::to_string_pretty(&value).expect("catalog serializes")
        );
        let path = root.join(name);
        if write {
            fs::write(path, bytes).expect("write generated artifact");
        } else {
            assert_eq!(
                fs::read_to_string(&path).expect("run generate_catalog --write first"),
                bytes,
                "stale {}",
                path.display()
            );
        }
    }
}
