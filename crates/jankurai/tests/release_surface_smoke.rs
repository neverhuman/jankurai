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
    assert!(
        text.contains("actions/attest-build-provenance@43d14bc2b83dec42d39ecae14e916627a18bb661")
    );
    assert!(text.contains("sigstore/cosign-installer@ba7bc0a3fef59531c69a25acd34668d6d3fe6f22"));
    assert!(!text.contains("Swatinem/rust-cache"));
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
    assert!(text.contains("audit/repo-score.json"));
    assert!(text.contains("audit/repo-score.md"));
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
    assert!(text.contains("--verify-only"));
    assert!(text.contains("--print-asset-name"));
    assert!(text.contains("sudo installer -pkg"));
    assert!(text.contains("tar -xzf"));
}

#[test]
fn ci_local_script_exposes_shadow_lane_for_post_main_mirror() {
    let text = read("scripts/ci-local.sh");

    assert!(text.contains("shadow) bash ops/ci/post-main-shadow.sh ;;"));
    assert!(text.contains("release-publish"));
    assert!(text.contains("GitLab"));
}

#[test]
fn node_tools_script_provisions_pinned_node_for_npm_parity() {
    let text = read("ops/ci/node-tools.sh");

    assert!(text.contains("NODE_VERSION"));
    assert!(text.contains("setup_${node_major}.x"));
    assert!(text.contains("nodejs"));
    assert!(text.contains("brew install"));
    assert!(text.contains("bootstrap did not make node/npm available"));
}

#[test]
fn security_tools_script_bootstraps_node_before_security_scans() {
    let text = read("ops/ci/security-tools.sh");

    assert!(text.contains("node-tools.sh"));
    assert!(text.contains("Node.js toolchain"));
    assert!(text.contains("cargo-audit"));
    assert!(text.contains("zizmor"));
    assert!(text.contains("gitleaks"));
}

#[test]
fn audit_script_bootstraps_node_before_npm_ci() {
    let text = read("ops/ci/audit.sh");

    let node_bootstrap = text.find("node-tools.sh").expect("node bootstrap");
    let npm_ci = text.find("step \"npm ci\"").expect("npm ci step");
    assert!(node_bootstrap < npm_ci);
}

#[test]
fn coverage_script_adds_cargo_home_bin_before_tool_checks() {
    let text = read("ops/ci/coverage-llvm.sh");

    assert!(text.contains("cargo_bin_dir"));
    assert!(text.contains("CARGO_HOME"));
    assert!(text.contains("cargo-llvm-cov"));
    assert!(text.contains("cargo-mutants"));
}

#[test]
fn post_main_shadow_script_is_local_origin_only_and_jeryu_backed() {
    let text = read("ops/ci/post-main-shadow.sh");

    assert!(text.contains("ssh://git@127.0.0.1:2224/root/jankurai.git"));
    assert!(text.contains(".jeryu/local/repos/jankurai.toml"));
    assert!(text.contains("jeryu repo shadow --repo root/jankurai"));
    assert!(text.contains("CI_COMMIT_BRANCH"));
    assert!(text.contains("CI_COMMIT_SHA"));
}

#[test]
fn homebrew_formula_template_uses_tagged_source_checkout() {
    let text = read("ops/homebrew/jankurai.rb");

    assert!(text.contains("__RELEASE_TAG__"));
    assert!(text.contains("https://github.com/neverhuman/jankurai.git"));
    assert!(text.contains("system \"cargo\", \"install\""));
    assert!(text.contains("bin/\"jankurai\""));
}
