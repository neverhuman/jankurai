//! Optional external cross-check. Never affects score; advisory evidence only.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrossCheckResult {
    pub tool: String,
    pub available: bool,
    pub version: Option<String>,
    pub duplicate_count: Option<usize>,
    pub raw_path: Option<String>,
    pub note: Option<String>,
}

pub fn run_jscpd(repo: &Path, out_dir: &Path) -> Result<CrossCheckResult> {
    let Some(bin) = which("jscpd") else {
        return Ok(CrossCheckResult {
            tool: "jscpd".into(),
            available: false,
            note: Some("jscpd not on PATH; install with `npm i -g jscpd`".into()),
            ..Default::default()
        });
    };
    std::fs::create_dir_all(out_dir).ok();
    let raw_path = out_dir.join("jscpd-report.json");
    let status = Command::new(&bin)
        .arg(repo)
        .arg("--silent")
        .arg("--min-lines")
        .arg("10")
        .arg("--min-tokens")
        .arg("100")
        .arg("--mode")
        .arg("strict")
        .arg("--reporters")
        .arg("json")
        .arg("--output")
        .arg(out_dir)
        .status()
        .context("failed to spawn jscpd")?;
    if !status.success() {
        return Ok(CrossCheckResult {
            tool: "jscpd".into(),
            available: true,
            note: Some(format!("jscpd exited non-zero: {status}")),
            ..Default::default()
        });
    }
    let dup_count = parse_jscpd_duplicates(&raw_path).ok();
    Ok(CrossCheckResult {
        tool: "jscpd".into(),
        available: true,
        version: jscpd_version(&bin),
        duplicate_count: dup_count,
        raw_path: Some(raw_path.display().to_string()),
        note: None,
    })
}

fn which(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return candidate.to_str().map(String::from);
        }
    }
    None
}

fn jscpd_version(bin: &str) -> Option<String> {
    let out = Command::new(bin).arg("--version").output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn parse_jscpd_duplicates(p: &Path) -> Result<usize> {
    #[allow(non_snake_case)]
    #[derive(Deserialize)]
    struct Total {
        clones: Option<u64>,
    }
    #[derive(Deserialize)]
    struct Stats {
        total: Total,
    }
    #[derive(Deserialize)]
    struct Report {
        statistics: Stats,
    }
    let text = std::fs::read_to_string(p)?;
    let r: Report = serde_json::from_str(&text)?;
    Ok(r.statistics.total.clones.unwrap_or(0) as usize)
}
