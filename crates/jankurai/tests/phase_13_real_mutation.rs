use jankurai::validation::{self, ArtifactSchema};
use serde_json::json;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

fn seed_repo(repo: &Path) {
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(
        repo.join("agent/owner-map.json"),
        r#"{"workspace":"fixture","owners":{"agent/":"agent","docs/":"standard","paper/":"paper","reference/":"read-only","target/":"workspace","crates/":"tools"}}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{"docs/":{"command":"true","purpose":"fixture docs proof"},"agent/":{"command":"true","purpose":"fixture agent proof"}}}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/proof-lanes.toml"),
        r#"[[lane]]
name = "audit"
command = "true"
purpose = "fixture proof"
"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/generated-zones.toml"),
        r#"[[zone]]
path = "agent/repo-score.json"
source = "crates/jankurai"
command = "cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md"
read_only = false
"#,
    )
    .unwrap();
}

fn write_plan(repo: &Path, path: &str, append_text: &str) -> PathBuf {
    fs::create_dir_all(repo.join("target/jankurai")).unwrap();
    let plan_path = repo.join("target/jankurai/repair-plan.json");
    let plan = json!({
        "schema_version": "1.0.0",
        "source_report": "target/jankurai/repo-score.json",
        "generated_at": "0",
        "target_stack_id": "jankurai:v0.4",
        "plan_mode": "dry-run",
        "planned_edits": [{
            "path": path,
            "operation": "modify",
            "reason": "append a line",
            "finding_fingerprint": "sha256:real-apply",
            "rule_id": "HLT-017-OPAQUE-OBSERVABILITY",
            "apply_strategy": "append-text",
            "risk_level": "medium",
            "repair_eligibility": "agent-assisted",
            "append_text": append_text
        }],
        "planned_commands": ["true"],
        "proof_lanes": ["audit"],
        "rollback_guidance": ["restore the file"],
        "human_approval_requirements": [],
        "packets": [{
            "finding_fingerprint": "sha256:real-apply",
            "finding_path": path,
            "rule_id": "HLT-017-OPAQUE-OBSERVABILITY",
            "check_id": "HLT-017-OPAQUE-OBSERVABILITY",
            "severity": "medium",
            "owner": "standard",
            "lane": "audit",
            "problem": "fixture problem",
            "why": "fixture reason",
            "permission_profile": "docs-only",
            "allowed_paths": ["docs/"],
            "forbidden_paths": ["reference/"],
            "expected_patch_shape": "append text",
            "required_proof": ["true"],
            "stop_conditions": ["stop"],
            "repair_eligibility": "agent-assisted",
            "risk_level": "medium",
            "eligibility_reason": "fixture repair is scoped to docs",
            "human_review_required": false,
            "rollback_guidance": "restore the file"
        }]
    });
    fs::write(&plan_path, serde_json::to_string_pretty(&plan).unwrap()).unwrap();
    plan_path
}

fn init_git_repo(repo: &Path) -> PathBuf {
    let remote = repo.join("remote.git");
    assert!(Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg(&remote)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .current_dir(repo)
        .arg("init")
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .current_dir(repo)
        .args(["checkout", "-b", "main"])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .current_dir(repo)
        .args(["config", "user.name", "Jankurai Test"])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .current_dir(repo)
        .args(["config", "user.email", "test@example.com"])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .current_dir(repo)
        .args(["remote", "add", "origin", remote.to_str().unwrap()])
        .status()
        .unwrap()
        .success());
    remote
}

fn commit_initial_state(repo: &Path) {
    assert!(Command::new("git")
        .current_dir(repo)
        .args(["add", "."])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .current_dir(repo)
        .args(["commit", "-m", "initial"])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .current_dir(repo)
        .args(["push", "-u", "origin", "main"])
        .status()
        .unwrap()
        .success());
}

fn make_gh_stub(repo: &Path) -> (PathBuf, PathBuf) {
    let bin_dir = repo.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let gh_path = bin_dir.join("gh");
    let log_path = repo.join("target/jankurai/gh.log");
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"{}\"\nprintf 'https://example.test/pr/42\\n'\n",
        log_path.display()
    );
    fs::write(&gh_path, script).unwrap();
    let mut perms = fs::metadata(&gh_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&gh_path, perms).unwrap();
    (bin_dir, log_path)
}

fn make_failing_gh_stub(repo: &Path) -> (PathBuf, PathBuf) {
    let bin_dir = repo.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let gh_path = bin_dir.join("gh");
    let log_path = repo.join("target/jankurai/gh-fail.log");
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"{}\"\nprintf 'gh stub failure\\n' >&2\nexit 1\n",
        log_path.display()
    );
    fs::write(&gh_path, script).unwrap();
    let mut perms = fs::metadata(&gh_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&gh_path, perms).unwrap();
    (bin_dir, log_path)
}

fn repair_command(repo: &Path, plan_path: &Path, out_path: &Path) -> Command {
    let md_path = out_path.with_extension("md");
    let mut cmd = Command::new(binary_path());
    cmd.arg("repair")
        .arg(repo)
        .arg("--plan")
        .arg(plan_path)
        .arg("--out")
        .arg(out_path)
        .arg("--md")
        .arg(&md_path);
    cmd
}

fn git_current_branch(repo: &Path) -> String {
    String::from_utf8(
        Command::new("git")
            .current_dir(repo)
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string()
}

fn git_branch_exists(repo: &Path, branch: &str) -> bool {
    Command::new("git")
        .current_dir(repo)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .status()
        .unwrap()
        .success()
}

#[test]
fn real_apply_mutates_worktree_and_proves_without_git_commit() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(repo.path(), "docs/notes.md", "beta\n");

    let out_path = repo.path().join("target/jankurai/repair-run.json");
    let md_path = out_path.with_extension("md");
    let output = Command::new(binary_path())
        .arg("repair")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .arg("--apply")
        .arg("--max-risk")
        .arg("medium")
        .arg("--out")
        .arg(&out_path)
        .arg("--md")
        .arg(&md_path)
        .env("JANKURAI_ALLOW_REPAIR_APPLY", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_path).unwrap()).unwrap();
    let md = fs::read_to_string(&md_path).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();

    assert_eq!(run["execution_mode"], "real-apply");
    assert_eq!(run["status"], "complete");
    assert_eq!(run["auto_pr_status"], "not-requested");
    assert!(run["git_mutation"].is_null());
    assert!(run["github_pr"].is_null());
    assert_eq!(
        fs::read_to_string(repo.path().join("docs/notes.md")).unwrap(),
        "alpha\nbeta\n"
    );
    assert!(run["proof_evidence_index"]
        .as_str()
        .unwrap()
        .contains("p13-real-evidence-index.json"));
    assert!(md.contains("real apply mutates the working tree"));
}

#[test]
fn real_apply_can_commit_and_create_draft_pr() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(repo.path(), "docs/notes.md", "beta\n");
    let remote = init_git_repo(repo.path());
    commit_initial_state(repo.path());
    let (bin_dir, gh_log) = make_gh_stub(repo.path());
    let path = env::var("PATH").unwrap_or_default();

    let out_path = repo.path().join("target/jankurai/repair-run.json");
    let md_path = out_path.with_extension("md");
    let output = Command::new(binary_path())
        .arg("repair")
        .arg(repo.path())
        .arg("--plan")
        .arg(&plan_path)
        .arg("--apply")
        .arg("--git-commit")
        .arg("--auto-pr")
        .arg("--github-pr")
        .arg("--max-risk")
        .arg("medium")
        .arg("--remote")
        .arg(remote.as_os_str())
        .arg("--base")
        .arg("main")
        .arg("--out")
        .arg(&out_path)
        .arg("--md")
        .arg(&md_path)
        .env("JANKURAI_ALLOW_REPAIR_APPLY", "1")
        .env("JANKURAI_ALLOW_GIT_MUTATION", "1")
        .env("JANKURAI_ALLOW_GITHUB_PR", "1")
        .env("PATH", format!("{}:{}", bin_dir.display(), path))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let run: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_path).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();

    assert_eq!(run["execution_mode"], "real-apply");
    assert_eq!(run["status"], "complete");
    assert_eq!(run["auto_pr_status"], "created");
    assert_eq!(run["git_mutation"]["status"], "committed");
    assert_eq!(run["git_mutation"]["pushed"], true);
    assert_eq!(run["github_pr"]["status"], "created");
    assert_eq!(run["github_pr"]["draft"], true);
    assert_eq!(
        fs::read_to_string(repo.path().join("docs/notes.md")).unwrap(),
        "alpha\nbeta\n"
    );
    let gh_text = fs::read_to_string(gh_log).unwrap();
    assert!(gh_text.contains("pr"));
    assert!(gh_text.contains("create"));
    assert!(gh_text.contains("--draft"));
    assert!(run["github_pr"]["url"]
        .as_str()
        .unwrap()
        .contains("example.test/pr/42"));
}

#[test]
fn real_apply_requires_environment_gate_before_mutation() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(repo.path(), "docs/notes.md", "beta\n");

    let out_path = repo.path().join("target/jankurai/repair-run.json");
    let _md_path = out_path.with_extension("md");
    let output = repair_command(repo.path(), &plan_path, &out_path)
        .arg("--apply")
        .arg("--max-risk")
        .arg("medium")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("JANKURAI_ALLOW_REPAIR_APPLY=1"), "{stderr}");
    assert_eq!(
        fs::read_to_string(repo.path().join("docs/notes.md")).unwrap(),
        "alpha\n"
    );
    assert!(!out_path.exists());
}

#[test]
fn real_apply_rejects_flag_dependencies_before_env_checks() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(repo.path(), "docs/notes.md", "beta\n");

    let out_path = repo.path().join("target/jankurai/repair-run.json");
    let _md_path = out_path.with_extension("md");

    let output = repair_command(repo.path(), &plan_path, &out_path)
        .arg("--git-commit")
        .arg("--max-risk")
        .arg("medium")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("`--git-commit` requires `--apply`"),
        "{stderr}"
    );

    let output = repair_command(repo.path(), &plan_path, &out_path)
        .arg("--apply")
        .arg("--github-pr")
        .arg("--max-risk")
        .arg("medium")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("`--github-pr` requires `--git-commit`"),
        "{stderr}"
    );

    let output = repair_command(repo.path(), &plan_path, &out_path)
        .arg("--apply")
        .arg("--git-commit")
        .arg("--github-pr")
        .arg("--max-risk")
        .arg("medium")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("`--github-pr` requires `--auto-pr`"),
        "{stderr}"
    );
}

#[test]
fn real_apply_rejects_missing_git_and_github_gates() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(repo.path(), "docs/notes.md", "beta\n");

    let out_path = repo.path().join("target/jankurai/repair-run.json");
    let _md_path = out_path.with_extension("md");
    let output = repair_command(repo.path(), &plan_path, &out_path)
        .arg("--apply")
        .arg("--git-commit")
        .arg("--auto-pr")
        .arg("--github-pr")
        .arg("--max-risk")
        .arg("medium")
        .env("JANKURAI_ALLOW_REPAIR_APPLY", "1")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("JANKURAI_ALLOW_GIT_MUTATION=1"), "{stderr}");

    let output = repair_command(repo.path(), &plan_path, &out_path)
        .arg("--apply")
        .arg("--git-commit")
        .arg("--auto-pr")
        .arg("--github-pr")
        .arg("--max-risk")
        .arg("medium")
        .env("JANKURAI_ALLOW_REPAIR_APPLY", "1")
        .env("JANKURAI_ALLOW_GIT_MUTATION", "1")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("JANKURAI_ALLOW_GITHUB_PR=1"), "{stderr}");
}

#[test]
fn real_apply_rejects_dirty_tracked_worktree_before_git_mutation() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(repo.path(), "docs/notes.md", "beta\n");
    init_git_repo(repo.path());
    commit_initial_state(repo.path());
    fs::write(repo.path().join("docs/notes.md"), "dirty\n").unwrap();

    let out_path = repo.path().join("target/jankurai/repair-run.json");
    let _md_path = out_path.with_extension("md");
    let output = repair_command(repo.path(), &plan_path, &out_path)
        .arg("--apply")
        .arg("--git-commit")
        .arg("--max-risk")
        .arg("medium")
        .env("JANKURAI_ALLOW_REPAIR_APPLY", "1")
        .env("JANKURAI_ALLOW_GIT_MUTATION", "1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("clean tracked git worktree"), "{stderr}");
    assert_eq!(git_current_branch(repo.path()), "main");
}

#[test]
fn validate_branch_name_rejects_unsafe_input() {
    let repo = tempdir().unwrap();
    let error = jankurai::commands::repair_git::validate_branch_name(
        repo.path(),
        "jankurai/repair/bad branch",
    )
    .unwrap_err();
    assert!(error.to_string().contains("not safe"));
}

#[test]
fn real_apply_rolls_back_file_changes_when_proof_fails_without_git_commit() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(repo.path(), "docs/notes.md", "beta\n");
    fs::write(
        repo.path().join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{"docs/":{"command":"false","purpose":"failing proof"},"agent/":{"command":"true","purpose":"fixture agent proof"}}}"#,
    )
    .unwrap();

    let out_path = repo.path().join("target/jankurai/repair-run.json");
    let _md_path = out_path.with_extension("md");
    let output = repair_command(repo.path(), &plan_path, &out_path)
        .arg("--apply")
        .arg("--max-risk")
        .arg("medium")
        .env("JANKURAI_ALLOW_REPAIR_APPLY", "1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(
        fs::read_to_string(repo.path().join("docs/notes.md")).unwrap(),
        "alpha\n"
    );
    let run: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_path).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();
    assert_eq!(run["status"], "failed");
    assert!(run["git_mutation"].is_null());
    assert!(run["proof_evidence_index"].is_null());
}

#[test]
fn real_apply_rolls_back_repair_branch_when_proof_fails() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(repo.path(), "docs/notes.md", "beta\n");
    fs::write(
        repo.path().join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{"docs/":{"command":"false","purpose":"failing proof"},"agent/":{"command":"true","purpose":"fixture agent proof"}}}"#,
    )
    .unwrap();
    init_git_repo(repo.path());
    commit_initial_state(repo.path());

    let out_path = repo.path().join("target/jankurai/repair-run.json");
    let _md_path = out_path.with_extension("md");
    let output = repair_command(repo.path(), &plan_path, &out_path)
        .arg("--apply")
        .arg("--git-commit")
        .arg("--max-risk")
        .arg("medium")
        .env("JANKURAI_ALLOW_REPAIR_APPLY", "1")
        .env("JANKURAI_ALLOW_GIT_MUTATION", "1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let run: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_path).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();
    let branch = run["git_mutation"]["head_branch"].as_str().unwrap();
    assert_eq!(run["git_mutation"]["status"], "rolled-back");
    assert_eq!(git_current_branch(repo.path()), "main");
    assert!(!git_branch_exists(repo.path(), branch));
    assert_eq!(
        fs::read_to_string(repo.path().join("docs/notes.md")).unwrap(),
        "alpha\n"
    );
}

#[test]
fn real_apply_records_failed_github_pr_receipt_when_gh_fails() {
    let repo = tempdir().unwrap();
    seed_repo(repo.path());
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/notes.md"), "alpha\n").unwrap();
    let plan_path = write_plan(repo.path(), "docs/notes.md", "beta\n");
    let remote = init_git_repo(repo.path());
    commit_initial_state(repo.path());
    let (bin_dir, gh_log) = make_failing_gh_stub(repo.path());
    let path = env::var("PATH").unwrap_or_default();

    let out_path = repo.path().join("target/jankurai/repair-run.json");
    let _md_path = out_path.with_extension("md");
    let output = repair_command(repo.path(), &plan_path, &out_path)
        .arg("--apply")
        .arg("--git-commit")
        .arg("--auto-pr")
        .arg("--github-pr")
        .arg("--max-risk")
        .arg("medium")
        .arg("--remote")
        .arg(remote.as_os_str())
        .arg("--base")
        .arg("main")
        .env("JANKURAI_ALLOW_REPAIR_APPLY", "1")
        .env("JANKURAI_ALLOW_GIT_MUTATION", "1")
        .env("JANKURAI_ALLOW_GITHUB_PR", "1")
        .env("PATH", format!("{}:{}", bin_dir.display(), path))
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("draft GitHub PR creation failed"),
        "{stderr}"
    );
    let run: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_path).unwrap()).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::RepairRun, &run).unwrap();
    assert_eq!(run["status"], "failed");
    assert_eq!(run["auto_pr_status"], "blocked");
    assert_eq!(run["git_mutation"]["status"], "committed");
    assert_eq!(run["git_mutation"]["pushed"], true);
    assert_eq!(run["github_pr"]["status"], "failed");
    assert!(run["github_pr"]["error"]
        .as_str()
        .unwrap()
        .contains("gh stub failure"));
    let gh_text = fs::read_to_string(gh_log).unwrap();
    assert!(gh_text.contains("pr"));
    assert!(gh_text.contains("create"));
    assert!(gh_text.contains("--draft"));
}
