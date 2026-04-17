//! Smoke tests for the wos-synth crate scaffold.
//!
//! The real assertion is that these tests compile and link: they prove
//! `wos-synth` builds both with and without `--features synth`. Once Task 2+
//! adds real public types, these tests will evolve into behaviour checks.

#[test]
fn crate_links_without_synth_feature() {
    // Linking the `wos_synth` crate is the assertion. Build-level
    // verification (`cargo tree`) is what enforces the feature gate
    // on provider deps.
    #[allow(unused_imports)]
    use wos_synth::types;
}

#[cfg(feature = "synth")]
#[test]
fn crate_links_with_synth_feature() {
    // With the feature on, the gated module must also be reachable.
    #[allow(unused_imports)]
    use wos_synth::provider_gated;
}
