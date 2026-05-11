use jankurai::init::profiles::BUNDLED_PROFILE_IDS;
use jankurai::{audit, commands::init, init::adapters};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_jankurai")
}

fn assert_command_success(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_audit_and_doctor(repo: &Path) {
    let json = repo.join("agent/repo-score.json");
    let md = repo.join("agent/repo-score.md");
    assert_command_success(
        Command::new(binary_path())
            .arg("audit")
            .arg(repo)
            .arg("--mode")
            .arg("advisory")
            .arg("--json")
            .arg(&json)
            .arg("--md")
            .arg(&md),
    );

    assert_command_success(
        Command::new(binary_path())
            .arg("doctor")
            .arg(repo)
            .arg("--fail-on")
            .arg("high"),
    );
}

fn dry_run_plan_args(
    repo: std::path::PathBuf,
    profile: &str,
    plan_json: Option<String>,
) -> init::InitArgs {
    init::InitArgs {
        repo,
        apply: false,
        dry_run: true,
        yes: false,
        profile: profile.into(),
        profile_file: None,
        level: "full".into(),
        ide: "all".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: false,
        plan_json,
        force_generated_adapters: false,
    }
}

fn greenfield_apply_args(repo: std::path::PathBuf, profile: &str) -> init::InitArgs {
    init::InitArgs {
        repo,
        apply: false,
        dry_run: false,
        yes: true,
        profile: profile.into(),
        profile_file: None,
        level: "full".into(),
        ide: "all".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: false,
        plan_json: None,
        force_generated_adapters: false,
    }
}

fn git(repo: &std::path::Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_stdout(repo: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {:?} failed", args);
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn init_git_repo(repo: &std::path::Path) {
    git(repo, &["init", "-q"]);
    git(repo, &["config", "user.email", "jankurai@example.test"]);
    git(repo, &["config", "user.name", "Jankurai Test"]);
}

fn plan_paths(value: &serde_json::Value) -> Vec<String> {
    let mut paths: Vec<String> = value["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|action| action["path"].as_str().unwrap().to_string())
        .collect();
    paths.sort();
    paths
}

#[test]
fn init_unknown_profile_errors() {
    let dir = tempdir().unwrap();
    let err = init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: true,
        yes: false,
        profile: "not-a-real-profile".into(),
        profile_file: None,
        level: "full".into(),
        ide: "all".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: false,
        plan_json: None,
        force_generated_adapters: false,
    })
    .unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("unknown init profile"), "{msg}");
}

#[test]
fn workspace_gitignore_keeps_conformance_repo_score_fixtures_visible() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let text = fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(
        text.contains("!conformance/expected/*.repo-score.json"),
        "{text}"
    );
    assert!(!text.contains("*-score.json"), "{text}");
    assert!(!text.contains("*-score.md"), "{text}");
}

#[test]
fn init_level_agents_only_plans_agent_and_provider_guidance() {
    let dir = tempdir().unwrap();
    let plan_path = dir.path().join("init-agents.json");
    let mut args = dry_run_plan_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
        Some(plan_path.to_string_lossy().into_owned()),
    );
    args.level = "agents".into();
    init::run(args).unwrap();

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(plan_path).unwrap()).unwrap();
    assert_eq!(value["level"], "agents");
    let paths = plan_paths(&value);
    assert!(paths.contains(&"AGENTS.md".to_string()));
    assert!(paths.contains(&"agent/JANKURAI_STANDARD.md".to_string()));
    assert!(paths.contains(&"CLAUDE.md".to_string()));
    assert!(paths.contains(&"GEMINI.md".to_string()));
    assert!(paths.contains(&".cursor/rules/jankurai.mdc".to_string()));
    assert!(paths.contains(&".github/copilot-instructions.md".to_string()));
    assert!(paths.contains(&".agents/skills/jankurai/SKILL.md".to_string()));
    assert!(!paths.contains(&"Justfile".to_string()));
    assert!(!paths.contains(&"agent/owner-map.json".to_string()));
    assert!(!paths.contains(&".github/workflows/jankurai.yml".to_string()));
    assert!(!paths.contains(&"tools/jankurai-hooks/pre-commit".to_string()));
    assert!(!paths.contains(&"tools/jankurai-hooks/prepare-commit-msg".to_string()));
    assert!(!paths.iter().any(|path| path.starts_with("apps/")));
    assert!(!paths.iter().any(|path| path.starts_with("ops/")));
    assert!(!paths.iter().any(|path| path.starts_with("docs/")));
    assert!(!paths.iter().any(|path| path.starts_with("contracts/")));
    assert!(!paths.iter().any(|path| path.starts_with("db/")));
}

#[test]
fn init_level_score_adds_local_scoring_without_ci_or_full_scaffold() {
    let dir = tempdir().unwrap();
    let plan_path = dir.path().join("init-score.json");
    let mut args = dry_run_plan_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
        Some(plan_path.to_string_lossy().into_owned()),
    );
    args.level = "score".into();
    init::run(args).unwrap();

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(plan_path).unwrap()).unwrap();
    assert_eq!(value["level"], "score");
    let paths = plan_paths(&value);
    for expected in [
        "Justfile",
        "apps/api/AGENTS.md",
        "apps/web/AGENTS.md",
        "contracts/AGENTS.md",
        "agent/audit-policy.toml",
        "agent/generated-zones.toml",
        "agent/owner-map.json",
        "agent/proof-lanes.toml",
        "agent/standard-version.toml",
        "agent/tool-adoption.toml",
        "agent/test-map.json",
        "crates/adapters/AGENTS.md",
        "crates/application/AGENTS.md",
        "crates/domain/AGENTS.md",
        "crates/workers/AGENTS.md",
        "db/AGENTS.md",
        "ops/AGENTS.md",
        "python/ai-service/AGENTS.md",
    ] {
        assert!(paths.contains(&expected.to_string()), "missing {expected}");
    }
    assert!(!paths.contains(&".github/workflows/jankurai.yml".to_string()));
    assert!(!paths.contains(&"agent/security-policy.toml".to_string()));
    assert!(!paths.contains(&"tools/security-lane.sh".to_string()));
    assert!(!paths.contains(&"tools/jankurai-hooks/pre-commit".to_string()));
    assert!(!paths.contains(&"tools/jankurai-hooks/prepare-commit-msg".to_string()));
    assert!(!paths.iter().any(|path| path.starts_with("docs/")));

    let rust_api_plan = dir.path().join("init-score-rust-api.json");
    let mut rust_api_args = dry_run_plan_args(
        dir.path().to_path_buf(),
        "rust-api",
        Some(rust_api_plan.to_string_lossy().into_owned()),
    );
    rust_api_args.level = "score".into();
    init::run(rust_api_args).unwrap();
    let rust_api: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(rust_api_plan).unwrap()).unwrap();
    let rust_api_paths = plan_paths(&rust_api);
    assert!(rust_api_paths.contains(&"Justfile".to_string()));
    assert!(rust_api_paths.contains(&"apps/api/AGENTS.md".to_string()));
    assert!(rust_api_paths.contains(&"contracts/AGENTS.md".to_string()));
    assert!(rust_api_paths.contains(&"crates/domain/AGENTS.md".to_string()));
    assert!(rust_api_paths.contains(&"ops/AGENTS.md".to_string()));
}

#[test]
fn init_level_ci_adds_observe_workflow_and_preserves_existing_workflow() {
    let dir = tempdir().unwrap();
    let plan_path = dir.path().join("init-ci.json");
    let mut args = dry_run_plan_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
        Some(plan_path.to_string_lossy().into_owned()),
    );
    args.level = "ci".into();
    init::run(args).unwrap();

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
    assert_eq!(value["level"], "ci");
    let paths = plan_paths(&value);
    assert!(paths.contains(&".github/workflows/jankurai.yml".to_string()));
    assert!(paths.contains(&"agent/security-policy.toml".to_string()));
    assert!(paths.contains(&"agent/tool-adoption.toml".to_string()));
    assert!(paths.contains(&"tools/security-lane.sh".to_string()));
    assert!(!paths.contains(&"tools/jankurai-hooks/pre-commit".to_string()));
    assert!(!paths.contains(&"tools/jankurai-hooks/prepare-commit-msg".to_string()));
    assert!(!paths.iter().any(|path| path.starts_with("docs/")));

    let mut apply = greenfield_apply_args(dir.path().to_path_buf(), "rust-ts-postgres");
    apply.level = "ci".into();
    init::run(apply).unwrap();
    let workflow = fs::read_to_string(dir.path().join(".github/workflows/jankurai.yml")).unwrap();
    assert!(workflow.contains("jankurai audit . --mode advisory"));
    assert!(!workflow.contains("Enforce score floor"));

    let existing_dir = tempdir().unwrap();
    let workflow_path = existing_dir.path().join(".github/workflows/jankurai.yml");
    fs::create_dir_all(workflow_path.parent().unwrap()).unwrap();
    fs::write(&workflow_path, "name: existing\n").unwrap();
    let mut existing_apply =
        greenfield_apply_args(existing_dir.path().to_path_buf(), "rust-ts-postgres");
    existing_apply.level = "ci".into();
    init::run(existing_apply).unwrap();
    assert_eq!(
        fs::read_to_string(workflow_path).unwrap(),
        "name: existing\n"
    );
}

#[test]
fn init_level_full_creates_tracked_hook_scripts() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "fixture-init"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
    ))
    .unwrap();

    let gitignore = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    let gitignore_lines: Vec<_> = gitignore.lines().map(str::trim).collect();
    assert!(gitignore_lines.contains(&"target/jankurai/"), "{gitignore}");
    assert!(gitignore_lines.contains(&".jankurai/"), "{gitignore}");
    assert!(!gitignore_lines.contains(&"target/"), "{gitignore}");

    assert!(dir.path().join("agent/tool-adoption.toml").exists());
    let pre_commit = dir.path().join("tools/jankurai-hooks/pre-commit");
    let prepare = dir.path().join("tools/jankurai-hooks/prepare-commit-msg");
    assert!(pre_commit.is_file());
    assert!(prepare.is_file());
    let pre_commit_text = fs::read_to_string(pre_commit).unwrap();
    assert!(pre_commit_text.contains("--mode advisory"));
    assert!(
        pre_commit_text.contains("JANKURAI_HOOK_REPORT_DIR"),
        "{pre_commit_text}"
    );
    assert!(
        pre_commit_text.contains("JANKURAI_HOOK_STAGE_ARTIFACTS"),
        "{pre_commit_text}"
    );
    assert!(
        pre_commit_text.contains("JANKURAI_HISTORY_MIRROR"),
        "{pre_commit_text}"
    );
    assert!(
        pre_commit_text.contains("JANKURAI_HISTORY_MIRROR_REQUIRED"),
        "{pre_commit_text}"
    );
    assert!(
        pre_commit_text.contains("JANKURAI_SCORE_HISTORY_MAX_ROWS"),
        "{pre_commit_text}"
    );
    assert!(
        pre_commit_text.contains("JANKURAI_SCORE_HISTORY_MAX_BYTES"),
        "{pre_commit_text}"
    );
    assert!(
        !pre_commit_text.contains("agent/repo-score.json"),
        "{pre_commit_text}"
    );
    assert!(
        !pre_commit_text.contains("agent/score-history.jsonl"),
        "{pre_commit_text}"
    );
    let prepare_text = fs::read_to_string(prepare).unwrap();
    assert!(prepare_text.contains("Jankurai-Score:"));
    assert!(
        prepare_text.contains("target/jankurai/hooks/pre-commit-score.json"),
        "{prepare_text}"
    );
    let witness = dir.path().join("tools/jankurai-rust/witness.sh");
    assert!(witness.is_file());
    assert!(fs::read_to_string(dir.path().join("Justfile"))
        .unwrap()
        .contains("rust-map:"));
    assert!(fs::read_to_string(dir.path().join("Justfile"))
        .unwrap()
        .contains("rust-diagnose:"));
}

#[test]
fn init_level_full_without_cargo_skips_rust_foundation_templates() {
    let dir = tempdir().unwrap();
    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
    ))
    .unwrap();

    assert!(dir.path().join("agent/tool-adoption.toml").exists());
    assert!(!dir.path().join("tools/jankurai-rust/witness.sh").exists());
    let justfile = fs::read_to_string(dir.path().join("Justfile")).unwrap();
    assert!(!justfile.contains("rust-map:"));
    assert!(!justfile.contains("rust-diagnose:"));
}

#[test]
fn init_level_full_gitignore_keeps_jankurai_outputs_narrow() {
    let dir = tempdir().unwrap();
    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
    ))
    .unwrap();

    let text = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    let lines: Vec<_> = text.lines().map(str::trim).collect();
    assert!(lines.contains(&"target/jankurai/"), "{text}");
    assert!(lines.contains(&".jankurai/"), "{text}");
    assert!(!lines.contains(&"target/"), "{text}");
    assert!(!lines.contains(&"*-score.json"), "{text}");
    assert!(!lines.contains(&"*-score.md"), "{text}");
}

#[test]
fn hooks_install_dry_run_writes_nothing() {
    let dir = tempdir().unwrap();
    init_git_repo(dir.path());

    assert_command_success(
        Command::new(binary_path())
            .arg("hooks")
            .arg("install")
            .arg(dir.path())
            .arg("--dry-run"),
    );
    assert!(!dir.path().join(".git/jankurai/env").exists());
    assert!(!dir.path().join(".git/hooks/pre-commit").exists());
    assert!(!dir.path().join(".git/hooks/prepare-commit-msg").exists());
}

#[test]
fn hooks_install_yes_installs_local_hooks() {
    let dir = tempdir().unwrap();
    init_git_repo(dir.path());

    assert_command_success(
        Command::new(binary_path())
            .arg("hooks")
            .arg("install")
            .arg(dir.path())
            .arg("--yes"),
    );

    let pre_commit = dir.path().join(".git/hooks/pre-commit");
    let prepare = dir.path().join(".git/hooks/prepare-commit-msg");
    assert!(pre_commit.is_file());
    assert!(prepare.is_file());
    assert!(dir.path().join(".git/jankurai/env").is_file());
    let pre_commit_text = fs::read_to_string(pre_commit).unwrap();
    assert!(pre_commit_text.contains("JANKURAI MANAGED HOOK: pre-commit"));
    assert!(
        pre_commit_text.contains("JANKURAI_HOOK_REPORT_DIR"),
        "{pre_commit_text}"
    );
    assert!(
        pre_commit_text.contains("JANKURAI_HOOK_STAGE_ARTIFACTS"),
        "{pre_commit_text}"
    );
    let prepare_text = fs::read_to_string(prepare).unwrap();
    assert!(prepare_text.contains("JANKURAI MANAGED HOOK: prepare-commit-msg"));
    assert!(
        prepare_text.contains("target/jankurai/hooks/pre-commit-score.json"),
        "{prepare_text}"
    );
}

#[test]
fn hooks_install_backs_up_and_chains_existing_hooks() {
    let dir = tempdir().unwrap();
    init_git_repo(dir.path());
    let hook_path = dir.path().join(".git/hooks/pre-commit");
    fs::write(&hook_path, "#!/usr/bin/env bash\necho user hook\n").unwrap();

    assert_command_success(
        Command::new(binary_path())
            .arg("hooks")
            .arg("install")
            .arg(dir.path())
            .arg("--yes"),
    );

    let env = fs::read_to_string(dir.path().join(".git/jankurai/env")).unwrap();
    assert!(env.contains("JANKURAI_PRE_COMMIT_CHAIN="), "{env}");
    let backups = fs::read_dir(dir.path().join(".git/jankurai/hooks"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(backups.len(), 1);
    assert!(fs::read_to_string(backups[0].path())
        .unwrap()
        .contains("user hook"));
}

#[test]
fn init_bootstrap_commit_installs_hooks_and_commit_score_trailers() {
    let dir = tempdir().unwrap();
    init_git_repo(dir.path());
    fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "fixture-bootstrap"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    let output = Command::new(binary_path())
        .arg("init")
        .arg(dir.path())
        .arg("--profile")
        .arg("rust-api")
        .arg("--bootstrap-commit")
        .arg("--yes")
        .arg("--bootstrap-message")
        .arg("Adopt test Jankurai")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "init bootstrap failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(dir.path().join(".git/hooks/pre-commit").is_file());
    assert!(dir.path().join(".git/hooks/prepare-commit-msg").is_file());
    assert!(dir.path().join("tools/jankurai-rust/witness.sh").is_file());
    let justfile = fs::read_to_string(dir.path().join("Justfile")).unwrap();
    assert!(justfile.contains("rust-map:"));
    assert!(justfile.contains("rust-witness:"));
    let first_message = git_stdout(dir.path(), &["log", "-1", "--format=%B"]);
    assert!(first_message.contains("Jankurai-Score:"), "{first_message}");
    assert!(
        first_message.contains("Jankurai-Report: target/jankurai/hooks/pre-commit-score.json"),
        "{first_message}"
    );

    fs::write(
        dir.path().join("docs/architecture/README.md"),
        "# Architecture\n\nCommit hook proof.\n",
    )
    .unwrap();
    git(dir.path(), &["add", "docs/architecture/README.md"]);
    git(dir.path(), &["commit", "-m", "Touch architecture docs"]);

    let second_message = git_stdout(dir.path(), &["log", "-1", "--format=%B"]);
    assert!(
        second_message.contains("Jankurai-Score:"),
        "{second_message}"
    );
    assert!(dir.path().join(".git/jankurai/last-score.env").is_file());
    assert!(dir
        .path()
        .join("target/jankurai/hooks/pre-commit-score.json")
        .is_file());
    assert!(dir
        .path()
        .join("target/jankurai/hooks/pre-commit-score.md")
        .is_file());
    let history = fs::read_to_string(
        dir.path()
            .join("target/jankurai/hooks/pre-commit-score-history.jsonl"),
    )
    .unwrap();
    assert!(
        history
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()
            >= 2,
        "{history}"
    );
}

#[test]
fn init_yolo_alias_is_rejected() {
    let dir = tempdir().unwrap();
    init_git_repo(dir.path());
    let output = Command::new(binary_path())
        .arg("init")
        .arg(dir.path())
        .arg("--yolo")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unexpected argument '--yolo'"), "{stderr}");
}

#[test]
fn init_plan_paths_match_profile_manifest_for_all_bundled_profiles() {
    for profile in BUNDLED_PROFILE_IDS {
        let dir = tempdir().unwrap();
        let plan_path = dir.path().join(format!("plan-{profile}.json"));
        init::run(dry_run_plan_args(
            dir.path().to_path_buf(),
            profile,
            Some(plan_path.to_string_lossy().into_owned()),
        ))
        .unwrap_or_else(|e| panic!("profile {profile}: {e:#}"));

        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
        let mut expected: Vec<_> = value["profile_manifest"]["generated_paths"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p.as_str().map(String::from))
            .collect();
        expected.sort();

        let create_paths: Vec<_> = value["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|a| {
                if a["action"] == "create" {
                    a["path"].as_str().map(String::from)
                } else {
                    None
                }
            })
            .collect();

        let mut sorted_create = create_paths.clone();
        sorted_create.sort();
        assert_eq!(
            sorted_create, expected,
            "profile {profile}: every generated path should be a create action when missing"
        );
    }
}

#[test]
fn init_profile_aliases_resolve_in_plan() {
    let dir = tempdir().unwrap();
    let plan_path = dir.path().join("plan.json");
    init::run(dry_run_plan_args(
        dir.path().to_path_buf(),
        "ai",
        Some(plan_path.to_string_lossy().into_owned()),
    ))
    .unwrap();
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
    assert_eq!(value["profile_manifest"]["id"], "ai-product");
}

#[test]
fn init_greenfield_apply_then_audit_and_doctor() {
    let dir = tempdir().unwrap();
    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
    ))
    .unwrap();

    let report = audit::run_audit(dir.path(), &[]).unwrap();
    assert!(dir.path().join("contracts/README.md").exists());
    assert!(dir.path().join("apps/web/AGENTS.md").exists());
    assert!(dir.path().join("apps/api/AGENTS.md").exists());
    assert!(dir.path().join("contracts/AGENTS.md").exists());
    assert!(dir.path().join("crates/domain/AGENTS.md").exists());
    assert!(dir.path().join("crates/application/AGENTS.md").exists());
    assert!(dir.path().join("crates/adapters/AGENTS.md").exists());
    assert!(dir.path().join("crates/workers/AGENTS.md").exists());
    assert!(dir.path().join("db/AGENTS.md").exists());
    assert!(dir.path().join("ops/AGENTS.md").exists());
    assert!(dir.path().join("python/ai-service/AGENTS.md").exists());
    assert!(dir.path().join("tools/security-lane.sh").exists());
    assert!(dir.path().join("agent/tool-adoption.toml").exists());
    assert!(dir.path().join("db/README.md").exists());
    for cell_id in [
        "web",
        "api",
        "domain",
        "application",
        "adapters",
        "workers",
        "contracts",
        "db",
        "ops",
        "python-ai",
    ] {
        let cell = report
            .profile_structure
            .cells
            .iter()
            .find(|cell| cell.id == cell_id)
            .unwrap();
        assert_eq!(cell.status, "canonical", "{cell_id} should be canonical");
        assert_eq!(
            cell.guidance_status, "present",
            "{cell_id} should have local guidance in the generated scaffold"
        );
    }
    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.rule_id.as_deref() != Some("HLT-038-REFERENCE-PROFILE-STRUCTURE-GAP")),
        "generated scaffold with canonical cell guidance should not emit profile-structure findings"
    );

    assert_audit_and_doctor(dir.path());
}

#[test]
fn init_respects_existing_contracts_readme() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("contracts")).unwrap();
    fs::write(
        dir.path().join("contracts/README.md"),
        "# Our contracts\nlegacy line\n",
    )
    .unwrap();

    init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: false,
        yes: true,
        profile: "rust-ts-postgres".into(),
        profile_file: None,
        level: "full".into(),
        ide: "all".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: false,
        plan_json: None,
        force_generated_adapters: false,
    })
    .unwrap();

    let text = fs::read_to_string(dir.path().join("contracts/README.md")).unwrap();
    assert!(
        text.contains("legacy line"),
        "existing contracts README must not be overwritten"
    );
    assert!(!text.contains("Put OpenAPI"));
}

#[test]
fn init_greenfield_apply_rust_api_then_audit_and_doctor() {
    let dir = tempdir().unwrap();
    init::run(greenfield_apply_args(dir.path().to_path_buf(), "rust-api")).unwrap();

    assert!(dir.path().join("contracts/README.md").exists());
    assert!(dir.path().join("apps/api/AGENTS.md").exists());
    assert!(dir.path().join("contracts/AGENTS.md").exists());
    assert!(dir.path().join("crates/domain/AGENTS.md").exists());
    assert!(dir.path().join("crates/application/AGENTS.md").exists());
    assert!(dir.path().join("tools/security-lane.sh").exists());
    assert!(dir.path().join("db/README.md").exists());
    assert!(dir.path().join("db/AGENTS.md").exists());
    assert!(dir.path().join("ops/AGENTS.md").exists());
    assert!(
        !dir.path().join("agent/ux-qa.toml").exists(),
        "rust-api does not include UX QA"
    );

    assert_audit_and_doctor(dir.path());
}

#[test]
fn init_greenfield_apply_react_web_then_audit_and_doctor() {
    let dir = tempdir().unwrap();
    init::run(greenfield_apply_args(dir.path().to_path_buf(), "react-web")).unwrap();

    assert!(dir.path().join("contracts/README.md").exists());
    assert!(dir.path().join("apps/web/AGENTS.md").exists());
    assert!(dir.path().join("contracts/AGENTS.md").exists());
    assert!(dir.path().join("tools/security-lane.sh").exists());
    assert!(dir.path().join("agent/tool-adoption.toml").exists());
    assert!(dir.path().join("agent/ux-qa.toml").exists());
    assert!(dir.path().join("ops/AGENTS.md").exists());
    assert!(
        !dir.path().join("db/README.md").exists(),
        "react-web does not include DB path"
    );

    assert_audit_and_doctor(dir.path());
}

#[test]
fn init_greenfield_apply_b2b_saas_then_audit_and_doctor() {
    let dir = tempdir().unwrap();
    init::run(greenfield_apply_args(dir.path().to_path_buf(), "b2b-saas")).unwrap();

    assert!(dir.path().join("contracts/README.md").exists());
    assert!(dir.path().join("apps/web/AGENTS.md").exists());
    assert!(dir.path().join("apps/api/AGENTS.md").exists());
    assert!(dir.path().join("contracts/AGENTS.md").exists());
    assert!(dir.path().join("tools/security-lane.sh").exists());
    assert!(dir.path().join("agent/ux-qa.toml").exists());
    assert!(dir.path().join("db/README.md").exists());
    assert!(dir.path().join("db/AGENTS.md").exists());
    assert!(dir.path().join("ops/AGENTS.md").exists());
    assert!(
        dir.path().join("docs/auth/README.md").exists(),
        "b2b-saas includes auth docs"
    );
    assert!(
        dir.path().join("docs/orgs/README.md").exists(),
        "b2b-saas includes orgs docs"
    );

    assert_audit_and_doctor(dir.path());
}

#[test]
fn init_greenfield_apply_ai_product_then_audit_and_doctor() {
    let dir = tempdir().unwrap();
    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "ai-product",
    ))
    .unwrap();

    assert!(dir.path().join("contracts/README.md").exists());
    assert!(dir.path().join("contracts/AGENTS.md").exists());
    assert!(dir.path().join("db/AGENTS.md").exists());
    assert!(dir.path().join("python/ai-service/README.md").exists());
    assert!(dir.path().join("python/ai-service/AGENTS.md").exists());
    assert!(dir.path().join("prompts/README.md").exists());
    assert!(dir.path().join("agent/tool-adoption.toml").exists());
    assert!(dir.path().join("ops/AGENTS.md").exists());
    assert!(
        !dir.path().join("agent/ux-qa.toml").exists(),
        "ai-product omits web UX controls by default"
    );

    assert_audit_and_doctor(dir.path());
}

#[test]
fn init_greenfield_apply_regulated_saas_then_audit_and_doctor() {
    let dir = tempdir().unwrap();
    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "regulated-saas",
    ))
    .unwrap();

    assert!(dir.path().join("apps/web/AGENTS.md").exists());
    assert!(dir.path().join("apps/api/AGENTS.md").exists());
    assert!(dir.path().join("contracts/AGENTS.md").exists());
    assert!(dir.path().join("db/AGENTS.md").exists());
    assert!(dir.path().join("docs/privacy/README.md").exists());
    assert!(dir.path().join("docs/compliance/README.md").exists());
    assert!(dir.path().join("agent/tool-adoption.toml").exists());
    assert!(dir.path().join("ops/AGENTS.md").exists());
    assert!(dir.path().join("agent/ux-qa.toml").exists());

    assert_audit_and_doctor(dir.path());
}

#[test]
fn init_greenfield_apply_migration_target_then_audit_and_doctor() {
    let dir = tempdir().unwrap();
    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "migration-target",
    ))
    .unwrap();

    assert!(dir.path().join("contracts/AGENTS.md").exists());
    assert!(dir.path().join("docs/migration/boundary-map.md").exists());
    assert!(dir.path().join("agent/tool-adoption.toml").exists());
    assert!(dir.path().join("ops/AGENTS.md").exists());
    assert!(
        !dir.path().join("db/README.md").exists(),
        "migration-target does not claim database ownership"
    );

    assert_audit_and_doctor(dir.path());
}

#[test]
fn init_profile_file_loads_manifest_from_disk() {
    let dir = tempdir().unwrap();
    let profile_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("templates/profiles/rust-api.json");
    let plan_path = dir.path().join("plan.json");
    init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: true,
        yes: false,
        profile: "this-value-is-ignored-when-profile-file-is-set".into(),
        profile_file: Some(profile_path),
        level: "full".into(),
        ide: "all".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: false,
        plan_json: Some(plan_path.to_string_lossy().into_owned()),
        force_generated_adapters: false,
    })
    .unwrap();

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
    assert_eq!(value["profile"], "rust-api");
    assert_eq!(value["profile_manifest"]["id"], "rust-api");
}

#[test]
fn init_profile_file_rejects_invalid_manifest() {
    let dir = tempdir().unwrap();
    let bad = dir.path().join("bad-profile.json");
    fs::write(&bad, r#"{"id": "only-id"}"#).unwrap();
    let err = init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: true,
        yes: false,
        profile: "rust-api".into(),
        profile_file: Some(bad),
        level: "full".into(),
        ide: "all".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: false,
        plan_json: None,
        force_generated_adapters: false,
    })
    .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("displayName") || msg.contains("required") || msg.contains("schema"),
        "{msg}"
    );
}

#[test]
fn init_profile_file_rejects_merge_policy_for_non_generated_path() {
    let dir = tempdir().unwrap();
    let bad = dir.path().join("bad-merge-policy-profile.json");
    let manifest = serde_json::json!({
        "id": "bad-merge-policy",
        "displayName": "Bad merge policy",
        "targetStackId": "test-stack",
        "generatedPaths": ["contracts/README.md"],
        "mergePolicy": {
            "docs/not-generated.md": "merge-lines"
        },
        "requiredLanes": [],
        "optionalLanes": [],
        "agentAdapters": [],
        "ciTemplates": [],
        "docs": [],
        "securityControls": [],
        "uxControls": [],
        "contractSystem": [],
        "dbPolicy": [],
        "validationCommands": []
    });
    fs::write(&bad, serde_json::to_string_pretty(&manifest).unwrap()).unwrap();

    let err = init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: true,
        yes: false,
        profile: "rust-api".into(),
        profile_file: Some(bad),
        level: "full".into(),
        ide: "all".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: false,
        plan_json: None,
        force_generated_adapters: false,
    })
    .unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("mergePolicy declares"), "{msg}");
}

#[test]
fn init_profile_file_merge_policy_overrides_default_keep_existing() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("contracts")).unwrap();
    fs::write(
        dir.path().join("contracts/README.md"),
        "# Our contracts\nlegacy line\n",
    )
    .unwrap();

    let profile_path = dir.path().join("merge-policy-profile.json");
    let plan_path = dir.path().join("plan.json");
    let manifest = serde_json::json!({
        "id": "merge-policy-profile",
        "displayName": "Merge policy profile",
        "targetStackId": "test-stack",
        "generatedPaths": ["contracts/README.md"],
        "mergePolicy": {
            "contracts/README.md": "merge-lines"
        },
        "requiredLanes": [],
        "optionalLanes": [],
        "agentAdapters": [],
        "ciTemplates": [],
        "docs": ["contracts/README.md"],
        "securityControls": [],
        "uxControls": [],
        "contractSystem": ["contracts/README.md"],
        "dbPolicy": [],
        "validationCommands": []
    });
    fs::write(
        &profile_path,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: true,
        yes: false,
        profile: "ignored-because-profile-file-is-set".into(),
        profile_file: Some(profile_path.clone()),
        level: "full".into(),
        ide: "all".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: false,
        plan_json: Some(plan_path.to_string_lossy().into_owned()),
        force_generated_adapters: false,
    })
    .unwrap();
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&plan_path).unwrap()).unwrap();
    assert_eq!(value["actions"][0]["action"], "merge-lines");

    let mut apply_args = greenfield_apply_args(dir.path().to_path_buf(), "ignored");
    apply_args.profile_file = Some(profile_path);
    init::run(apply_args).unwrap();

    let text = fs::read_to_string(dir.path().join("contracts/README.md")).unwrap();
    assert!(text.contains("legacy line"), "{text}");
    assert!(text.contains("Put OpenAPI"), "{text}");
}

#[test]
fn init_merges_existing_json() {
    let dir = tempdir().unwrap();
    let agent_dir = dir.path().join("agent");
    fs::create_dir_all(&agent_dir).unwrap();

    // Seed an existing owner-map.json
    fs::write(
        agent_dir.join("owner-map.json"),
        r#"{
  "schema": "https://jankurai.io/schemas/owner-map.schema.json",
  "version": 1,
  "owners": {
    "custom/": "my-custom-agent"
  }
}"#,
    )
    .unwrap();

    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
    ))
    .unwrap();

    let json_text = fs::read_to_string(agent_dir.join("owner-map.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json_text).unwrap();
    let owners = value["owners"].as_object().unwrap();

    let has_custom = owners
        .get("custom/")
        .is_some_and(|v| v == "my-custom-agent");
    let has_standard = owners.get("crates/").is_some_and(|v| v == "tools");

    assert!(has_custom, "must retain existing custom owner");
    assert!(
        has_standard,
        "must merge in standard crates owner from template"
    );
}

#[test]
fn init_merges_existing_lines_justfile() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Justfile"),
        "# existing custom recipe\n\ncustom:\n\t@echo preserved\n",
    )
    .unwrap();

    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
    ))
    .unwrap();

    let text = fs::read_to_string(dir.path().join("Justfile")).unwrap();
    assert!(
        text.contains("preserved"),
        "must retain existing Justfile content: {text}"
    );
    assert!(
        text.contains("fast:") && text.contains("jankurai doctor --fail-on critical"),
        "must merge in scaffold recipes from template: {text}"
    );
}

#[test]
fn init_generated_templates_are_external_repo_safe() {
    let dir = tempdir().unwrap();
    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
    ))
    .unwrap();

    for rel in [
        "Justfile",
        ".github/workflows/jankurai.yml",
        "agent/generated-zones.toml",
        "agent/test-map.json",
        "agent/proof-lanes.toml",
    ] {
        let text = fs::read_to_string(dir.path().join(rel)).unwrap();
        assert!(
            !text.contains("cargo run -p jankurai") && !text.contains("cargo test -p jankurai"),
            "{rel} must not assume this source workspace: {text}"
        );
        assert!(
            !text.contains("\"true\"") && !text.contains("command = \"true\""),
            "{rel} must not use false-green noop proof lanes: {text}"
        );
    }

    let justfile = fs::read_to_string(dir.path().join("Justfile")).unwrap();
    for recipe in ["fast:", "score:", "doctor:", "security:", "check:"] {
        assert!(justfile.contains(recipe), "missing {recipe}: {justfile}");
    }

    for rel in [
        ".agents/skills/jankurai/SKILL.md",
        ".claude/skills/jankurai/SKILL.md",
    ] {
        let text = fs::read_to_string(dir.path().join(rel)).unwrap();
        assert!(
            text.starts_with("---\nname: jankurai\n"),
            "{rel} must start with YAML skill frontmatter: {text}"
        );
        assert!(
            text.contains("\ndescription: Jankurai workspace guidance")
                && text.contains("\n---\n\n# jankurai"),
            "{rel} must contain a complete skill frontmatter block: {text}"
        );
    }
}

#[test]
fn init_adapter_sync_includes_command_workflows() {
    let dir = tempdir().unwrap();

    let all_plan = adapters::adapter_plan(dir.path(), "all");
    let all_paths: Vec<_> = all_plan.iter().map(|action| action.path.as_str()).collect();
    for expected in [
        ".agents/workflows/jankurai-kickoff.md",
        ".agents/workflows/jankurai-context-pack.md",
        ".agents/workflows/jankurai-prove.md",
        ".agents/workflows/jankurai-witness.md",
        ".agents/workflows/jankurai-repair-plan.md",
    ] {
        assert!(
            all_paths.contains(&expected),
            "missing {expected} from adapter plan"
        );
    }

    let agents_plan = adapters::adapter_plan(dir.path(), "antigravity");
    let agents_paths: Vec<_> = agents_plan
        .iter()
        .map(|action| action.path.as_str())
        .collect();
    for expected in [
        ".agents/workflows/jankurai-kickoff.md",
        ".agents/workflows/jankurai-context-pack.md",
        ".agents/workflows/jankurai-prove.md",
        ".agents/workflows/jankurai-witness.md",
        ".agents/workflows/jankurai-repair-plan.md",
    ] {
        assert!(
            agents_paths.contains(&expected),
            "missing {expected} from antigravity plan"
        );
    }

    adapters::write_adapters(dir.path(), "antigravity", true).unwrap();
    for (rel, marker) in [
        (
            ".agents/workflows/jankurai-kickoff.md",
            "jankurai kickoff . --intent",
        ),
        (
            ".agents/workflows/jankurai-context-pack.md",
            "jankurai context-pack . --changed",
        ),
        (
            ".agents/workflows/jankurai-prove.md",
            "jankurai prove . --changed",
        ),
        (
            ".agents/workflows/jankurai-witness.md",
            "jankurai witness . --changed-from",
        ),
        (
            ".agents/workflows/jankurai-repair-plan.md",
            "jankurai repair-plan . --from",
        ),
    ] {
        let text = fs::read_to_string(dir.path().join(rel)).unwrap();
        assert!(text.contains("jankurai generated adapter"), "{rel}: {text}");
        assert!(text.contains("agent/JANKURAI_STANDARD.md"), "{rel}: {text}");
        assert!(
            text.contains("When a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan."),
            "{rel}: {text}"
        );
        assert!(text.contains("If jankurai is installed"), "{rel}: {text}");
        assert!(text.contains(marker), "{rel}: {text}");
    }
}

#[test]
fn init_repairs_generated_skill_adapters_missing_frontmatter() {
    let dir = tempdir().unwrap();
    let skill = dir.path().join(".agents/skills/jankurai/SKILL.md");
    fs::create_dir_all(skill.parent().unwrap()).unwrap();
    fs::write(
        &skill,
        "# jankurai\n\n<!-- jankurai generated adapter -->\nRead `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\nWhen a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.\nFor explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.\nFor explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.\nRun the proof lane in `agent/test-map.json` for changed paths.\n",
    )
    .unwrap();

    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
    ))
    .unwrap();

    let text = fs::read_to_string(skill).unwrap();
    assert!(
        text.starts_with("---\nname: jankurai\n"),
        "generated skill adapter should be repaired with frontmatter: {text}"
    );
}

#[test]
fn init_merges_existing_toml() {
    let dir = tempdir().unwrap();
    let agent_dir = dir.path().join("agent");
    fs::create_dir_all(&agent_dir).unwrap();

    // Seed an existing proof-lanes.toml
    fs::write(
        agent_dir.join("proof-lanes.toml"),
        r#"
schema = "https://jankurai.io/schemas/proof-lanes.schema.json"
version = 1

[[lane]]
name = "custom-lane"
command = "echo custom"
"#,
    )
    .unwrap();

    init::run(greenfield_apply_args(
        dir.path().to_path_buf(),
        "rust-ts-postgres",
    ))
    .unwrap();

    let toml_text = fs::read_to_string(agent_dir.join("proof-lanes.toml")).unwrap();
    let value: toml::Value = toml::from_str(&toml_text).unwrap();
    let lanes = value["lane"].as_array().unwrap();

    let has_custom = lanes
        .iter()
        .any(|l| l["name"].as_str() == Some("custom-lane"));
    let has_standard = lanes.iter().any(|l| l["name"].as_str() == Some("fast"));

    assert!(has_custom, "must retain existing custom lane");
    assert!(
        has_standard,
        "must merge in standard fast lane from template"
    );
}
