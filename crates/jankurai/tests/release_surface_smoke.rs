use serde_yaml::Value as YamlValue;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).expect(path)
}

#[test]
fn release_workflow_exposes_attestation_and_signed_artifacts() {
    let text = read(".github/workflows/release.yml");
    let yaml: YamlValue = serde_yaml::from_str(&text).expect("release workflow parses as YAML");

    assert_eq!(yaml["name"].as_str(), Some("release"));
    assert!(text.contains("actions/attest-build-provenance@43d14bc2b83dec42d39ecae14e916627a18bb661"));
    assert!(text.contains("sigstore/cosign-installer@ba7bc0a3fef59531c69a25acd34668d6d3fe6f22"));
    assert!(text.contains("id-token: write"));
    assert!(text.contains("attestations: write"));
    assert!(text.contains("dist/*.tar.gz.sha256"));
    assert!(text.contains("dist/*.tar.gz.sigstore.bundle"));
    assert!(text.contains("dist/*.pkg.sha256"));
    assert!(text.contains("dist/*.pkg.sigstore.bundle"));
    assert!(text.contains("release-build.sh"));
    assert!(text.contains("release-publish.sh"));
}

#[test]
fn release_build_script_switches_between_tar_and_pkg_outputs() {
    let text = read("ops/ci/release-build.sh");

    assert!(text.contains("release-macos-sign.sh"));
    assert!(text.contains("release-sign-blob.sh"));
    assert!(text.contains("tar -czf"));
    assert!(text.contains(".pkg.sha256"));
    assert!(text.contains(".pkg.sigstore.bundle"));
    assert!(text.contains(".tar.gz.sha256"));
    assert!(text.contains(".tar.gz.sigstore.bundle"));
    assert!(text.contains("unsupported release target"));
}

#[test]
fn release_publish_script_stages_installer_and_formula_metadata() {
    let text = read("ops/ci/release-publish.sh");

    assert!(text.contains("jankurai-installer.sh"));
    assert!(text.contains("jankurai-homebrew.rb"));
    assert!(text.contains("__RELEASE_TAG__"));
    assert!(text.contains("gh release create"));
    assert!(text.contains("--verify-tag"));
    assert!(text.contains("gh release verify"));
    assert!(text.contains("jankurai-installer.sh.sha256"));
    assert!(text.contains("jankurai-homebrew.rb.sha256"));
}

#[test]
fn installer_script_verifies_release_provenance_before_installing() {
    let text = read("jankurai-installer.sh");

    assert!(text.contains("gh release verify"));
    assert!(text.contains("gh attestation verify"));
    assert!(text.contains("cosign verify-blob"));
    assert!(text.contains("JANKURAI_RELEASE_TAG"));
    assert!(text.contains("JANKURAI_INSTALL_DIR"));
    assert!(text.contains("sudo installer -pkg"));
    assert!(text.contains("tar -xzf"));
}

#[test]
fn homebrew_formula_template_uses_tagged_source_checkout() {
    let text = read("ops/homebrew/jankurai.rb");

    assert!(text.contains("__RELEASE_TAG__"));
    assert!(text.contains("https://github.com/neverhuman/jankurai.git"));
    assert!(text.contains("system \"cargo\", \"install\""));
    assert!(text.contains("bin/\"jankurai\""));
}
