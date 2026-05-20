//! Integration coverage for `jankurai audit-file` — the single-file save-gate
//! that powers the Jankurai Guard hook.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::tempdir;

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_jankurai")
}

/// Writes a believable target Rust repo (not the jankurai repo itself, so the
/// run does not need `--self-audit`).
fn write_target_repo(repo: &Path) {
    fs::write(
        repo.join("AGENTS.md"),
        "Read agent/JANKURAI_STANDARD.md first.\n",
    )
    .unwrap();
    fs::write(repo.join("README.md"), "# fixture app\n").unwrap();
    fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::write(
        repo.join("agent/JANKURAI_STANDARD.md"),
        "Standard version: `0.9.0`\n",
    )
    .unwrap();
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(
        repo.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .unwrap();
}

fn audit_file(repo: &Path, args: &[&str], stdin: Option<&[u8]>) -> std::process::Output {
    let mut cmd = Command::new(binary_path());
    cmd.arg("audit-file").arg(repo);
    for arg in args {
        cmd.arg(arg);
    }
    if let Some(bytes) = stdin {
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut child = cmd.spawn().unwrap();
        child.stdin.as_mut().unwrap().write_all(bytes).unwrap();
        child.wait_with_output().unwrap()
    } else {
        cmd.output().unwrap()
    }
}

fn exit_code(output: &std::process::Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

#[test]
fn clean_candidate_passes() {
    let repo = tempdir().unwrap();
    write_target_repo(repo.path());
    let candidate = repo.path().join("candidate.rs");
    fs::write(
        &candidate,
        "pub fn mul(a: i32, b: i32) -> i32 {\n    a * b\n}\n",
    )
    .unwrap();

    let output = audit_file(
        repo.path(),
        &[
            "--path",
            "src/mul.rs",
            "--candidate",
            candidate.to_str().unwrap(),
            "--op",
            "create",
            "--mode",
            "save-gate",
            "--format",
            "json",
        ],
        None,
    );

    assert_eq!(
        exit_code(&output),
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["verdict"], "pass");
}

#[test]
fn todo_marker_blocks_with_exit_3() {
    let repo = tempdir().unwrap();
    write_target_repo(repo.path());
    let candidate = repo.path().join("candidate.rs");
    fs::write(
        &candidate,
        "pub fn handler() {\n    // TODO: implement this path\n    let _ = 1;\n}\n",
    )
    .unwrap();

    let output = audit_file(
        repo.path(),
        &[
            "--path",
            "src/handler.rs",
            "--candidate",
            candidate.to_str().unwrap(),
            "--op",
            "create",
            "--mode",
            "save-gate",
            "--format",
            "json",
        ],
        None,
    );

    assert_eq!(
        exit_code(&output),
        3,
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["verdict"], "block");
    assert!(!value["blocking"]["new_hard_findings"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn preexisting_debt_does_not_block() {
    let repo = tempdir().unwrap();
    write_target_repo(repo.path());
    // The file already carries a TODO before this save.
    fs::write(
        repo.path().join("src/legacy.rs"),
        "pub fn legacy() {\n    // TODO: long-standing debt\n    let _ = 1;\n}\n",
    )
    .unwrap();
    // The candidate adds a clean line and leaves the pre-existing TODO in place.
    let candidate = repo.path().join("candidate.rs");
    fs::write(
        &candidate,
        "pub fn legacy() {\n    // TODO: long-standing debt\n    let _ = 1;\n}\n\npub fn added() -> i32 {\n    2\n}\n",
    )
    .unwrap();

    let output = audit_file(
        repo.path(),
        &[
            "--path",
            "src/legacy.rs",
            "--candidate",
            candidate.to_str().unwrap(),
            "--op",
            "modify",
            "--mode",
            "save-gate",
            "--format",
            "json",
        ],
        None,
    );

    assert_eq!(
        exit_code(&output),
        0,
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["verdict"], "pass");
}

#[test]
fn candidate_via_stdin_matches_file_form() {
    let repo = tempdir().unwrap();
    write_target_repo(repo.path());
    let bytes = b"pub fn handler() {\n    // TODO: from stdin\n}\n";

    let output = audit_file(
        repo.path(),
        &[
            "--path",
            "src/handler.rs",
            "--candidate",
            "-",
            "--op",
            "create",
            "--mode",
            "save-gate",
            "--format",
            "json",
        ],
        Some(bytes),
    );

    assert_eq!(
        exit_code(&output),
        3,
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["verdict"], "block");
}

#[test]
fn advisory_mode_never_blocks() {
    let repo = tempdir().unwrap();
    write_target_repo(repo.path());
    let candidate = repo.path().join("candidate.rs");
    fs::write(
        &candidate,
        "pub fn handler() {\n    // TODO: still advisory\n}\n",
    )
    .unwrap();

    let output = audit_file(
        repo.path(),
        &[
            "--path",
            "src/handler.rs",
            "--candidate",
            candidate.to_str().unwrap(),
            "--op",
            "create",
            "--mode",
            "advisory",
            "--format",
            "json",
        ],
        None,
    );

    let code = exit_code(&output);
    assert!(
        code == 0 || code == 2,
        "advisory must never block, got {code}"
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_ne!(value["verdict"], "block");
}

#[test]
fn docs_exceptions_paths_are_blocked_for_automation() {
    let repo = tempdir().unwrap();
    write_target_repo(repo.path());

    let output = audit_file(
        repo.path(),
        &[
            "--path",
            "docs/exceptions/0001-current.md",
            "--candidate",
            "-",
            "--op",
            "create",
            "--mode",
            "save-gate",
            "--format",
            "json",
        ],
        Some(b"---\ncode: HB_SQL_SHIM\n---\n"),
    );

    assert_eq!(
        exit_code(&output),
        3,
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["verdict"], "block");
    assert!(value["summary"]
        .as_str()
        .unwrap()
        .contains("read-only to automation"));
}

#[test]
fn missing_candidate_file_exits_4() {
    let repo = tempdir().unwrap();
    write_target_repo(repo.path());

    let output = audit_file(
        repo.path(),
        &[
            "--path",
            "src/handler.rs",
            "--candidate",
            "/nonexistent/candidate/file.rs",
            "--op",
            "create",
        ],
        None,
    );

    assert_eq!(exit_code(&output), 4);
    assert!(String::from_utf8_lossy(&output.stderr).contains("jankurai audit-file"));
}

#[test]
fn json_format_carries_schema_and_rerun() {
    let repo = tempdir().unwrap();
    write_target_repo(repo.path());
    let candidate = repo.path().join("candidate.rs");
    fs::write(&candidate, "pub fn ok() -> i32 {\n    1\n}\n").unwrap();

    let output = audit_file(
        repo.path(),
        &[
            "--path",
            "src/ok.rs",
            "--candidate",
            candidate.to_str().unwrap(),
            "--op",
            "create",
            "--format",
            "json",
        ],
        None,
    );

    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "jankurai-save-gate/1");
    assert!(value["rerun_command"]
        .as_str()
        .unwrap()
        .contains("audit-file"));
    assert!(value.get("blocking").is_some());
    assert!(value.get("advisory").is_some());
}

#[test]
fn agent_format_renders_blocked_banner() {
    let repo = tempdir().unwrap();
    write_target_repo(repo.path());
    let candidate = repo.path().join("candidate.rs");
    fs::write(
        &candidate,
        "pub fn handler() {\n    // TODO: render banner\n}\n",
    )
    .unwrap();

    let output = audit_file(
        repo.path(),
        &[
            "--path",
            "src/handler.rs",
            "--candidate",
            candidate.to_str().unwrap(),
            "--op",
            "create",
            "--mode",
            "save-gate",
            "--format",
            "agent",
        ],
        None,
    );

    assert_eq!(exit_code(&output), 3);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("JANKURAI GUARD: BLOCKED"));
    assert!(stdout.contains("Re-run after fixing"));
}
