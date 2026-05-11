use crate::model::FileInfo;
use anyhow::Result;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::file_kinds::{is_code_file, is_text_candidate, suffix_of};
pub use super::fs_policy::{InventoryOptions, InventoryResult, InventoryTimings};

const EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".idea",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    ".venv",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
    "venv",
    ".witness",
];

const EXCLUDED_AGENT_STATE_DIRS: &[&str] = &[".antigravity", "antigravity"];
const CURSOR_ALLOWED_PREFIXES: &[&str] = &[".cursor/rules/"];

pub fn inventory_repo(root: &Path) -> Result<Vec<FileInfo>> {
    Ok(inventory_repo_detailed(root, &InventoryOptions::from_policy(root))?.files)
}

pub fn inventory_repo_for_paths(root: &Path, paths: &[String]) -> Result<Vec<FileInfo>> {
    let options = InventoryOptions::from_policy(root);
    Ok(inventory_paths_detailed(root, paths, &options)?.files)
}

pub fn inventory_repo_detailed(root: &Path, options: &InventoryOptions) -> Result<InventoryResult> {
    let walk_started = Instant::now();
    let mut paths: Vec<PathBuf> = Vec::new();
    let filter_root = root.to_path_buf();
    let filter_options = options.clone();
    for entry in WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(true)
        .max_depth(None)
        .filter_entry(move |entry| {
            let Ok(rel) = entry.path().strip_prefix(&filter_root) else {
                return true;
            };
            rel.as_os_str().is_empty() || !should_skip(rel, &filter_options)
        })
        .build()
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let rel = match path.strip_prefix(root) {
            Ok(rel) => rel,
            Err(_) => continue,
        };
        if should_skip(rel, options) {
            continue;
        }
        paths.push(rel.to_path_buf());
    }
    paths.sort();
    paths.dedup();
    let walk = walk_started.elapsed();

    inventory_from_paths(root, paths, options, walk)
}

pub fn inventory_paths_detailed(
    root: &Path,
    paths: &[String],
    options: &InventoryOptions,
) -> Result<InventoryResult> {
    let walk_started = Instant::now();
    let mut collected = Vec::new();
    for rel in paths {
        let rel = rel.trim().trim_start_matches("./");
        if rel.is_empty() {
            continue;
        }
        let rel_path = PathBuf::from(rel);
        if should_skip(&rel_path, options) {
            continue;
        }
        let abs = root.join(&rel_path);
        if abs.is_file() {
            collected.push(rel_path);
        } else if abs.is_dir() {
            let filter_root = root.to_path_buf();
            let filter_options = options.clone();
            for entry in WalkBuilder::new(&abs)
                .hidden(false)
                .git_ignore(true)
                .git_exclude(true)
                .git_global(true)
                .filter_entry(move |entry| {
                    let Ok(rel) = entry.path().strip_prefix(&filter_root) else {
                        return true;
                    };
                    rel.as_os_str().is_empty() || !should_skip(rel, &filter_options)
                })
                .build()
            {
                let Ok(entry) = entry else {
                    continue;
                };
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Ok(rel) = path.strip_prefix(root) else {
                    continue;
                };
                if should_skip(rel, options) {
                    continue;
                }
                collected.push(rel.to_path_buf());
            }
        }
    }
    collected.sort();
    collected.dedup();
    let walk = walk_started.elapsed();
    inventory_from_paths(root, collected, options, walk)
}

fn inventory_from_paths(
    root: &Path,
    paths: Vec<PathBuf>,
    options: &InventoryOptions,
    walk: Duration,
) -> Result<InventoryResult> {
    let metadata_started = Instant::now();
    let mut seeds: Vec<FileSeed> = paths
        .par_iter()
        .filter_map(|rel| {
            if should_skip(rel, options) {
                return None;
            }
            file_seed(root, rel)
        })
        .collect();
    seeds.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    let metadata = metadata_started.elapsed();

    let text_started = Instant::now();
    let mut files: Vec<FileInfo> = seeds
        .into_par_iter()
        .map(|seed| {
            let (text, line_count) = if seed.is_text {
                read_text_sample(&root.join(&seed.rel), options.text_capture_chars)
                    .unwrap_or_default()
            } else {
                (String::new(), 0)
            };
            FileInfo {
                rel_path: seed.rel_path,
                name: seed.name,
                suffix: seed.suffix,
                size: seed.size,
                line_count,
                text,
                is_generated: seed.is_generated,
                is_code: seed.is_code,
            }
        })
        .collect();
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    let text_capture = text_started.elapsed();

    Ok(InventoryResult {
        files,
        timings: InventoryTimings {
            walk_ms: walk.as_millis(),
            metadata_ms: metadata.as_millis(),
            text_capture_ms: text_capture.as_millis(),
        },
    })
}

fn file_seed(root: &Path, rel: &Path) -> Option<FileSeed> {
    let abs = root.join(rel);
    let meta = abs.metadata().ok()?;
    let rel_path = rel.to_string_lossy().replace('\\', "/");
    let name = abs
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let suffix = suffix_of(&rel_path);
    let is_code = is_code_file(&name, &suffix);
    let is_generated = rel_path.split('/').any(|part| {
        part == "generated" || part.starts_with("generated") || part == "gen" || part == "artifacts"
    });
    let is_text = is_text_candidate(&name, &suffix, &rel_path);
    Some(FileSeed {
        rel: rel.to_path_buf(),
        rel_path,
        name,
        suffix,
        size: meta.len(),
        is_text,
        is_generated,
        is_code,
    })
}

fn should_skip(path: &Path, options: &InventoryOptions) -> bool {
    let rel = path.to_string_lossy().replace('\\', "/");
    if rel.starts_with(".cursor/")
        && rel != ".cursor/rules"
        && !CURSOR_ALLOWED_PREFIXES.iter().any(|p| rel.starts_with(p))
    {
        return true;
    }
    if options.excluded_paths.iter().any(|excluded| {
        rel == *excluded || rel.starts_with(&format!("{}/", excluded.trim_end_matches('/')))
    }) {
        return true;
    }
    if options
        .extra_excluded_globs
        .as_ref()
        .is_some_and(|set| set.is_match(&rel))
    {
        return true;
    }
    if EXCLUDED_AGENT_STATE_DIRS
        .iter()
        .any(|dir| rel == *dir || rel.starts_with(&format!("{dir}/")))
    {
        return true;
    }
    path.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        EXCLUDED_DIRS.contains(&s.as_ref())
    })
}

fn read_text_sample(path: &Path, max_capture_chars: usize) -> Result<(String, usize)> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut line_count = 0usize;
    let mut captured = String::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = reader.read_until(b'\n', &mut line)?;
        if read == 0 {
            break;
        }
        line_count += 1;
        if captured.len() < max_capture_chars {
            let remaining = max_capture_chars - captured.len();
            let mut piece = line.as_slice();
            if piece.len() > remaining {
                piece = &piece[..remaining];
            }
            captured.push_str(&String::from_utf8_lossy(piece));
        }
    }
    Ok((captured, line_count))
}

struct FileSeed {
    rel: PathBuf,
    rel_path: String,
    name: String,
    suffix: String,
    size: u64,
    is_text: bool,
    is_generated: bool,
    is_code: bool,
}
