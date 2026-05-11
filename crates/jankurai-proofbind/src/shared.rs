use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as GitProcess;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn resolve_repo_path(repo: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.join(path)
    }
}

pub fn resolve_changed_paths<F>(
    repo: &Path,
    changed: &[PathBuf],
    changed_from: Option<&str>,
    include_path: F,
) -> Result<Vec<String>>
where
    F: Fn(&str) -> bool,
{
    let paths = if let Some(base) = changed_from {
        changed_paths_from_git(repo, base)?
    } else if changed.is_empty() {
        local_changed_paths_from_git(repo)?
    } else {
        changed
            .iter()
            .filter_map(|path| normalize_changed_path(repo, path))
            .collect()
    };
    let mut paths = paths
        .into_iter()
        .filter(|path| !path.trim().is_empty())
        .filter(|path| include_path(path))
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

pub fn changed_paths_from_git(repo: &Path, base: &str) -> Result<Vec<String>> {
    let refspec = format!("{base}...HEAD");
    let output = GitProcess::new("git")
        .args(["diff", "--name-only", refspec.as_str()])
        .current_dir(repo)
        .output()
        .with_context(|| format!("run git diff for {base}"))?;
    if !output.status.success() {
        return Ok(vec![]);
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

pub fn local_changed_paths_from_git(repo: &Path) -> Result<Vec<String>> {
    let output = GitProcess::new("git")
        .args(["diff", "--name-only"])
        .current_dir(repo)
        .output();
    let Ok(output) = output else {
        return Ok(vec![]);
    };
    if !output.status.success() {
        return Ok(vec![]);
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

pub fn normalize_changed_path(repo: &Path, path: &Path) -> Option<String> {
    let candidate = resolve_repo_path(repo, path);
    candidate
        .strip_prefix(repo)
        .ok()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
}

pub fn read_json<T: DeserializeOwned>(path: PathBuf) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn read_toml<T: DeserializeOwned>(path: PathBuf) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    toml::from_str(&text).ok()
}

pub fn prefix_matches(prefix: &str, path: &str) -> bool {
    let prefix = prefix.trim().trim_matches('/');
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

pub fn normalize_prefix(prefix: &str) -> String {
    prefix.trim().trim_end_matches('/').to_string()
}

pub fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    GitProcess::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

pub fn git_dirty(repo: &Path) -> bool {
    if let Ok(out) = GitProcess::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
    {
        !out.stdout.is_empty()
    } else {
        false
    }
}

pub fn unix_seconds() -> String {
    if let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) {
        duration.as_secs().to_string()
    } else {
        "0".into()
    }
}

pub fn elapsed_ms(started: SystemTime) -> u128 {
    if let Ok(duration) = started.elapsed() {
        duration.as_millis()
    } else {
        0
    }
}

pub fn path_symbol(path: &str) -> String {
    if let Some(name) = Path::new(path).file_stem().and_then(|name| name.to_str()) {
        name.to_string()
    } else {
        "file".into()
    }
}

pub fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                ':'
            }
        })
        .collect::<String>()
        .trim_matches(':')
        .to_string()
}

pub fn display_rel(repo: &Path, path: &Path) -> String {
    if let Ok(rel) = path.strip_prefix(repo) {
        rel.to_string_lossy().replace('\\', "/")
    } else {
        path.to_string_lossy().replace('\\', "/")
    }
}
