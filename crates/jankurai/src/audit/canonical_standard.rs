//! HLT-047-CANONICAL-README and HLT-048-CANONICAL-CI-GAP detectors
//! (Jankurai pillar, canonical-shape guard).
//!
//! Validates that a repository's README and CI match the canonical agent-native
//! shape described in `agent/JANKURAI_STANDARD.md`:
//!
//! - **README (HLT-047)** should link `AGENTS.md` (so agents find the entrypoint),
//!   state the target stack, carry a status/score badge, and offer a quick-start
//!   (install / getting-started) section.
//! - **CI (HLT-048)** should delegate to versioned `ops/ci/*.sh` scripts (CI Local
//!   Parity), pin every `uses:` action to a full 40-character commit SHA, and run
//!   a jankurai audit lane.
//!
//! Both rules are ADVISORY (registered, not in `fail_on`). Each check only fires
//! when the relevant artifact exists: a repo with no README never trips HLT-047,
//! and a repo with no `.github/workflows/*` never trips HLT-048. A repo that
//! already matches the canonical shape (such as jankurai) yields zero findings
//! and stays ratchet-ready.

use crate::audit::helpers::AuditContext;
use crate::audit::scan::FindingHit;
use crate::model::FileInfo;
use once_cell::sync::Lazy;
use regex::Regex;

/// Canonical-shape policy thresholds read from `[canonical]` in
/// `agent/audit-policy.toml`. Defaults keep both checks advisory and disabled
/// only when explicitly turned off.
struct CanonicalPolicy {
    check_readme: bool,
    check_ci: bool,
    require_badge: bool,
    require_quick_start: bool,
    require_stack: bool,
    require_agents_link: bool,
}

impl Default for CanonicalPolicy {
    fn default() -> Self {
        Self {
            check_readme: true,
            check_ci: true,
            require_badge: true,
            require_quick_start: true,
            require_stack: true,
            require_agents_link: true,
        }
    }
}

fn load_policy(ctx: &AuditContext) -> CanonicalPolicy {
    let mut policy = CanonicalPolicy::default();
    let path = ctx.root.join("agent/audit-policy.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return policy;
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return policy;
    };
    let Some(section) = value.get("canonical") else {
        return policy;
    };
    let flag = |key: &str, default: bool| {
        section
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    };
    policy.check_readme = flag("check_readme", policy.check_readme);
    policy.check_ci = flag("check_ci", policy.check_ci);
    policy.require_badge = flag("require_badge", policy.require_badge);
    policy.require_quick_start = flag("require_quick_start", policy.require_quick_start);
    policy.require_stack = flag("require_stack", policy.require_stack);
    policy.require_agents_link = flag("require_agents_link", policy.require_agents_link);
    policy
}

/// Locates the root README file (case-insensitive `README.md` / `README`).
fn readme_file(ctx: &AuditContext) -> Option<&FileInfo> {
    ctx.all_files.iter().find(|file| {
        let lower = file.rel_path.to_ascii_lowercase();
        lower == "readme.md" || lower == "readme" || lower == "readme.markdown"
    })
}

/// Detects README gaps against the canonical agent-native shape. Returns one
/// [`FindingHit`] per missing canonical element so an agent can repair each gap
/// independently. No README => no findings.
pub fn detect_readme_gaps(ctx: &AuditContext) -> Vec<FindingHit> {
    let policy = load_policy(ctx);
    if !policy.check_readme {
        return vec![];
    }
    let Some(readme) = readme_file(ctx) else {
        return vec![];
    };
    let text = &readme.text;
    let lower = text.to_ascii_lowercase();
    let path = readme.rel_path.as_str();
    let mut hits = Vec::new();

    if policy.require_agents_link && !links_agents_md(text) {
        hits.push(readme_hit(
            path,
            "AGENTS.md link",
            "README does not link `AGENTS.md`, so agents cannot find the repository entrypoint",
            "add a link to `AGENTS.md` (the agent entrypoint) near the top of the README",
        ));
    }
    if policy.require_stack && !states_target_stack(&lower) {
        hits.push(readme_hit(
            path,
            "target stack",
            "README does not state the target stack, so contributors cannot tell what the repo is built on",
            "state the target stack (for example Rust core, TypeScript/React product surface, PostgreSQL) in the README intro",
        ));
    }
    if policy.require_badge && !has_badge(text) {
        hits.push(readme_hit(
            path,
            "status badge",
            "README has no status or score badge, so build/audit health is not visible at a glance",
            "add a CI/score badge (for example a shields.io or jankurai score badge) near the top of the README",
        ));
    }
    if policy.require_quick_start && !has_quick_start(&lower) {
        hits.push(readme_hit(
            path,
            "quick-start",
            "README has no quick-start (install / getting-started) section, so the first-run path is undocumented",
            "add a `## Quick start` (or install / getting-started) section with the minimal commands to run the project",
        ));
    }
    hits
}

/// Detects CI gaps against the canonical CI-Local-Parity shape. Returns one
/// [`FindingHit`] per missing canonical element. No workflows => no findings.
pub fn detect_ci_gaps(ctx: &AuditContext) -> Vec<FindingHit> {
    let policy = load_policy(ctx);
    if !policy.check_ci {
        return vec![];
    }
    let workflows: Vec<&FileInfo> = ctx
        .all_files
        .iter()
        .filter(|file| is_workflow(file))
        .collect();
    if workflows.is_empty() {
        return vec![];
    }
    let anchor = workflows[0].rel_path.as_str();
    let mut hits = Vec::new();

    let delegates = workflows.iter().any(|file| calls_ops_ci(file));
    if !delegates {
        hits.push(ci_hit(
            anchor,
            "ops/ci delegation",
            "no workflow delegates to a versioned `ops/ci/*.sh` script, so CI cannot be reproduced locally before push",
            "move CI commands into `ops/ci/*.sh` and call `bash ops/ci/<lane>.sh` from the workflow",
        ));
    }

    // Flag every workflow that pins an action to a floating tag instead of a full
    // 40-character commit SHA. Floating tags (`@v4`, `@main`) are not reproducible
    // and are a supply-chain hazard.
    for file in &workflows {
        if let Some(unpinned) = first_unpinned_use(file) {
            let mut hit = ci_hit(
                &file.rel_path,
                "pinned action SHA",
                "workflow pins a GitHub Action to a floating tag instead of a full commit SHA",
                "pin every `uses:` action to a full 40-character commit SHA so the workflow is reproducible",
            );
            hit.text = format!("unpinned action `uses: {unpinned}`");
            hits.push(hit);
        }
    }

    let has_audit_lane = workflows.iter().any(|file| runs_jankurai_audit(file));
    if !has_audit_lane {
        hits.push(ci_hit(
            anchor,
            "jankurai audit lane",
            "CI has no jankurai audit lane, so merges are not gated on a repository conformance score",
            "add a jankurai audit lane (for example `bash ops/ci/audit.sh` or a `jankurai audit` step) to CI",
        ));
    }
    hits
}

fn readme_hit(path: &str, element: &str, problem: &str, fix: &str) -> FindingHit {
    FindingHit {
        path: path.to_string(),
        line: Some(1),
        text: format!("README missing canonical element: {element}"),
        matched_term: Some("canonical-readme".into()),
        agent_fix: fix.to_string(),
        problem: problem.to_string(),
    }
}

fn ci_hit(path: &str, element: &str, problem: &str, fix: &str) -> FindingHit {
    FindingHit {
        path: path.to_string(),
        line: Some(1),
        text: format!("CI missing canonical element: {element}"),
        matched_term: Some("canonical-ci".into()),
        agent_fix: fix.to_string(),
        problem: problem.to_string(),
    }
}

fn links_agents_md(text: &str) -> bool {
    text.contains("AGENTS.md")
}

fn states_target_stack(lower: &str) -> bool {
    // The canonical stack is Rust-first; any explicit stack statement that names
    // a primary stack language counts. Keep this permissive so a repo that names
    // its real stack is never nagged.
    [
        "rust",
        "typescript",
        "react",
        "postgres",
        "python",
        "go",
        "node",
    ]
    .iter()
    .any(|stack| lower.contains(stack))
}

fn has_badge(text: &str) -> bool {
    static BADGE_RE: Lazy<Regex> = Lazy::new(|| {
        // A Markdown image badge `[![alt](img)](link)` or a bare image `![alt](img)`,
        // or an explicit jankurai badge marker.
        Regex::new(r"!\[[^\]]*\]\([^)]*\)").expect("badge regex is valid")
    });
    BADGE_RE.is_match(text)
        || text.contains("shields.io")
        || text.contains("jankurai-badge")
        || text.contains("badge.svg")
}

fn has_quick_start(lower: &str) -> bool {
    lower.contains("quick start")
        || lower.contains("quickstart")
        || lower.contains("quick-start")
        || lower.contains("getting started")
        || lower.contains("## install")
        || lower.contains("# install")
        || lower.contains("## usage")
}

fn is_workflow(file: &FileInfo) -> bool {
    let lower = file.rel_path.to_ascii_lowercase();
    lower.starts_with(".github/workflows/") && (lower.ends_with(".yml") || lower.ends_with(".yaml"))
}

fn calls_ops_ci(file: &FileInfo) -> bool {
    static OPS_CI_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?m)\bbash\s+ops/ci/[A-Za-z0-9_./-]+\.sh").expect("regex"));
    OPS_CI_RE.is_match(&file.text)
}

fn runs_jankurai_audit(file: &FileInfo) -> bool {
    let lower = file.text.to_ascii_lowercase();
    lower.contains("ops/ci/audit.sh")
        || lower.contains("jankurai audit")
        || (lower.contains("jankurai") && lower.contains("audit"))
}

/// Returns the first `uses:` reference in `file` that is NOT pinned to a full
/// 40-character commit SHA (i.e. uses a floating tag or branch), if any.
fn first_unpinned_use(file: &FileInfo) -> Option<String> {
    static USES_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?m)^\s*-?\s*uses:\s*(\S+)").expect("uses regex is valid"));
    static SHA_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@[0-9a-f]{40}$").expect("sha regex"));
    for caps in USES_RE.captures_iter(&file.text) {
        let reference = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        // Local composite actions (`./.github/...`) and docker refs are not SHA-pinnable.
        if reference.starts_with("./") || reference.starts_with("docker://") {
            continue;
        }
        if !reference.contains('@') {
            return Some(reference.to_string());
        }
        if !SHA_RE.is_match(reference) {
            return Some(reference.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn file(rel: &str, text: &str) -> FileInfo {
        FileInfo {
            rel_path: rel.into(),
            name: rel.rsplit('/').next().unwrap_or(rel).into(),
            suffix: format!(".{}", rel.rsplit('.').next().unwrap_or("")),
            size: text.len() as u64,
            line_count: text.lines().count(),
            text: text.into(),
            is_generated: false,
            is_code: false,
        }
    }

    fn ctx_for(files: Vec<FileInfo>) -> AuditContext {
        AuditContext {
            root: PathBuf::from("/nonexistent-canonical-root"),
            all_files: files,
            scope_files: vec![],
            scope_paths: vec![],
            self_audit: false,
            boundary_reclassifications: vec![],
            copy_code: None,
        }
    }

    const GOOD_README: &str = "# My Project\n\
        [![CI](https://img.shields.io/badge/ci-green.svg)](ci)\n\n\
        Built on a Rust core with a PostgreSQL durable store.\n\n\
        See [AGENTS.md](AGENTS.md) for the agent entrypoint.\n\n\
        ## Quick start\n\n```\ncargo install --path .\n```\n";

    const GOOD_WORKFLOW: &str = concat!(
        "name: ci\n",
        "jobs:\n",
        "  build:\n",
        "    runs-on: ubuntu-latest\n",
        "    steps:\n",
        "      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd\n",
        "      - name: gates\n",
        "        run: bash ops/ci/quality-gates.sh\n",
        "  audit:\n",
        "    steps:\n",
        "      - name: audit\n",
        "        run: bash ops/ci/audit.sh\n",
    );

    #[test]
    fn canonical_readme_yields_no_findings() {
        let hits = detect_readme_gaps(&ctx_for(vec![file("README.md", GOOD_README)]));
        assert!(hits.is_empty(), "canonical README must be clean: {hits:?}");
    }

    #[test]
    fn missing_readme_yields_no_findings() {
        let hits = detect_readme_gaps(&ctx_for(vec![file("src/main.rs", "fn main() {}")]));
        assert!(hits.is_empty(), "no README => no HLT-047: {hits:?}");
    }

    #[test]
    fn readme_missing_agents_link_and_quick_start_fires() {
        let bare = "# Project\n\
            [![CI](https://img.shields.io/badge/ci-green.svg)](ci)\n\n\
            Built on Rust.\n";
        let hits = detect_readme_gaps(&ctx_for(vec![file("README.md", bare)]));
        assert_eq!(hits.len(), 2, "missing AGENTS link + quick-start: {hits:?}");
        assert!(hits
            .iter()
            .all(|h| h.matched_term.as_deref() == Some("canonical-readme")));
        assert!(hits.iter().any(|h| h.text.contains("AGENTS.md")));
        assert!(hits.iter().any(|h| h.text.contains("quick-start")));
    }

    #[test]
    fn readme_missing_badge_and_stack_fires() {
        let bare = "# Project\n\nSee [AGENTS.md](AGENTS.md).\n\n## Quick start\nrun it\n";
        let hits = detect_readme_gaps(&ctx_for(vec![file("README.md", bare)]));
        assert!(hits.iter().any(|h| h.text.contains("status badge")));
        assert!(hits.iter().any(|h| h.text.contains("target stack")));
    }

    #[test]
    fn canonical_ci_yields_no_findings() {
        let hits = detect_ci_gaps(&ctx_for(vec![file(
            ".github/workflows/ci.yml",
            GOOD_WORKFLOW,
        )]));
        assert!(hits.is_empty(), "canonical CI must be clean: {hits:?}");
    }

    #[test]
    fn missing_workflows_yields_no_findings() {
        let hits = detect_ci_gaps(&ctx_for(vec![file("README.md", GOOD_README)]));
        assert!(hits.is_empty(), "no workflows => no HLT-048: {hits:?}");
    }

    #[test]
    fn ci_without_ops_delegation_and_audit_fires() {
        let inline = concat!(
            "name: ci\n",
            "jobs:\n",
            "  build:\n",
            "    steps:\n",
            "      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd\n",
            "      - run: cargo test\n",
        );
        let hits = detect_ci_gaps(&ctx_for(vec![file(".github/workflows/ci.yml", inline)]));
        assert!(hits.iter().any(|h| h.text.contains("ops/ci delegation")));
        assert!(hits.iter().any(|h| h.text.contains("jankurai audit lane")));
        assert!(hits
            .iter()
            .all(|h| h.matched_term.as_deref() == Some("canonical-ci")));
    }

    #[test]
    fn ci_with_floating_tag_fires() {
        let floating = concat!(
            "name: ci\n",
            "jobs:\n",
            "  build:\n",
            "    steps:\n",
            "      - uses: actions/checkout@v4\n",
            "      - run: bash ops/ci/audit.sh\n",
        );
        let hits = detect_ci_gaps(&ctx_for(vec![file(".github/workflows/ci.yml", floating)]));
        assert!(
            hits.iter().any(|h| h.text.contains("actions/checkout@v4")),
            "floating tag must be flagged: {hits:?}"
        );
    }

    #[test]
    fn policy_can_disable_checks() {
        // With no policy file on disk (root does not exist), defaults apply and a
        // bare README fires; this guards the default-on behavior.
        let bare = "# Project\n";
        let hits = detect_readme_gaps(&ctx_for(vec![file("README.md", bare)]));
        assert!(!hits.is_empty(), "defaults keep README checks enabled");
    }
}
