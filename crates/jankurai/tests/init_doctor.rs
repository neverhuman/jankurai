use jankurai::{audit, commands::init};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn assert_command_success(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn init_dry_run_writes_nothing() {
    let dir = tempdir().unwrap();
    init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: true,
        yes: false,
        profile: "rust-ts-vite-react-postgres".into(),
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

    assert!(!dir.path().join("AGENTS.md").exists());
    assert!(!dir.path().join("target/jankurai/receipts").exists());
}

#[test]
fn generated_adapters_are_protected_unless_force_refresh_is_requested() {
    let dir = tempdir().unwrap();
    let adapter = dir.path().join("CLAUDE.md");
    fs::write(
        &adapter,
        "<!-- jankurai generated adapter -->\ncustom local edit\n",
    )
    .unwrap();

    init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: false,
        yes: true,
        profile: "rust-ts-vite-react-postgres".into(),
        profile_file: None,
        level: "agents".into(),
        ide: "claude".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: false,
        plan_json: None,
        force_generated_adapters: false,
    })
    .unwrap();
    assert!(fs::read_to_string(&adapter)
        .unwrap()
        .contains("custom local edit"));

    init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: false,
        yes: true,
        profile: "rust-ts-vite-react-postgres".into(),
        profile_file: None,
        level: "agents".into(),
        ide: "claude".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: false,
        plan_json: None,
        force_generated_adapters: true,
    })
    .unwrap();
    let refreshed = fs::read_to_string(&adapter).unwrap();
    assert!(!refreshed.contains("custom local edit"));
    assert!(refreshed.contains("explicit MASTER_PLAN/phase work only"));
}

#[test]
fn init_v061_artifacts_are_idempotent() {
    let dir = tempdir().unwrap();
    for _ in 0..2 {
        init::run(init::InitArgs {
            repo: dir.path().to_path_buf(),
            apply: false,
            dry_run: false,
            yes: true,
            profile: "rust-ts-vite-react-postgres".into(),
            profile_file: None,
            level: "score".into(),
            ide: "none".into(),
            mode: "advisory".into(),
            diff: false,
            ci: "github".into(),
            issue_backend: "jsonl".into(),
            ux_qa: false,
            plan_json: None,
            force_generated_adapters: false,
        })
        .unwrap();
    }

    let standard = fs::read_to_string(dir.path().join("agent/standard-version.toml")).unwrap();
    assert!(standard.contains("standard_version = \"0.8.0\""));
    assert!(standard.contains("schema_version = \"1.7.0\""));
    assert_eq!(standard.matches("standard_version").count(), 1);
}

#[test]
fn audit_omits_vibe_coverage_when_source_is_absent() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "# fixture\n").unwrap();
    let report = audit::run_audit(dir.path(), &[]).unwrap();
    assert!(report.vibe_coverage.is_none());
}

#[test]
fn init_yes_is_idempotent_for_existing_root_guidance() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("AGENTS.md"), "# Existing\n").unwrap();

    for _ in 0..2 {
        init::run(init::InitArgs {
            repo: dir.path().to_path_buf(),
            apply: false,
            dry_run: false,
            yes: true,
            profile: "rust-ts-vite-react-postgres".into(),
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
    }

    let agents = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert_eq!(agents.matches("jankurai merge marker").count(), 1);
    assert!(dir.path().join(".cursor/rules/jankurai.mdc").exists());
    assert!(dir.path().join("CLAUDE.md").exists());
    assert!(dir.path().join("GEMINI.md").exists());
    assert!(dir.path().join(".github/copilot-instructions.md").exists());
    assert!(dir.path().join(".agents/agents.md").exists());
    assert!(dir.path().join("target/jankurai/receipts").exists());
    for rel in [
        ".cursor/rules/jankurai.mdc",
        "CLAUDE.md",
        "GEMINI.md",
        ".github/copilot-instructions.md",
        ".agents/agents.md",
    ] {
        let text = fs::read_to_string(dir.path().join(rel)).unwrap();
        assert!(
            text.contains("agent/MASTER_PLAN.md#detailed-planner-protocol"),
            "generated adapter {rel} should point at detailed planner protocol"
        );
    }
}

#[test]
fn init_dry_run_plan_json_is_machine_readable() {
    let dir = tempdir().unwrap();
    let plan = dir.path().join("target/jankurai/init-plan.json");

    init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: true,
        yes: false,
        profile: "rust-ts-vite-react-postgres-bounded-python".into(),
        profile_file: None,
        level: "full".into(),
        ide: "all".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: true,
        plan_json: Some(plan.display().to_string()),
        force_generated_adapters: false,
    })
    .unwrap();

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(plan).unwrap()).unwrap();
    assert_eq!(value["profile"], "rust-ts-postgres");
    assert!(value["actions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|action| { action["path"] == "CLAUDE.md" && action["action"] == "create" }));
    assert!(!dir.path().join("AGENTS.md").exists());
}

#[test]
fn init_dry_run_profile_manifest_is_included() {
    let dir = tempdir().unwrap();
    let plan = dir.path().join("target/jankurai/init-plan.json");

    init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: true,
        yes: false,
        profile: "rust-ts-postgres".into(),
        profile_file: None,
        level: "full".into(),
        ide: "all".into(),
        mode: "advisory".into(),
        diff: false,
        ci: "github".into(),
        issue_backend: "jsonl".into(),
        ux_qa: true,
        plan_json: Some(plan.display().to_string()),
        force_generated_adapters: false,
    })
    .unwrap();

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(plan).unwrap()).unwrap();
    assert_eq!(value["profile"], "rust-ts-postgres");
    assert_eq!(value["profile_manifest"]["id"], "rust-ts-postgres");
    assert_eq!(
        value["profile_manifest"]["target_stack_id"],
        "rust-ts-vite-react-postgres-bounded-python"
    );
    assert!(value["profile_manifest"]["generated_paths"]
        .as_array()
        .unwrap()
        .iter()
        .any(|path| path == "agent/ux-qa.toml"));
    assert!(value["profile_manifest"]["validation_commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|cmd| cmd == "jankurai doctor --fail-on critical"));
}

#[test]
fn init_yes_keeps_existing_jankurai_guidance_without_marker() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("AGENTS.md"),
        "Read `agent/JANKURAI_STANDARD.md` first.\n",
    )
    .unwrap();

    init::run(init::InitArgs {
        repo: dir.path().to_path_buf(),
        apply: false,
        dry_run: false,
        yes: true,
        profile: "rust-ts-vite-react-postgres".into(),
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

    let agents = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(!agents.contains("jankurai merge marker"));
}

#[test]
fn cli_surfaces_smoke() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("AGENTS.md"), "Read agent standard\n").unwrap();
    fs::write(dir.path().join("README.md"), "# Repo\n").unwrap();
    fs::write(dir.path().join("Justfile"), "fast:\n    cargo test\n").unwrap();
    fs::create_dir_all(dir.path().join("db")).unwrap();
    fs::write(dir.path().join("db/README.md"), "# db\n").unwrap();
    fs::create_dir_all(dir.path().join("tools")).unwrap();
    fs::write(
        dir.path().join("tools/security-lane.sh"),
        "#!/bin/sh\nexit 0\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("agent/boundaries.toml"),
        r#"
[stack]
id = "test-stack"
[queues]
adapter_paths = []
event_contract_paths = []
generated_type_paths = []
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("agent/security-policy.toml"),
        r#"
schema_version = "1.0.0"
enabled_tools = ["gitleaks"]
required_tools = []
advisory_tools = ["gitleaks"]
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("agent/JANKURAI_STANDARD.md"),
        "Canonical agent standard stub for smoke test.\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("agent/owner-map.json"),
        r#"{"workspace":"fixture","owners":{"./":"workspace"}}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{".":{"command":"true","purpose":"smoke"}}}"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("agent/generated-zones.toml"),
        r#"[[zone]]
path = "agent/repo-score.json"
source = "fixture"
command = "true"
read_only = true
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("agent/proof-lanes.toml"),
        r#"[[lane]]
name = "fast"
command = "true"
purpose = "smoke"
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("agent/standard-version.toml"),
        r#"standard = "fixture"
standard_version = "0.0.0"
"#,
    )
    .unwrap();
    fs::write(dir.path().join("agent/repo-score.json"), "{}\n").unwrap();
    fs::write(dir.path().join("agent/repo-score.md"), "# score\n").unwrap();

    let json = dir.path().join("score.json");
    let md = dir.path().join("score.md");
    assert_command_success(
        Command::new(env!("CARGO_BIN_EXE_jankurai"))
            .arg("audit")
            .arg(dir.path())
            .arg("--mode")
            .arg("advisory")
            .arg("--json")
            .arg(&json)
            .arg("--md")
            .arg(&md),
    );
    assert!(json.exists());
    assert!(md.exists());

    let doctor_json = dir.path().join("doctor.json");
    let doctor_md = dir.path().join("doctor.md");
    assert_command_success(
        Command::new(env!("CARGO_BIN_EXE_jankurai"))
            .arg("doctor")
            .arg(dir.path())
            .arg("--fail-on")
            .arg("high")
            .arg("--json")
            .arg(&doctor_json)
            .arg("--md")
            .arg(&doctor_md),
    );
    let doctor_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&doctor_json).unwrap()).unwrap();
    let first_diag = doctor_value.as_array().unwrap().first().unwrap();
    assert!(first_diag.get("kind").is_some());
    assert!(first_diag.get("environment_sensitive").is_some());
    assert!(first_diag.get("strictly_blocking").is_some());
    assert!(first_diag.get("common_fixes").is_some());
    assert!(fs::read_to_string(&doctor_md)
        .unwrap()
        .contains("# jankurai doctor"));

    let issues = dir.path().join("issues.jsonl");
    assert_command_success(
        Command::new(env!("CARGO_BIN_EXE_jankurai"))
            .arg("issues")
            .arg("export")
            .arg(dir.path())
            .arg("--format")
            .arg("jsonl")
            .arg("--out")
            .arg(&issues),
    );
    assert!(issues.exists());

    let ci_dir = tempdir().unwrap();
    assert_command_success(
        Command::new(env!("CARGO_BIN_EXE_jankurai"))
            .arg("ci")
            .arg("install")
            .arg(ci_dir.path())
            .arg("--github")
            .arg("--mode")
            .arg("ratchet")
            .arg("--baseline")
            .arg("agent/repo-score.json")
            .arg("--min-score")
            .arg("85"),
    );
    let workflow =
        fs::read_to_string(ci_dir.path().join(".github/workflows/jankurai.yml")).unwrap();
    assert!(workflow.contains("target/jankurai/accepted-baseline.json"));
    assert!(workflow.contains("jankurai security run . --strict --profile ci"));
    assert!(workflow.contains("cargo install jankurai --locked"));
    assert!(workflow.contains("jankurai audit . --mode ratchet"));
    assert!(!workflow.contains("cargo run -p jankurai"));

    assert_command_success(
        Command::new(env!("CARGO_BIN_EXE_jankurai"))
            .arg("explain")
            .arg("HLT-003-OWNERLESS-PATH"),
    );
}
