use serde_yaml::Value as YamlValue;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).expect(path)
}

fn write_executable(path: &std::path::Path, body: &str) {
    fs::write(path, body).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
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
fn release_audit_gate_binds_tag_identity_and_relocation_proof() {
    let text = read("ops/ci/release-audit-gate.sh");
    let audit = read("ops/ci/audit.sh");
    let lib = read("ops/ci/lib.sh");

    assert!(text.contains("agent/standard-version.toml"));
    assert!(text.contains("RELEASE_TAG (${RELEASE_TAG}) does not match release_tag"));
    assert!(text.contains("relocation-test.sh"));
    assert!(text.contains("JAIN_HOST_CI_NETWORK_ISOLATED"));
    assert!(text.contains("security_profile=release"));
    assert!(text.contains("--profile \"$security_profile\""));
    assert!(lib.contains("install_local_jankurai()"));
    assert!(lib.contains("--root \"$install_root\""));
    assert!(lib.contains("JANKURAI_CANDIDATE_BIN=\"${install_root}/bin/jankurai\""));
    for lane in [&text, &audit] {
        assert!(lane.contains("install_local_jankurai \"${ARTIFACT_ROOT}/candidate-install\""));
        assert!(lane.contains("\"${JANKURAI_CANDIDATE_BIN}\" security run"));
        assert!(lane.contains("\"${JANKURAI_CANDIDATE_BIN}\" audit"));
        assert!(!lane
            .lines()
            .any(|line| line.trim_start().starts_with("jankurai ")));
    }

    let relocation = read("ops/ci/relocation-test.sh");
    assert!(relocation.contains("CARGO_NET_OFFLINE=true"));
    assert!(relocation.contains("rm -rf \"${source_dir}\" \"${target_dir}\""));
    assert!(relocation.contains("diff-audit"));
    assert!(relocation.contains("gate \"${fixture}\" --staged-only"));
}

#[test]
fn local_candidate_install_ignores_a_hostile_path_jankurai() {
    let dir = tempdir().unwrap();
    let bin_dir = dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let stale_marker = dir.path().join("stale-selected");
    let candidate_marker = dir.path().join("candidate-selected");
    let install_root = dir.path().join("candidate-install");

    write_executable(
        &bin_dir.join("jankurai"),
        "#!/usr/bin/env bash\n: >\"${STALE_MARKER:?}\"\n",
    );
    write_executable(
        &bin_dir.join("cargo"),
        r#"#!/usr/bin/env bash
set -euo pipefail
install_root=""
while [[ $# -gt 0 ]]; do
  if [[ "$1" == "--root" ]]; then
    install_root="$2"
    shift 2
  else
    shift
  fi
done
[[ -n "$install_root" ]]
mkdir -p "$install_root/bin"
cat >"$install_root/bin/jankurai" <<'EOF'
#!/usr/bin/env bash
: >"${CANDIDATE_MARKER:?}"
EOF
chmod +x "$install_root/bin/jankurai"
"#,
    );

    let command = format!(
        "source '{}'; install_local_jankurai \"$INSTALL_ROOT\"; \"$JANKURAI_CANDIDATE_BIN\"",
        repo_root().join("ops/ci/lib.sh").display()
    );
    let output = Command::new("/bin/bash")
        .arg("-c")
        .arg(command)
        .env("CI_ROOT", repo_root())
        .env("ARTIFACT_ROOT", dir.path().join("artifacts"))
        .env("INSTALL_ROOT", &install_root)
        .env("STALE_MARKER", &stale_marker)
        .env("CANDIDATE_MARKER", &candidate_marker)
        .env("PATH", format!("{}:/usr/bin:/bin", bin_dir.display()))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "candidate binding failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        candidate_marker.exists(),
        "candidate executable was not used"
    );
    assert!(!stale_marker.exists(), "hostile PATH jankurai was selected");
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
    assert!(text.contains("JAIN_HOST_CI_NETWORK_ISOLATED"));
    assert!(text.contains("network-isolated release uses the offline release security profile"));
    assert!(text.contains("cargo_bin_dir"));
    assert!(text.contains("cargo-audit"));
    assert!(text.contains("zizmor"));
    assert!(text.contains("gitleaks"));
    assert!(text.contains("local_bin"));
    assert!(text.contains("could not install gitleaks without sudo or a writable"));
}

#[test]
fn security_tools_rejects_malformed_host_isolation_mode_before_setup() {
    let output = Command::new("bash")
        .arg(repo_root().join("ops/ci/security-tools.sh"))
        .current_dir(repo_root())
        .env("JAIN_HOST_CI_NETWORK_ISOLATED", "invalid")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("JAIN_HOST_CI_NETWORK_ISOLATED must be exactly 0 or 1"));
}

#[test]
fn network_isolated_security_tools_consumes_projection_without_installers() {
    let bin_dir = tempdir().unwrap();
    for tool in ["cargo-audit", "zizmor", "gitleaks", "syft", "grype"] {
        write_executable(&bin_dir.path().join(tool), "#!/usr/bin/env bash\nexit 93\n");
    }
    let tripwire = bin_dir.path().join("installer-invoked");
    for tool in ["cargo", "curl", "sudo", "npm", "node"] {
        write_executable(
            &bin_dir.path().join(tool),
            "#!/usr/bin/env bash\n: >\"${TRIPWIRE_MARKER:?}\"\nexit 97\n",
        );
    }

    let output = Command::new("/bin/bash")
        .arg(repo_root().join("ops/ci/security-tools.sh"))
        .current_dir(repo_root())
        .env("JAIN_HOST_CI_NETWORK_ISOLATED", "1")
        .env("TRIPWIRE_MARKER", &tripwire)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", bin_dir.path().display()),
        )
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "projected setup failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!tripwire.exists(), "release setup invoked an installer");
}

#[test]
fn network_isolated_security_tools_requires_every_projected_command() {
    let bin_dir = tempdir().unwrap();
    for tool in ["cargo-audit", "zizmor", "gitleaks", "syft"] {
        write_executable(&bin_dir.path().join(tool), "#!/usr/bin/env bash\nexit 0\n");
    }

    let output = Command::new("/bin/bash")
        .arg(repo_root().join("ops/ci/security-tools.sh"))
        .current_dir(repo_root())
        .env("JAIN_HOST_CI_NETWORK_ISOLATED", "1")
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", bin_dir.path().display()),
        )
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("network-isolated release requires projected security tool: grype"));
}

#[test]
fn release_security_profile_uses_the_offline_javascript_advisory_path() {
    let policy = read("agent/security-policy.toml");
    let lane = read("tools/security-lane.sh");

    assert!(policy.contains("[profiles.release]"));
    assert!(policy.contains(
        "required_tools = [\"gitleaks\", \"cargo-audit\", \"zizmor\", \"syft\", \"grype\"]"
    ));
    assert!(lane.contains("JANKURAI_SECURITY_PROFILE"));
    assert!(lane.contains("javascript-lock-cataloger"));
    assert!(lane.contains("GRYPE_DB_AUTO_UPDATE=false"));
    assert!(lane.contains("grype sbom:"));
    assert!(lane.contains("npm audit --audit-level=high"));
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
