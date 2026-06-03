//! HLT-044-WORKTREE-SPRAWL detector (jankurai workstream #6, Feature B guard 1).
//!
//! Scans the sibling directories next to `ctx.root` for git worktrees or clones
//! that share the SAME origin as `ctx.root`. Same-origin sibling copies (the
//! `~/jeryu-*` parallel-checkout pattern) are a sprawl smell: parallel mutable
//! checkouts of one repo invite divergent audit state and accidental cross-edits.
//!
//! This guard is ADVISORY (registered, but not in `fail_on`) and only flags
//! same-origin copies. Unrelated sibling repos are ignored, so a repository with
//! no parallel checkout of itself (such as jankurai) yields zero findings.

use crate::audit::helpers::AuditContext;
use crate::audit::scan::FindingHit;
use std::path::Path;
use std::process::Command;

/// Detects sibling directories that are same-origin worktrees or clones of the
/// repository at `ctx.root`. Returns one [`FindingHit`] per same-origin sibling.
///
/// Detection compares the canonical `remote.origin.url` of each readable sibling
/// directory against `ctx.root`'s origin. Siblings with no origin, a different
/// origin, or no git metadata are skipped. The scan never recurses and never
/// touches `ctx.root` itself.
pub fn detect_sprawl(ctx: &AuditContext) -> Vec<FindingHit> {
    let root = ctx.root.as_path();
    let Some(parent) = root.parent() else {
        return vec![];
    };
    // A repo with no origin URL cannot have a "same-origin" sibling, so there is
    // nothing to compare against and we exit early (zero findings).
    let Some(root_origin) = origin_url(root) else {
        return vec![];
    };
    let root_canonical = std::fs::canonicalize(root).ok();

    let Ok(entries) = std::fs::read_dir(parent) else {
        return vec![];
    };

    let mut hits = vec![];
    for entry in entries.flatten() {
        let sibling = entry.path();
        if !sibling.is_dir() {
            continue;
        }
        // Never flag the audited repo against itself.
        if let (Some(root_real), Ok(sib_real)) =
            (root_canonical.as_ref(), std::fs::canonicalize(&sibling))
        {
            if root_real == &sib_real {
                continue;
            }
        }
        let Some(sibling_origin) = origin_url(&sibling) else {
            continue;
        };
        if !origins_match(&root_origin, &sibling_origin) {
            continue;
        }
        // Only flag genuine separate checkouts/worktrees, not subdirectories of
        // the same repo: workspace member crates (e.g. `crates/jankurai-guard`)
        // inherit the repo's `origin` url, so without this guard every sibling
        // crate would be a false-positive "clone".
        if !is_own_checkout_root(&sibling) {
            continue;
        }
        let name = sibling
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| sibling.display().to_string());
        hits.push(FindingHit {
            path: ".".into(),
            line: None,
            text: format!(
                "sibling `{name}` is a same-origin worktree/clone of this repository (origin `{root_origin}`)"
            ),
            matched_term: Some("worktree-sprawl".into()),
            agent_fix: format!(
                "consolidate parallel checkouts: keep one canonical clone of `{root_origin}` and remove or archive the sibling `{name}`, or convert it to a named `git worktree` under one repo so audit state stays single-source"
            ),
            problem: format!(
                "same-origin sibling checkout `{name}` invites divergent audit state and accidental cross-edits"
            ),
        });
    }
    hits
}

/// Reads `git -C <dir> config --get remote.origin.url`, returning the trimmed
/// URL when the directory is a git repository with an `origin` remote.
fn origin_url(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(dir)
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8(output.stdout).ok()?;
    let trimmed = url.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// True when `dir` is itself the root of a git checkout or worktree — i.e.
/// `git -C <dir> rev-parse --show-toplevel` resolves back to `dir`. A workspace
/// member crate (a subdirectory of a larger repo) resolves to the repo root
/// instead, so it is correctly NOT treated as a separate same-origin checkout.
fn is_own_checkout_root(dir: &Path) -> bool {
    let Ok(output) = Command::new("git")
        .args(["-C"])
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let Ok(top) = String::from_utf8(output.stdout) else {
        return false;
    };
    match (std::fs::canonicalize(dir), std::fs::canonicalize(top.trim())) {
        (Ok(d), Ok(t)) => d == t,
        _ => false,
    }
}

/// Compares two origin URLs for same-repo identity after normalization (strip a
/// trailing `.git`, trailing slash, and case-fold the host-insensitive form).
/// SSH (`git@host:org/repo`) and HTTPS (`https://host/org/repo`) spellings of the
/// same remote are treated as equal so a clone and a worktree of one repo match.
fn origins_match(a: &str, b: &str) -> bool {
    normalize_origin(a) == normalize_origin(b)
}

fn normalize_origin(url: &str) -> String {
    let mut s = url.trim().to_ascii_lowercase();
    // Normalize scp-like SSH form `git@host:org/repo` to `host/org/repo`.
    if let Some(rest) = s.strip_prefix("git@") {
        s = rest.replacen(':', "/", 1);
    } else {
        for scheme in ["https://", "http://", "ssh://", "git://"] {
            if let Some(rest) = s.strip_prefix(scheme) {
                s = rest.to_string();
                break;
            }
        }
        // Drop a `user@` prefix that can survive after an `ssh://` strip.
        if let Some(idx) = s.find('@') {
            if !s[..idx].contains('/') {
                s = s[idx + 1..].to_string();
            }
        }
    }
    s = s.trim_end_matches('/').to_string();
    s = s.strip_suffix(".git").unwrap_or(&s).to_string();
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FileInfo;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git available");
        assert!(
            status.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&status.stderr)
        );
    }

    fn init_repo_with_origin(dir: &Path, origin: &str) {
        std::fs::create_dir_all(dir).unwrap();
        git(dir, &["init", "-q"]);
        git(dir, &["remote", "add", "origin", origin]);
    }

    fn ctx_for(root: PathBuf) -> AuditContext {
        AuditContext {
            root,
            all_files: vec![FileInfo {
                rel_path: "README.md".into(),
                name: "README.md".into(),
                suffix: ".md".into(),
                size: 0,
                line_count: 1,
                text: String::new(),
                is_generated: false,
                is_code: false,
            }],
            scope_files: vec![],
            scope_paths: vec![],
            self_audit: false,
            boundary_reclassifications: vec![],
            copy_code: None,
        }
    }

    #[test]
    fn detects_same_origin_sibling_clone() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("jeryu-a");
        let sibling = parent.path().join("jeryu-b");
        init_repo_with_origin(&root, "https://example.com/org/repo.git");
        init_repo_with_origin(&sibling, "git@example.com:org/repo.git");

        let hits = detect_sprawl(&ctx_for(root));
        assert_eq!(hits.len(), 1, "one same-origin sibling expected");
        assert_eq!(hits[0].matched_term.as_deref(), Some("worktree-sprawl"));
        assert!(hits[0].text.contains("jeryu-b"));
    }

    #[test]
    fn ignores_unrelated_sibling_repo() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("repo-a");
        let sibling = parent.path().join("repo-b");
        init_repo_with_origin(&root, "https://example.com/org/repo.git");
        init_repo_with_origin(&sibling, "https://example.com/org/other.git");

        let hits = detect_sprawl(&ctx_for(root));
        assert!(
            hits.is_empty(),
            "unrelated sibling repo must not be flagged: {hits:?}"
        );
    }

    #[test]
    fn ignores_sibling_without_origin() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("repo-a");
        let sibling = parent.path().join("plain-dir");
        init_repo_with_origin(&root, "https://example.com/org/repo.git");
        std::fs::create_dir_all(&sibling).unwrap();
        std::fs::write(sibling.join("notes.txt"), "no git here").unwrap();

        let hits = detect_sprawl(&ctx_for(root));
        assert!(hits.is_empty(), "non-git sibling must be ignored: {hits:?}");
    }

    #[test]
    fn root_without_origin_yields_nothing() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("repo-a");
        let sibling = parent.path().join("repo-b");
        std::fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]); // no origin remote
        init_repo_with_origin(&sibling, "https://example.com/org/repo.git");

        assert!(detect_sprawl(&ctx_for(root)).is_empty());
    }

    #[test]
    fn advisory_ratchet_readiness_no_auto_fail() {
        // A repo with no same-origin sibling produces zero findings, so the
        // guard can never auto-fail a currently-green repo (ratchet-ready).
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("solo");
        init_repo_with_origin(&root, "https://example.com/org/solo.git");
        assert!(detect_sprawl(&ctx_for(root)).is_empty());
    }

    #[test]
    fn origin_normalization_matches_ssh_and_https() {
        assert!(origins_match(
            "https://example.com/org/repo.git",
            "git@example.com:org/repo"
        ));
        assert!(origins_match(
            "ssh://git@example.com/org/repo.git",
            "https://example.com/org/repo/"
        ));
        assert!(!origins_match(
            "https://example.com/org/repo.git",
            "https://example.com/org/other.git"
        ));
    }
}
