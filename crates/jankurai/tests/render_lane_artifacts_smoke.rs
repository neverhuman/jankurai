use jankurai::audit::run_audit;
use jankurai::render::render_markdown;
use jankurai::report::github;
use std::fs;
use tempfile::tempdir;

fn thin_repo(dir: &std::path::Path) {
    fs::write(dir.join("README.md"), "# thin repo\n").unwrap();
}

fn minimal_ux_envelope() -> serde_json::Value {
    serde_json::json!({
        "reports": [{
            "schemaVersion": "1.4.0",
            "toolVersion": "0.5.0",
            "url": "about:blank",
            "checkedAt": "2026-05-02T12:00:00.000Z",
            "viewport": { "width": 1280, "height": 720 },
            "metrics": {
                "scrollWidth": 1280,
                "clientWidth": 1280,
                "scrollHeight": 720,
                "clientHeight": 720
            },
            "elements": [],
            "violations": [],
            "artifacts": [{
                "kind": "screenshot",
                "path": "target/jankurai/ux-qa/local.png",
                "sha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "viewport": { "width": 1280, "height": 720 }
            },{
                "kind": "accessibility",
                "path": "target/jankurai/ux-qa/local.a11y.json",
                "viewport": { "width": 1280, "height": 720 }
            }],
            "visualBaseline": {
                "mode": "review",
                "status": "changed",
                "decision": "review",
                "actualPath": "target/jankurai/ux-qa/local.png",
                "baselinePath": "target/jankurai/ux-qa/baseline.png",
                "diffPath": "target/jankurai/ux-qa/diff.png",
                "actualSha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "baselineSha256": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "owner": "design",
                "approvedBy": "ux",
                "approvedAt": "2026-05-02T12:00:00.000Z",
                "approvalNote": "fixture"
            },
            "artifactCoverage": {
                "required": ["screenshot", "aria-snapshot", "accessibility"],
                "present": ["screenshot", "accessibility"],
                "missing": ["aria-snapshot"]
            },
            "accessibility": {
                "violations": 1,
                "incomplete": 2,
                "passes": 9,
                "artifactPath": "target/jankurai/ux-qa/local.a11y.json"
            },
            "summary": { "errors": 0, "warnings": 0, "byRule": {} },
            "decision": "warn"
        }]
    })
}

fn minimal_boundaries_toml() -> &'static str {
    r#"
[stack]
id = "fixture-stack"
version = "0.1.0"

[queues]
adapter_paths = ["a/"]
event_contract_paths = []
generated_type_paths = ["g/"]
client_markers = ["m"]
"#
}

fn minimal_security_envelope() -> serde_json::Value {
    serde_json::json!({
        "schema_version": "1.0.0",
        "standard_version": "0.5.0",
        "generated_at": "2026-05-02T12:00:00.000Z",
        "repo_root": "/tmp/x",
        "lane": "security",
        "wrapper": { "kind": "bash_script", "path": "tools/security-lane.sh", "strict": false },
        "exit_code": 0,
        "elapsed_ms": 9,
        "log_path": "target/jankurai/security/run.log",
        "policy": {
            "schema_version": "1.0.0",
            "profile": "local",
            "enabled_tools": ["gitleaks"],
            "required_tools": ["gitleaks"],
            "advisory_tools": [],
            "require_one_of": [],
            "fail_lane_on": "high"
        },
        "commands": [{
            "label": "lane",
            "shell_command": "bash tools/security-lane.sh",
            "status": "ran",
            "advisory": false,
            "required_by_policy": true,
            "blocking": false
        }]
    })
}

#[test]
fn markdown_and_github_summary_include_lane_artifacts() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("target/jankurai")).unwrap();
    fs::create_dir_all(dir.path().join("target/jankurai/security")).unwrap();

    fs::write(
        dir.path().join("target/jankurai/ux-qa.json"),
        serde_json::to_string(&minimal_ux_envelope()).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.path().join("target/jankurai/security/evidence.json"),
        serde_json::to_string(&minimal_security_envelope()).unwrap(),
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        minimal_boundaries_toml(),
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report.ux_qa.artifact.is_some());
    assert!(report.security_evidence.artifact.is_some());
    assert!(report.boundaries.artifact.is_some());

    let md = render_markdown(&report);
    assert!(md.contains("### Ingested UX QA report (`target/jankurai/ux-qa.json`)"));
    assert!(md.contains("- Worst decision: `warn`"));
    let counts_line = md
        .lines()
        .find(|l| l.contains("Artifact counts"))
        .unwrap_or("");
    assert!(
        counts_line.contains("accessibility=1"),
        "expected accessibility artifact count in markdown; counts line was {:?}; md:\n{md}",
        counts_line
    );
    assert!(md.contains("- Artifact fingerprints: `1`"));
    assert!(md.contains("- Visual baseline counts: missing=`0` changed=`1` review=`1` block=`0`"));
    assert!(md.contains("- Missing required artifacts: `1` report(s) `aria-snapshot`"));
    assert!(md.contains("- Accessibility violations / incomplete / passes: `1` / `2` / `9`"));
    assert!(md.contains("## Security evidence (ingested)"));
    assert!(md.contains("- Source: `target/jankurai/security/evidence.json`"));
    assert!(md.contains("## Boundary manifest (ingested)"));
    assert!(md.contains("- Path: `agent/boundaries.toml`"));
    assert!(md.contains("- Stack: `fixture-stack` · version: `0.1.0`"));
    assert!(md.contains(
        "- Queue path counts — adapter: `1`, event_contract: `0`, generated_type: `1`, client_marker: `1`, streaming_exception: `0`"
    ));
    let b = report.boundaries.artifact.as_ref().unwrap();
    assert!(md.contains(&format!(
        "- Content fingerprint: `{}`",
        b.content_fingerprint
    )));

    let gh = github::render_step_summary(&report);
    assert!(gh.contains("#### lane artifacts"));
    assert!(gh.contains("ux-qa `target/jankurai/ux-qa.json`"));
    assert!(gh.contains("worst=warn"));
    assert!(gh.contains("missing_artifacts=1"));
    assert!(gh.contains("fingerprints=1"));
    assert!(gh.contains("visual_baseline=missing:0/changed:1/review:1/block:0"));
    assert!(gh.contains("a11y_violations=1"));
    assert!(gh.contains("security `target/jankurai/security/evidence.json`"));
    assert!(gh.contains("- boundaries `agent/boundaries.toml`:"));
    assert!(gh.contains("stack=`fixture-stack`"));
    assert!(gh.contains("adapters=`1`"));
    assert!(gh.contains("fp=`sha256:"));
    let digest_short: String = b
        .content_fingerprint
        .strip_prefix("sha256:")
        .unwrap()
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(12)
        .collect();
    assert!(gh.contains(&format!("fp=`sha256:{digest_short}`")));
    assert!(!gh.contains(&format!("fp=`{}`", b.content_fingerprint)));
}

#[test]
fn github_summary_shows_lane_artifacts_when_only_boundaries_ingested() {
    let dir = tempdir().unwrap();
    thin_repo(dir.path());
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        minimal_boundaries_toml(),
    )
    .unwrap();

    let report = run_audit(dir.path(), &[]).unwrap();
    assert!(report.boundaries.artifact.is_some());
    assert!(report.ux_qa.artifact.is_none());
    assert!(report.security_evidence.artifact.is_none());

    let md = render_markdown(&report);
    assert!(md.contains("## Boundary manifest (ingested)"));

    let gh = github::render_step_summary(&report);
    assert!(gh.contains("#### lane artifacts"));
    assert!(gh.contains("- boundaries `agent/boundaries.toml`:"));
    assert!(!gh.contains("ux-qa `target/"));
    assert!(!gh.contains("security `target/"));
}
