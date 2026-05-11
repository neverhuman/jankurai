use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command as GitProcess;

use crate::shared::resolve_repo_path;

#[derive(Debug, Clone, Default)]
pub(crate) struct CoverageData {
    pub loaded: bool,
    pub files: BTreeMap<String, BTreeSet<u32>>,
}

impl CoverageData {
    pub(crate) fn covered_lines(&self, path: &str) -> BTreeSet<u32> {
        let mut out = BTreeSet::new();
        for (file, lines) in &self.files {
            if file == path || file.ends_with(&format!("/{path}")) || path.ends_with(file) {
                out.extend(lines.iter().copied());
            }
        }
        out
    }
}

pub(crate) fn load_coverage(repo: &Path, path: Option<&Path>) -> Result<CoverageData> {
    let Some(path) = path else {
        return Ok(CoverageData::default());
    };
    let path = resolve_repo_path(repo, path);
    let text =
        fs::read_to_string(&path).with_context(|| format!("read coverage {}", path.display()))?;
    if text.trim_start().starts_with('{') {
        return Ok(load_json_coverage(&text));
    }
    Ok(load_lcov(&text))
}

pub(crate) fn changed_lines_for_paths(
    repo: &Path,
    changed_from: Option<&str>,
    paths: &[String],
) -> BTreeMap<String, BTreeSet<u32>> {
    let mut out = BTreeMap::new();
    for path in paths {
        if !path.ends_with(".rs") {
            continue;
        }
        let lines = match changed_lines_from_git(repo, changed_from, path) {
            Ok(lines) if !lines.is_empty() => lines,
            Ok(_) | Err(_) => BTreeSet::from([1]),
        };
        out.insert(path.clone(), lines);
    }
    out
}

fn load_lcov(text: &str) -> CoverageData {
    let mut data = CoverageData {
        loaded: true,
        files: BTreeMap::new(),
    };
    let mut current: Option<String> = None;
    for line in text.lines() {
        if let Some(file) = line.strip_prefix("SF:") {
            current = Some(file.trim().replace('\\', "/"));
        } else if let Some(rest) = line.strip_prefix("DA:") {
            let Some(file) = current.clone() else {
                continue;
            };
            let mut parts = rest.split(',');
            let line_no = parts.next().and_then(|value| value.parse::<u32>().ok());
            let hits = parts.next().and_then(|value| value.parse::<u64>().ok());
            if let (Some(line_no), Some(hits)) = (line_no, hits) {
                if hits > 0 {
                    data.files.entry(file).or_default().insert(line_no);
                }
            }
        }
    }
    data
}

fn load_json_coverage(text: &str) -> CoverageData {
    let mut data = CoverageData {
        loaded: true,
        files: BTreeMap::new(),
    };
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        data.loaded = false;
        return data;
    };
    if let Some(files) = value.get("files").and_then(Value::as_array) {
        for file in files {
            let filename_value = match file.get("filename") {
                Some(value) => Some(value),
                None => file.get("path"),
            };
            let Some(filename) = filename_value.and_then(Value::as_str) else {
                continue;
            };
            if let Some(lines) = file.get("covered_lines").and_then(Value::as_array) {
                for line in lines.iter().filter_map(Value::as_u64) {
                    data.files
                        .entry(filename.replace('\\', "/"))
                        .or_default()
                        .insert(line as u32);
                }
            }
        }
    }
    data
}

fn changed_lines_from_git(
    repo: &Path,
    changed_from: Option<&str>,
    path: &str,
) -> Result<BTreeSet<u32>> {
    let mut command = GitProcess::new("git");
    command.current_dir(repo).arg("diff").arg("--unified=0");
    if let Some(base) = changed_from {
        command.arg(format!("{base}...HEAD"));
    }
    command.arg("--").arg(path);
    let output = command.output()?;
    if !output.status.success() {
        return Ok(BTreeSet::new());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_unified_diff_changed_lines(&text))
}

fn parse_unified_diff_changed_lines(diff: &str) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    let mut current_new_line = 0u32;
    for line in diff.lines() {
        if line.starts_with("@@") {
            if let Some(start) = parse_hunk_new_start(line) {
                current_new_line = start;
            }
            continue;
        }
        if current_new_line == 0 {
            continue;
        }
        if line.starts_with('+') && !line.starts_with("+++") {
            out.insert(current_new_line);
            current_new_line = current_new_line.saturating_add(1);
        } else if line.starts_with('-') && !line.starts_with("---") {
            continue;
        } else {
            current_new_line = current_new_line.saturating_add(1);
        }
    }
    out
}

fn parse_hunk_new_start(line: &str) -> Option<u32> {
    let plus = line.find('+')?;
    let rest = &line[plus + 1..];
    let number = rest.split(|c: char| c == ',' || c.is_whitespace()).next()?;
    number.parse().ok()
}
