use crate::commands::update;
use crate::model::{
    AUDITOR_VERSION, PAPER_EDITION, SCHEMA_VERSION, STANDARD_VERSION, TARGET_STACK_ID,
};
use anyhow::{anyhow, Result};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};

pub fn print_version(repo: &Path) -> Result<()> {
    let root = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    let exe = std::env::current_exe().ok();
    println!("CLI version: `{}`", env!("CARGO_PKG_VERSION"));
    println!("Standard version: `{STANDARD_VERSION}`");
    println!("Schema version: `{SCHEMA_VERSION}`");
    println!(
        "Executable: `{}`",
        exe.as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<unknown>".into())
    );
    println!(
        "Install root: `{}`",
        update::cargo_install_root().unwrap_or_else(|| "<unknown>".into())
    );
    println!("Recommended upgrade command: `jankurai upgrade`");
    if is_source_checkout(&root) {
        println!("Source checkout: `{}`", root.display());
    }
    Ok(())
}

pub fn check_versions(repo: &Path) -> Result<()> {
    let root = repo.canonicalize()?;
    if !is_source_checkout(&root) {
        return Err(anyhow!("use `jankurai version` for installed CLI version."));
    }
    let manifest_path = root.join("agent/standard-version.toml");
    let manifest_text = fs::read_to_string(&manifest_path)?;
    let manifest: toml::Value = toml::from_str(&manifest_text)?;

    let standard_version = scalar(&manifest, "standard_version")?;
    let auditor_version = scalar(&manifest, "auditor_version")?;
    let schema_version = scalar(&manifest, "schema_version")?;
    let paper_edition = scalar(&manifest, "paper_edition")?;
    let target_stack = scalar(&manifest, "target_stack")?;

    assert_contains(root.join("VERSION"), auditor_version.as_str(), "VERSION")?;
    assert_contains(
        root.join("docs/agent-native-standard.md"),
        &format!("Standard version: `{}`", STANDARD_VERSION),
        "docs/agent-native-standard.md",
    )?;
    assert_contains(
        root.join("agent/JANKURAI_STANDARD.md"),
        &format!("Standard version: `{}`", STANDARD_VERSION),
        "agent/JANKURAI_STANDARD.md",
    )?;
    assert_contains(
        root.join("paper/jankurai.md"),
        &format!("Paper edition: `{}`", PAPER_EDITION),
        "paper/jankurai.md",
    )?;
    assert_contains(
        root.join("paper/jankurai.md"),
        &format!("Standard version: `{}`", STANDARD_VERSION),
        "paper/jankurai.md",
    )?;

    let pkg = root.join("crates/jankurai/Cargo.toml");
    let pkg_text = fs::read_to_string(&pkg)?;
    let pkg_val: toml::Value = toml::from_str(&pkg_text)?;
    assert_str(
        &pkg_val,
        &["package", "version"],
        AUDITOR_VERSION,
        "crates/jankurai/Cargo.toml package.version",
    )?;

    let ux_pkg = root.join("packages/ux-qa/package.json");
    let ux_text = fs::read_to_string(&ux_pkg)?;
    let ux_val: JsonValue = serde_json::from_str(&ux_text)?;
    let ux_version = ux_val
        .get("version")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow!("missing packages/ux-qa/package.json version"))?;
    if ux_version != AUDITOR_VERSION {
        return Err(anyhow!(
            "packages/ux-qa/package.json version: expected {AUDITOR_VERSION}, got {ux_version}"
        ));
    }

    if standard_version != STANDARD_VERSION
        || auditor_version != AUDITOR_VERSION
        || schema_version != SCHEMA_VERSION
        || paper_edition != PAPER_EDITION
        || target_stack != TARGET_STACK_ID
    {
        return Err(anyhow!("manifest bindings mismatch"));
    }
    println!(
        "versions ok: standard={} auditor={} schema={} paper={}",
        STANDARD_VERSION, AUDITOR_VERSION, SCHEMA_VERSION, PAPER_EDITION
    );
    Ok(())
}

fn is_source_checkout(root: &Path) -> bool {
    root.join("agent/standard-version.toml").exists()
        && root.join("crates/jankurai/Cargo.toml").exists()
}

fn scalar(value: &toml::Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("missing key {key}"))
}

fn assert_str(value: &toml::Value, path: &[&str], expected: &str, label: &str) -> Result<()> {
    let mut cur = value;
    for key in path {
        cur = cur.get(*key).ok_or_else(|| anyhow!("missing {label}"))?;
    }
    let actual = cur.as_str().ok_or_else(|| anyhow!("non-string {label}"))?;
    if actual != expected {
        return Err(anyhow!("{label}: expected {expected}, got {actual}"));
    }
    Ok(())
}

fn assert_contains(path: PathBuf, expected: &str, label: &str) -> Result<()> {
    let actual = fs::read_to_string(&path)?;
    if !actual.contains(expected) {
        return Err(anyhow!(
            "{label}: expected to contain {expected}, got {actual}"
        ));
    }
    Ok(())
}
