//! Jeryu family onboard contract for the 1.6.11 pin.
//! Init/CI templates must download the GitHub Release, never crates.io.

#[test]
fn init_workflow_forbids_cargo_install_and_pins_1611_sha() {
    let body = include_str!("../src/init/templates.rs");
    assert!(body.contains("v1.6.11-deadlang-precision-split.3"));
    assert!(body.contains("9e6b8857a26f6004d4c74e510e13b06d880f2e2ae0c89502698889ed690c5d6c"));
    assert!(body.contains("jankurai badge --check"));
    assert!(body.contains("[precommit_gate]"));
    assert!(body.contains("blocking = true"));
    assert!(body.contains("gate ."));
    assert!(
        !body.contains("run: cargo install jankurai --locked"),
        "init workflow must not cargo-install the auditor"
    );
}

#[test]
fn readme_names_jeryu_pin_1611() {
    let readme = include_str!("../../../README.md");
    assert!(readme.contains("Jeryu pins **Jankurai 1.6.11**"));
    assert!(readme.contains("9e6b8857a26f6004d4c74e510e13b06d880f2e2ae0c89502698889ed690c5d6c"));
    assert!(readme.contains("JANKURAI_SKIP_HOOKS=1"));
    assert!(readme.contains("adopt` stays plan-only"));
}
