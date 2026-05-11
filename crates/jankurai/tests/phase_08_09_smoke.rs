use jankurai::validation::{self, ArtifactSchema};
use serde_json::json;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn context_pack_command_writes_pack_and_markdown() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());
    fs::write(
        dir.path().join("AGENTS.md"),
        "Read `agent/JANKURAI_STANDARD.md` first.\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("agent/JANKURAI_STANDARD.md"),
        "Standard version: `0.5.0`\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("docs")).unwrap();
    fs::write(
        dir.path().join("docs/agent-native-standard.md"),
        "Read `agent/JANKURAI_STANDARD.md` first.\n",
    )
    .unwrap();
    fs::write(dir.path().join("docs/moonshot.md"), "# moonshot\n").unwrap();
    fs::write(dir.path().join("docs/boundary-oracle.md"), "# boundary\n").unwrap();
    fs::write(dir.path().join("docs/install.md"), "# install\n").unwrap();
    fs::write(dir.path().join("docs/ide-integrations.md"), "# ide\n").unwrap();
    fs::create_dir_all(dir.path().join("tips/phases")).unwrap();
    fs::write(
        dir.path().join("tips/phases/08-agent-context-repair.md"),
        "# phase 08\n",
    )
    .unwrap();

    let json_out = dir.path().join("target/jankurai/context-pack.json");
    let md_out = dir.path().join("target/jankurai/context-pack.md");
    assert!(Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .arg("context-pack")
        .arg(dir.path())
        .arg("--task")
        .arg("repair agent context routing")
        .arg("--changed")
        .arg("agent/JANKURAI_STANDARD.md")
        .arg("--out")
        .arg(&json_out)
        .arg("--md")
        .arg(&md_out)
        .status()
        .unwrap()
        .success());

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_out).unwrap()).unwrap();
    validation::validate_value(dir.path(), ArtifactSchema::ContextPack, &value).unwrap();
    assert_eq!(value["schema_version"], "1.2.0");
    assert_eq!(value["token_budget"], 6000);
    assert!(value["included_files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["source_trust"] == "trusted-policy" && item["path"] == "agent/JANKURAI_STANDARD.md"
        }));
    assert_eq!(value["owner"], "agent");
    assert_eq!(value["permission_profile"], "code-edit");
    assert!(value["allowed_paths"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "agent/"));
    assert!(value["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "fast"));
    assert!(value["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "audit"));
    assert!(value["commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "just fast"));
    assert_eq!(value["human_approval_required"], false);
    let decisions = value["scope_decisions"].as_array().unwrap();
    let agent_decision = decisions
        .iter()
        .find(|decision| decision["path"] == "agent/JANKURAI_STANDARD.md")
        .expect("scope decision for changed agent file");
    assert_eq!(agent_decision["decision"], "allowed");
    assert_eq!(agent_decision["owner"], "agent");
    assert_eq!(agent_decision["owner_route"], "agent/");
    assert_eq!(agent_decision["proof_lane"], "test-map");
    assert!(!agent_decision["generated_zone"].as_bool().unwrap());
    let markdown = fs::read_to_string(md_out).unwrap();
    assert!(markdown.contains("# jankurai Context Pack"));
    assert!(markdown.contains("## Scope decisions"));
}

#[test]
fn context_pack_flags_generated_changed_paths_for_human_review() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());

    let out = dir.path().join("target/jankurai/context-pack.json");
    assert!(Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .arg("context-pack")
        .arg(dir.path())
        .arg("--task")
        .arg("update generated repo score evidence")
        .arg("--changed")
        .arg("agent/repo-score.json")
        .arg("--out")
        .arg(&out)
        .status()
        .unwrap()
        .success());

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
    validation::validate_value(dir.path(), ArtifactSchema::ContextPack, &value).unwrap();
    assert_eq!(value["human_approval_required"], true);
    assert!(value["human_approval_reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str().unwrap().contains("generated output")));
    let decision = &value["scope_decisions"][0];
    assert_eq!(decision["path"], "agent/repo-score.json");
    assert_eq!(decision["decision"], "read-only");
    assert!(decision["generated_zone"].as_bool().unwrap());
    assert_eq!(decision["generated_source"], "crates/jankurai");
    assert_eq!(
        decision["generated_command"],
        "cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md"
    );
    assert!(value["stop_conditions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str().unwrap().contains("generated output")));
}

#[test]
fn repair_plan_command_writes_packets() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());
    fs::create_dir_all(dir.path().join("target/jankurai")).unwrap();
    let report_path = dir.path().join("target/jankurai/repo-score.json");
    let report = json!({
        "findings": [{
            "severity": "high",
            "category": "security",
            "path": "AGENTS.md",
            "problem": "policy bypass language",
            "agent_fix": "tighten policy",
            "evidence": ["policy bypass language"],
            "check_id": "demo",
            "hardness": "hard",
            "confidence": 0.9,
            "evidence_kind": "text",
            "rerun_command": "just security",
            "fingerprint": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "rule_id": "HLT-011-PROMPT-INJECTION",
            "owner": "agent",
            "lane": "security"
        }]
    });
    fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

    let out = dir.path().join("target/jankurai/repair-plan.json");
    let md = dir.path().join("target/jankurai/repair-plan.md");
    assert!(Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .arg("repair-plan")
        .arg(dir.path())
        .arg("--from")
        .arg(&report_path)
        .arg("--out")
        .arg(&out)
        .arg("--md")
        .arg(&md)
        .status()
        .unwrap()
        .success());

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
    validation::validate_value(dir.path(), ArtifactSchema::RepairPlan, &value).unwrap();
    assert_eq!(value["plan_mode"], "dry-run");
    assert_eq!(value["planned_edits"][0]["operation"], "review-only");
    assert_eq!(value["planned_edits"][0]["risk_level"], "high");
    assert_eq!(
        value["planned_edits"][0]["repair_eligibility"],
        "human-required"
    );
    assert!(value["planned_commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "just security"));
    assert!(value["proof_lanes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "security"));
    assert!(!value["human_approval_requirements"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(value["packets"].as_array().unwrap().len(), 1);
    let packet = &value["packets"][0];
    assert_eq!(packet["permission_profile"], "security-investigation");
    assert_eq!(packet["repair_eligibility"], "human-required");
    assert_eq!(packet["risk_level"], "high");
    assert!(packet["eligibility_reason"]
        .as_str()
        .unwrap()
        .contains("prompt injection"));
    assert!(packet["human_review_required"].as_bool().unwrap());
    assert!(packet["required_proof"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "just security"));
    assert!(fs::read_to_string(md)
        .unwrap()
        .contains("# jankurai Repair Plan"));
}

#[test]
fn repair_plan_schema_rejects_missing_packets() {
    let dir = tempdir().unwrap();
    seed_catalog(dir.path());
    let bad = json!({
        "schema_version": "1.0.0",
        "source_report": "agent/repo-score.json",
        "generated_at": "0",
        "target_stack_id": "jankurai:v0.4",
    });
    let err = validation::validate_value(dir.path(), ArtifactSchema::RepairPlan, &bad).unwrap_err();
    assert!(
        err.to_string().contains("packets"),
        "expected missing packets error, got {err:?}"
    );
}

#[test]
fn agent_verify_command_succeeds_on_canonical_adapter() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("CLAUDE.md"),
        "Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\nWhen a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.\nFor explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.\nFor explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.\n",
    )
    .unwrap();

    assert!(Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .arg("agent")
        .arg("verify")
        .arg(dir.path())
        .status()
        .unwrap()
        .success());
}

#[test]
fn agent_verify_rejects_adapter_missing_planner_protocol_pointer() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("agent")).unwrap();
    fs::write(
        dir.path().join("CLAUDE.md"),
        "Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.\nWhen a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.\nFor explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .arg("agent")
        .arg("verify")
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("planner protocol"),
        "expected planner protocol failure, got {stdout}"
    );
}

fn seed_catalog(repo: &std::path::Path) {
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(
        repo.join("agent/owner-map.json"),
        r#"{"workspace":"fixture","owners":{"agent/":"agent","docs/":"standard","tips/":"paper","target/":"workspace"}}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/test-map.json"),
        r#"{"workspace":"fixture","tests":{"agent/":{"command":"cargo test -p jankurai","purpose":"agent checks"},"docs/":{"command":"cargo run -p jankurai -- . --json agent/repo-score.json --md agent/repo-score.md","purpose":"audit"}}}"#,
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
    fs::write(
        repo.join("agent/proof-lanes.toml"),
        r#"[[lane]]
name = "fast"
command = "just fast"
purpose = "fast lane"

[[lane]]
name = "audit"
command = "just score"
purpose = "audit lane"

[[lane]]
name = "security"
command = "just security"
purpose = "security lane"

[[lane]]
name = "full"
command = "just check"
purpose = "full lane"
"#,
    )
    .unwrap();
}
