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
const PROTECTED_DIR_PREFIXES: &[&str] = &[".github/", "agent/", "crates/", "tools/"];
const PROTECTED_ROOT_FILES: &[&str] = &[
    "AGENTS.md",
    "CHANGELOG.md",
    "CODEOWNERS",
    "CODE_OF_CONDUCT.md",
    "CONTRIBUTING.md",
    "Cargo.lock",
    "Cargo.toml",
    "Justfile",
    "README.md",
    "SECURITY.md",
    "SUPPORT.md",
    "VERSION",
    "package-lock.json",
    "package.json",
];
const READ_ONLY_EXCEPTION_DIR: &str = "docs/exceptions";

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
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .git_ignore(false)
        .git_exclude(false)
        .git_global(false)
        .max_depth(None);
    for entry in builder
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
            let protected_dir = is_inventory_protected_path(rel);
            let mut builder = WalkBuilder::new(&abs);
            builder.hidden(false);
            if protected_dir {
                builder
                    .git_ignore(false)
                    .git_exclude(false)
                    .git_global(false);
            } else {
                builder.git_ignore(true).git_exclude(true).git_global(true);
            }
            for entry in builder
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
    if is_inventory_protected_path(&rel) {
        return false;
    }
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

fn normalize_repo_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .replace('\\', "/")
        .trim_matches('/')
        .to_string()
}

pub fn is_inventory_protected_path(path: &str) -> bool {
    let rel = normalize_repo_path(path);
    PROTECTED_ROOT_FILES.contains(&rel.as_str())
        || PROTECTED_DIR_PREFIXES.iter().any(|prefix| {
            let trimmed = prefix.trim_end_matches('/');
            rel == trimmed || rel.starts_with(prefix)
        })
}

pub fn is_read_only_exception_path(path: &str) -> bool {
    let rel = normalize_repo_path(path);
    rel == READ_ONLY_EXCEPTION_DIR
        || rel.starts_with(&format!("{READ_ONLY_EXCEPTION_DIR}/"))
        || rel.contains(&format!("/{READ_ONLY_EXCEPTION_DIR}/"))
        || rel.ends_with(&format!("/{READ_ONLY_EXCEPTION_DIR}"))
}

pub fn is_generated_zone_protected_path(path: &str) -> bool {
    let rel = normalize_repo_path(path);
    if rel == ".jankurai/repo-score.json"
        || rel == ".jankurai/repo-score.md"
        || rel == "agent/repo-score.json"
        || rel == "agent/repo-score.md"
        || rel.starts_with("agent/baselines/")
        || rel == "Cargo.lock"
        || rel == "package-lock.json"
        || rel == "pnpm-lock.yaml"
        || rel == "yarn.lock"
    {
        return false;
    }
    is_inventory_protected_path(&rel)
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

/// How a candidate write changes the repo, used to overlay unsaved bytes onto
/// an in-memory inventory so the audit engine can score a file before it lands.
#[derive(Debug, Clone)]
pub enum OverlayOp {
    /// The path does not exist yet and is being created.
    Create,
    /// The path exists and is being overwritten.
    Modify,
    /// The path is being removed.
    Delete,
    /// The path is being renamed from `from` to the overlay's `rel_path`.
    Rename {
        /// Previous repo-relative path (forward-slash normalized).
        from: String,
    },
}

/// A single candidate file change to overlay onto an inventory.
#[derive(Debug, Clone)]
pub struct CandidateOverlay {
    /// Repo-relative path of the candidate (forward-slash normalized).
    pub rel_path: String,
    /// The change being applied.
    pub op: OverlayOp,
    /// Candidate bytes; `None` for [`OverlayOp::Delete`].
    pub bytes: Option<Vec<u8>>,
}

/// Builds a [`FileInfo`] from in-memory candidate bytes without touching disk.
/// Mirrors the classification [`file_seed`] applies to on-disk files so overlaid
/// and walked files are scored identically.
pub fn file_info_from_candidate(
    rel_path: &str,
    bytes: &[u8],
    text_capture_chars: usize,
) -> FileInfo {
    let rel_path = rel_path.replace('\\', "/");
    let name = rel_path
        .rsplit('/')
        .next()
        .unwrap_or(rel_path.as_str())
        .to_string();
    let suffix = suffix_of(&rel_path);
    let is_code = is_code_file(&name, &suffix);
    let is_generated = rel_path.split('/').any(|part| {
        part == "generated" || part.starts_with("generated") || part == "gen" || part == "artifacts"
    });
    let is_text = is_text_candidate(&name, &suffix, &rel_path);
    let (text, line_count) = if is_text {
        capture_candidate_text(bytes, text_capture_chars)
    } else {
        (String::new(), 0)
    };
    FileInfo {
        rel_path,
        name,
        suffix,
        size: bytes.len() as u64,
        line_count,
        text,
        is_generated,
        is_code,
    }
}

/// Captures a text sample and line count from candidate bytes, matching the
/// line-counting semantics of [`read_text_sample`].
fn capture_candidate_text(bytes: &[u8], max_capture_chars: usize) -> (String, usize) {
    let mut line_count = 0usize;
    let mut captured = String::new();
    for line in bytes.split_inclusive(|&b| b == b'\n') {
        line_count += 1;
        if captured.len() < max_capture_chars {
            let remaining = max_capture_chars - captured.len();
            let piece = if line.len() > remaining {
                &line[..remaining]
            } else {
                line
            };
            captured.push_str(&String::from_utf8_lossy(piece));
        }
    }
    (captured, line_count)
}

/// Applies a [`CandidateOverlay`] to an inventory in place: a create/modify
/// upserts the candidate, a delete removes the path, a rename removes the source
/// path and upserts the destination. The list is re-sorted by `rel_path` so
/// downstream consumers see a stable order.
pub fn apply_overlay(
    files: &mut Vec<FileInfo>,
    overlay: &CandidateOverlay,
    text_capture_chars: usize,
) {
    match &overlay.op {
        OverlayOp::Delete => {
            files.retain(|f| f.rel_path != overlay.rel_path);
        }
        OverlayOp::Rename { from } => {
            files.retain(|f| f.rel_path != *from && f.rel_path != overlay.rel_path);
            if let Some(bytes) = &overlay.bytes {
                files.push(file_info_from_candidate(
                    &overlay.rel_path,
                    bytes,
                    text_capture_chars,
                ));
            }
        }
        OverlayOp::Create | OverlayOp::Modify => {
            files.retain(|f| f.rel_path != overlay.rel_path);
            let bytes = overlay.bytes.as_deref().unwrap_or(&[]);
            files.push(file_info_from_candidate(
                &overlay.rel_path,
                bytes,
                text_capture_chars,
            ));
        }
    }
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
}

#[cfg(test)]
mod overlay_tests {
    use super::*;

    fn sample(rel: &str) -> FileInfo {
        file_info_from_candidate(rel, b"existing\n", 4096)
    }

    #[test]
    fn create_inserts_candidate() {
        let mut files = vec![sample("src/a.rs")];
        let overlay = CandidateOverlay {
            rel_path: "src/b.rs".into(),
            op: OverlayOp::Create,
            bytes: Some(b"fn main() {}\n".to_vec()),
        };
        apply_overlay(&mut files, &overlay, 4096);
        assert_eq!(files.len(), 2);
        let added = files.iter().find(|f| f.rel_path == "src/b.rs").unwrap();
        assert!(added.text.contains("fn main"));
        assert!(added.is_code);
    }

    #[test]
    fn modify_replaces_candidate() {
        let mut files = vec![sample("src/a.rs")];
        let overlay = CandidateOverlay {
            rel_path: "src/a.rs".into(),
            op: OverlayOp::Modify,
            bytes: Some(b"changed\n".to_vec()),
        };
        apply_overlay(&mut files, &overlay, 4096);
        assert_eq!(files.len(), 1);
        assert!(files[0].text.contains("changed"));
    }

    #[test]
    fn delete_removes_candidate() {
        let mut files = vec![sample("src/a.rs"), sample("src/b.rs")];
        let overlay = CandidateOverlay {
            rel_path: "src/a.rs".into(),
            op: OverlayOp::Delete,
            bytes: None,
        };
        apply_overlay(&mut files, &overlay, 4096);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, "src/b.rs");
    }

    #[test]
    fn rename_moves_candidate() {
        let mut files = vec![sample("src/source.rs")];
        let overlay = CandidateOverlay {
            rel_path: "src/target.rs".into(),
            op: OverlayOp::Rename {
                from: "src/source.rs".into(),
            },
            bytes: Some(b"renamed\n".to_vec()),
        };
        apply_overlay(&mut files, &overlay, 4096);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, "src/target.rs");
        assert!(files[0].text.contains("renamed"));
    }

    #[test]
    fn candidate_line_count_matches_disk_semantics() {
        let info = file_info_from_candidate("src/a.rs", b"a\nb", 4096);
        assert_eq!(info.line_count, 2);
        let info = file_info_from_candidate("src/a.rs", b"a\nb\n", 4096);
        assert_eq!(info.line_count, 2);
    }
}
