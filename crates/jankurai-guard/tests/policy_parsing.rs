//! Tests for `jankurai_guard::policy`: the default policy matches the
//! documented schema, a partial policy file merges over the defaults, invalid
//! values are rejected, and a missing file degrades to the defaults.

use jankurai_guard::policy::{AuditScope, GuardPolicy, OnFail};
use jankurai_guard::GuardMode;
use std::fs;
use tempfile::tempdir;

/// Writes `body` to `<repo>/agent/guard-policy.toml`.
fn write_policy(repo: &std::path::Path, body: &str) {
    let dir = repo.join("agent");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("guard-policy.toml"), body).unwrap();
}

#[test]
fn default_matches_documented_schema() {
    let p = GuardPolicy::default();
    assert_eq!(p.schema_version, "1.0.0");
    assert_eq!(p.mode, GuardMode::Enforce);
    assert_eq!(p.block_on, vec!["critical", "high"]);
    assert_eq!(p.warn_on, vec!["medium", "low"]);
    assert_eq!(p.debounce_ms, 150);
    assert_eq!(p.stable_ms, 75);
    assert_eq!(p.on_fail, OnFail::Poison);
    assert!(p.quarantine_new_files);
    assert!(p.respect_gitignore);
    assert_eq!(p.audit_scope, AuditScope::FilePlusControl);
    assert!(p.fail_closed);
    assert_eq!(p.max_audit_ms, 4000);
    assert!(p
        .paths
        .extra_excluded_paths
        .contains(&".jankurai/".to_string()));
    assert!(p
        .paths
        .extra_excluded_paths
        .contains(&"target/jankurai/".to_string()));
    assert!(!p.hardening.linux_landlock);
    assert!(!p.hardening.linux_fanotify);
    assert!(!p.hardening.macos_endpoint_security);
}

#[test]
fn missing_file_falls_back_to_defaults() {
    let dir = tempdir().unwrap();
    let loaded = GuardPolicy::load(dir.path()).unwrap();
    assert_eq!(loaded, GuardPolicy::default());
}

#[test]
fn partial_file_merges_over_defaults() {
    let dir = tempdir().unwrap();
    write_policy(
        dir.path(),
        r#"
            mode = "observe"
            debounce_ms = 500
            on_fail = "revert"
        "#,
    );
    let loaded = GuardPolicy::load(dir.path()).unwrap();
    // Overridden fields take the file's value.
    assert_eq!(loaded.mode, GuardMode::Observe);
    assert_eq!(loaded.debounce_ms, 500);
    assert_eq!(loaded.on_fail, OnFail::Revert);
    // Unspecified fields keep the documented defaults.
    assert_eq!(loaded.stable_ms, 75);
    assert!(loaded.fail_closed);
    assert_eq!(loaded.block_on, vec!["critical", "high"]);
}

#[test]
fn invalid_mode_is_rejected() {
    let dir = tempdir().unwrap();
    write_policy(dir.path(), "mode = \"paranoid\"\n");
    let err = GuardPolicy::load(dir.path()).unwrap_err();
    assert!(err.to_string().contains("invalid guard policy"));
}

#[test]
fn invalid_severity_is_rejected() {
    let dir = tempdir().unwrap();
    write_policy(dir.path(), "block_on = [\"catastrophic\"]\n");
    let err = GuardPolicy::load(dir.path()).unwrap_err();
    assert!(err.to_string().contains("unknown severity"));
}

#[test]
fn zero_max_audit_ms_is_rejected() {
    let dir = tempdir().unwrap();
    write_policy(dir.path(), "max_audit_ms = 0\n");
    let err = GuardPolicy::load(dir.path()).unwrap_err();
    assert!(err.to_string().contains("max_audit_ms"));
}

#[test]
fn negative_timing_is_rejected_by_type() {
    // A negative number cannot deserialize into the `u64` timing fields, so the
    // load fails with an invalid-policy error rather than silently clamping.
    let dir = tempdir().unwrap();
    write_policy(dir.path(), "debounce_ms = -10\n");
    let err = GuardPolicy::load(dir.path()).unwrap_err();
    assert!(err.to_string().contains("invalid guard policy"));
}

#[test]
fn exclusion_matching_covers_hard_and_policy_prefixes() {
    let p = GuardPolicy::default();
    assert!(p.is_excluded(".jankurai/guard/x.md"));
    assert!(p.is_excluded("target/jankurai/cache"));
    assert!(p.is_excluded("node_modules/pkg/index.js"));
    assert!(p.is_excluded(".git/HEAD"));
    assert!(!p.is_excluded("src/lib.rs"));
}
