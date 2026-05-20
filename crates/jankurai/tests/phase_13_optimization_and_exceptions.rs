use jankurai::validation::{self, ArtifactSchema};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

fn seed_release_data(repo: &Path) {
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(
        repo.join("agent/standard-version.toml"),
        r#"
standard = "jankurai"
standard_version = "0.5.0"
paper_edition = "2026.05-ed4"
auditor_version = "0.5.0"
schema_version = "1.3.0"
target_stack = "rust-ts-vite-react-postgres-bounded-python"
published = "2026-05-02"
"#,
    )
    .unwrap();
}

fn seed_optimize_repo(repo: &Path) {
    seed_release_data(repo);
    fs::write(
        repo.join("AGENTS.md"),
        "shared optimization guidance line\n",
    )
    .unwrap();
    fs::write(
        repo.join("CLAUDE.md"),
        "shared optimization guidance line\n",
    )
    .unwrap();
    fs::write(
        repo.join("Cargo.toml"),
        r#"
[package]
name = "fixture-app"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
serde = "1"
"#,
    )
    .unwrap();
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(repo.join("src/lib.rs"), "pub fn ready() {}\n").unwrap();
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::create_dir_all(repo.join(".jankurai")).unwrap();
    fs::write(
        repo.join(".jankurai/repo-score.json"),
        json!({
            "findings": [
                {
                    "path": "src/lib.rs",
                    "problem": "performance regression in hot path",
                    "rule_id": "HLT-999-PERF",
                    "severity": "medium",
                    "check_id": "perf-check",
                    "agent_fix": "measure the hot path and narrow the regression"
                },
                {
                    "path": "src/lib.rs",
                    "problem": "placeholder stub code remains",
                    "rule_id": "HLT-001-DEAD-MARKER",
                    "severity": "high",
                    "check_id": "dead-check",
                    "agent_fix": "replace the placeholder with implemented behavior"
                }
            ]
        })
        .to_string(),
    )
    .unwrap();
}

fn seed_clean_exception_repo(repo: &Path) {
    seed_release_data(repo);
    let exceptions = repo.join("docs/exceptions");
    fs::create_dir_all(&exceptions).unwrap();
    fs::write(
        exceptions.join("0001-current-only.md"),
        r#"---
code: HB_SQL_SHIM
owner: data-platform
reason: Brownfield SQL shim is still in migration.
expires: 2026-12-31
migration_plan: Move the SQL into the adapter and retire the shim.
proof_lane: just score
---
# current only
"#,
    )
    .unwrap();
}

fn seed_exception_repo(repo: &Path) {
    seed_release_data(repo);
    let exceptions = repo.join("docs/exceptions");
    fs::create_dir_all(&exceptions).unwrap();
    fs::write(
        exceptions.join("0001-expired.md"),
        r#"---
code: HB_CONTRACT_DRIFT
owner: platform
reason: Regenerated client still lags the contract.
expires: 2026-05-01
migration_plan: Regenerate the client and commit the generated diff.
proof_lane: just fast
repair_guidance: Renew only if the contract fix is blocked.
---
# expired
"#,
    )
    .unwrap();
    fs::write(
        exceptions.join("0002-current.md"),
        r#"---
code: HB_SQL_SHIM
owner: data-platform
reason: Brownfield SQL shim is still in migration.
expires: 2026-12-31
migration_plan: Move the SQL into the adapter and retire the shim.
proof_lane: just score
repair_guidance: Keep the exception narrow and dated.
---
# current
"#,
    )
    .unwrap();
    fs::write(
        exceptions.join("0003-invalid.md"),
        "# missing front matter\n",
    )
    .unwrap();
}

fn run_command(
    repo: &Path,
    subcommand: &str,
    args: &[&str],
    out_name: &str,
) -> (serde_json::Value, String) {
    let out_path = repo.join(out_name);
    let md_path = out_path.with_extension("md");
    let output = Command::new(binary_path())
        .arg(subcommand)
        .arg(repo)
        .args(args)
        .arg("--out")
        .arg(&out_path)
        .arg("--md")
        .arg(&md_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_path).unwrap()).unwrap();
    let md = fs::read_to_string(md_path).unwrap();
    (json, md)
}

#[test]
fn optimize_command_emits_schema_valid_report_with_candidates() {
    let repo = tempdir().unwrap();
    seed_optimize_repo(repo.path());

    let (report, md) = run_command(
        repo.path(),
        "optimize",
        &["--mode", "all"],
        "target/jankurai/optimization-report.json",
    );
    validation::validate_value(repo.path(), ArtifactSchema::OptimizationReport, &report).unwrap();
    assert_eq!(report["status"], "complete");
    assert_eq!(report["mode"], "all");
    assert!(
        report["context_size_before_bytes"].as_u64().unwrap()
            > report["context_size_after_bytes"].as_u64().unwrap()
    );
    assert!(report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["kind"] == "token"));
    assert!(report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["kind"] == "performance"));
    assert!(report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["kind"] == "dependency"));
    assert!(report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|finding| finding["kind"] == "dead-code"));
    assert!(report["proof_requirements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|proof| proof == "just bench"));
    assert!(md.starts_with("# jankurai Optimization Report"));
}

#[test]
fn exception_expiry_command_reports_expired_and_invalid_docs() {
    let repo = tempdir().unwrap();
    seed_exception_repo(repo.path());

    let out_path = repo
        .path()
        .join("target/jankurai/exception-expiry-report.json");
    let md_path = out_path.with_extension("md");
    let output = Command::new(binary_path())
        .arg("exceptions")
        .arg("expire")
        .arg(repo.path())
        .arg("--warning-days")
        .arg("7")
        .arg("--out")
        .arg(&out_path)
        .arg("--md")
        .arg(&md_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_path).unwrap()).unwrap();
    let md = fs::read_to_string(md_path).unwrap();
    validation::validate_value(repo.path(), ArtifactSchema::ExceptionExpiryReport, &report)
        .unwrap();
    assert_eq!(report["status"], "blocked");
    assert_eq!(report["expired_count"], 1);
    assert_eq!(report["expiring_soon_count"], 0);
    assert_eq!(report["invalid_count"], 1);
    assert_eq!(report["total_exceptions"], 3);
    let exceptions = report["exceptions"].as_array().unwrap();
    assert!(exceptions
        .iter()
        .any(|entry| entry["status"] == "expired" && entry["code"] == "HB_CONTRACT_DRIFT"));
    assert!(exceptions
        .iter()
        .any(|entry| entry["status"] == "invalid"
            && entry["path"] == "docs/exceptions/0003-invalid.md"));
    assert!(exceptions
        .iter()
        .any(|entry| entry["status"] == "current" && entry["code"] == "HB_SQL_SHIM"));
    assert!(md.starts_with("# jankurai Exception Expiry"));
}

#[test]
fn exception_expire_strict_fails_when_blocked() {
    let repo = tempdir().unwrap();
    seed_exception_repo(repo.path());

    let out_path = repo
        .path()
        .join("target/jankurai/exception-expiry-strict.json");
    let md_path = out_path.with_extension("md");
    let output = Command::new(binary_path())
        .arg("exceptions")
        .arg("expire")
        .arg(repo.path())
        .arg("--warning-days")
        .arg("7")
        .arg("--strict")
        .arg("--out")
        .arg(&out_path)
        .arg("--md")
        .arg(&md_path)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "strict mode should exit non-zero when status is blocked"
    );
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_path).unwrap()).unwrap();
    assert_eq!(report["status"], "blocked");
}

#[test]
fn exception_expire_strict_passes_when_only_current_exceptions() {
    let repo = tempdir().unwrap();
    seed_clean_exception_repo(repo.path());

    let out_path = repo.path().join("target/jankurai/exception-clean.json");
    let md_path = out_path.with_extension("md");
    let output = Command::new(binary_path())
        .arg("exceptions")
        .arg("expire")
        .arg(repo.path())
        .arg("--warning-days")
        .arg("7")
        .arg("--strict")
        .arg("--out")
        .arg(&out_path)
        .arg("--md")
        .arg(&md_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_path).unwrap()).unwrap();
    assert_eq!(report["status"], "complete");
}
