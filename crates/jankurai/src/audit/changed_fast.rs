use anyhow::Result;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn path_matches_scope(rel_path: &str, scopes: &[String]) -> bool {
    if scopes.is_empty() {
        return true;
    }
    scopes.iter().any(|scope| {
        rel_path == scope
            || rel_path.starts_with(&format!("{scope}/"))
            || scope.starts_with(&format!("{rel_path}/"))
    })
}

pub(super) fn inventory_paths(scope_paths: &[String]) -> Vec<String> {
    let mut paths: BTreeSet<String> = scope_paths.iter().cloned().collect();
    for path in [
        "AGENTS.md",
        "CLAUDE.md",
        "GEMINI.md",
        "CHANGELOG.md",
        "Justfile",
        "README.md",
        "Cargo.toml",
        "Cargo.lock",
        "package.json",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "go.mod",
        "go.sum",
        "agent",
        ".github/workflows",
        "contracts",
        "db",
        "schemas",
        "apps/marketing/e2e",
        "apps/web/e2e",
        "apps/web/src/storybook",
        "apps/web/src/styles.css",
        "docs/architecture.md",
        "docs/artifact-contracts.md",
        "docs/boundaries.md",
        "docs/branch-protection.md",
        "docs/release.md",
        "docs/release-plan.md",
        "docs/security-tool-matrix.md",
        "docs/testing.md",
        "ops/AGENTS.md",
        "ops/ci",
        "ops/git-hooks",
        "tools/security-lane.sh",
    ] {
        paths.insert(path.to_string());
    }
    paths.into_iter().collect()
}

pub(super) fn normalize_path(root: &Path, path: &Path) -> Option<String> {
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let rel = candidate
        .strip_prefix(root)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    Some(rel)
}

pub fn changed_paths_from_git(root: &Path, base: &str) -> Result<Vec<PathBuf>> {
    let refspec = format!("{base}...HEAD");
    let output = Command::new("git")
        .args(["diff", "--no-ext-diff", "--name-only", refspec.as_str()])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Ok(vec![]);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| root.join(line.trim()))
        .collect())
}
