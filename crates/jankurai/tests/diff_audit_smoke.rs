//! Smoke tests for `jankurai diff-audit`.
//!
//! Strategy: spin up a minimal git repo with a base commit, make a modification
//! on a feature branch, invoke the subcommand, and assert that the artifacts
//! land where we expect with a non-failing report on a benign change.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_AUTHOR_NAME", "smoke")
        .env("GIT_AUTHOR_EMAIL", "smoke@example.com")
        .env("GIT_COMMITTER_NAME", "smoke")
        .env("GIT_COMMITTER_EMAIL", "smoke@example.com")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("JERYU_GIT_BYPASS", "1")
        .status()
        .expect("git invocation");
    assert!(
        status.success(),
        "git {args:?} failed in {}",
        repo.display()
    );
}

fn init_repo() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    // A trivial benign file as the base commit.
    fs::write(dir.path().join("README.md"), "# smoke\n").unwrap();
    git(dir.path(), &["add", "README.md"]);
    git(dir.path(), &["commit", "-q", "-m", "base"]);
    dir
}

#[test]
fn help_works() {
    let status = Command::new(binary_path())
        .arg("diff-audit")
        .arg("--help")
        .status()
        .unwrap();
    assert!(status.success(), "diff-audit --help must exit 0");
}

#[test]
fn no_changes_writes_empty_score_and_passes() {
    let repo = init_repo();
    let out_dir = repo.path().join("target/jankurai/diff");
    // Use HEAD as the base ref so HEAD-vs-HEAD is provably empty regardless of
    // user / system git config (templates, default-branch name, etc.).
    let output = Command::new(binary_path())
        .arg("diff-audit")
        .arg(repo.path())
        .arg("--base-ref")
        .arg("HEAD")
        .arg("--skip-proof") // no agent/owner-map.json in this minimal repo
        .arg("--out-dir")
        .arg(&out_dir)
        // Belt-and-braces: ensure no stray env nudges from cargo test's parent.
        .env_remove("GITHUB_BASE_REF")
        .env_remove("CI_MERGE_REQUEST_DIFF_BASE_SHA")
        .env_remove("JANKURAI_DIFF_BASE")
        .env("JERYU_GIT_BYPASS", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "no-changes case must exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let changed_lst = out_dir.join("changed.lst");
    assert!(changed_lst.exists(), "changed.lst must be written");
    let body = fs::read_to_string(&changed_lst).unwrap();
    assert_eq!(body.trim(), "", "no-changes case → empty list");

    let json = out_dir.join("diff-score.json");
    assert!(json.exists(), "diff-score.json must be written");
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(json).unwrap()).unwrap();
    assert_eq!(v["kind"], "diff-audit");
    assert_eq!(v["result"], "no-changes");
    assert_eq!(v["changed_count"], 0);
}

#[test]
fn benign_change_succeeds_advisory() {
    let repo = init_repo();
    // Add a new untracked file in the worktree — diff-audit must see it via
    // `git diff --name-only` over the worktree leg.
    fs::write(
        repo.path().join("CHANGELOG.md"),
        "# changelog\n\n- nothing yet\n",
    )
    .unwrap();
    git(repo.path(), &["add", "CHANGELOG.md"]);

    let out_dir = repo.path().join("target/jankurai/diff");
    let output = Command::new(binary_path())
        .arg("diff-audit")
        .arg(repo.path())
        .arg("--base-ref")
        .arg("main")
        .arg("--skip-proof")
        .arg("--advisory-only") // benign Markdown shouldn't trigger findings, but be defensive
        .arg("--out-dir")
        .arg(&out_dir)
        .env("JERYU_GIT_BYPASS", "1")
        .env_remove("JANKURAI_DIFF_BASE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "benign change in advisory mode must exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let changed_lst = out_dir.join("changed.lst");
    assert!(changed_lst.exists());
    let body = fs::read_to_string(&changed_lst).unwrap();
    assert!(
        body.contains("CHANGELOG.md"),
        "expected CHANGELOG.md in changed.lst, got: {body:?}"
    );
}

#[test]
fn skip_hooks_env_short_circuits() {
    let repo = init_repo();
    let out_dir = repo.path().join("target/jankurai/diff");
    let status = Command::new(binary_path())
        .arg("diff-audit")
        .arg(repo.path())
        .arg("--out-dir")
        .arg(&out_dir)
        .env("JANKURAI_SKIP_HOOKS", "1")
        .env("JERYU_GIT_BYPASS", "1")
        .status()
        .unwrap();
    assert!(status.success(), "JANKURAI_SKIP_HOOKS=1 must exit 0");
    // No artifacts should have been written.
    assert!(!out_dir.join("changed.lst").exists());
}
