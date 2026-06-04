//! HLT-046-UNNECESSARY-VARIETY detector (Jankurai pillar, variety guard).
//!
//! Detects redundant *variety* where consistency is expected: the same logical
//! definition is spelled out in two or more active-source modules with diverging
//! shapes. The canonical smell is a duplicated PUBLIC enum that is declared
//! independently in several files instead of being defined once and imported,
//! which lets the variant sets drift apart over time. The guard is deliberately
//! scoped to public enums (shared domain types) so it stays high-precision;
//! private definitions and const/static values are excluded.
//!
//! This is the inverse of the copy-code lane (`audit::copy_code`): copy-code
//! flags *identical* duplicates, while this guard flags same-name definitions
//! whose normalized shapes have *diverged*. Identical copies are deferred to the
//! copy-code lane and never double-reported here.
//!
//! The guard reuses the shared token-normalization idiom from
//! `audit::copy_code::normalize_token_line` so divergence is judged on code
//! *structure*, ignoring whitespace and comment differences.
//!
//! It is ADVISORY (registered, but not in `fail_on`) and gated behind a
//! configurable `min_instance` threshold (`[variety] min_instance` in
//! `agent/audit-policy.toml`, default 2). A definition that appears in fewer than
//! `min_instance` active files can never fire, so a repository that defines each
//! enum/const once (such as jankurai) yields zero findings and stays
//! ratchet-ready.

use crate::audit::copy_code::normalize_token_line;
use crate::audit::helpers::AuditContext;
use crate::audit::scan::FindingHit;
use crate::model::FileInfo;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeMap;

/// Default minimum number of distinct active files a definition must appear in
/// before divergent variety is reported. A value below 2 is meaningless (one
/// occurrence is never variety) and is clamped up to 2.
const DEFAULT_MIN_INSTANCE: usize = 2;

/// One declaration of a named public enum found in an active-source file,
/// captured with its normalized body shape so divergence can be judged
/// structurally.
#[derive(Clone)]
struct Declaration {
    name: String,
    path: String,
    line: usize,
    kind: &'static str,
    /// Normalized token shape of the definition body, used to decide whether two
    /// same-name declarations have diverged.
    shape: String,
}

/// Detects same-name public enum definitions that are declared in `min_instance`
/// or more distinct active-source files with diverging shapes. Returns one
/// [`FindingHit`] per redundant definition name (anchored at the first
/// occurrence) so a report lists each divergent name once.
pub fn detect_variety(ctx: &AuditContext) -> Vec<FindingHit> {
    let min_instance = configured_min_instance(ctx).max(DEFAULT_MIN_INSTANCE);

    // name -> declarations across active files.
    let mut by_name: BTreeMap<String, Vec<Declaration>> = BTreeMap::new();
    for file in ctx.all_files.iter() {
        if !is_active_rust_source(file) {
            continue;
        }
        for decl in declarations_in(file) {
            by_name.entry(decl.name.clone()).or_default().push(decl);
        }
    }

    let mut hits = Vec::new();
    for (name, decls) in by_name {
        // Distinct files only: re-declaring a name twice in one file is not the
        // cross-module variety we care about (and is usually a scoping pattern).
        let mut by_file: BTreeMap<&str, &Declaration> = BTreeMap::new();
        for decl in &decls {
            by_file.entry(decl.path.as_str()).or_insert(decl);
        }
        if by_file.len() < min_instance {
            continue;
        }
        // Distinct normalized shapes => the definitions have diverged. When every
        // copy is byte-for-byte identical in shape, this is a copy-code concern,
        // not unnecessary variety, so we skip it (copy-code owns that signal).
        let shapes: std::collections::BTreeSet<&str> =
            by_file.values().map(|d| d.shape.as_str()).collect();
        if shapes.len() < 2 {
            continue;
        }
        let mut locations: Vec<&Declaration> = by_file.values().copied().collect();
        locations.sort_by(|a, b| (a.path.as_str(), a.line).cmp(&(b.path.as_str(), b.line)));
        let first = locations[0];
        let kind = first.kind;
        let where_list = locations
            .iter()
            .map(|d| format!("{}:{}", d.path, d.line))
            .collect::<Vec<_>>()
            .join(", ");
        hits.push(FindingHit {
            path: first.path.clone(),
            line: Some(first.line),
            text: format!(
                "{kind} `{name}` is defined with diverging shapes in {} modules ({where_list})",
                by_file.len()
            ),
            matched_term: Some("unnecessary-variety".into()),
            agent_fix: format!(
                "define `{name}` once in a shared module and import it everywhere, or reconcile the diverging definitions so one canonical shape is used; redundant variety lets the copies drift apart"
            ),
            problem: format!(
                "{kind} `{name}` has {} divergent definitions across modules where one consistent definition is expected",
                by_file.len()
            ),
        });
    }
    hits
}

/// Reads `[variety] min_instance` from `agent/audit-policy.toml`. Returns the
/// configured value when present and parseable, otherwise [`DEFAULT_MIN_INSTANCE`].
fn configured_min_instance(ctx: &AuditContext) -> usize {
    let path = ctx.root.join("agent/audit-policy.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return DEFAULT_MIN_INSTANCE;
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return DEFAULT_MIN_INSTANCE;
    };
    value
        .get("variety")
        .and_then(|v| v.get("min_instance"))
        .and_then(|v| v.as_integer())
        .filter(|n| *n >= 0)
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_MIN_INSTANCE)
}

/// True for Rust files that carry product/library semantics: a non-generated
/// `.rs` file that is not under a warning-only root (tests, fixtures, examples,
/// benches, conformance). Test and fixture trees legitimately redefine helper
/// shapes per case, so they are out of scope for the variety guard.
fn is_active_rust_source(file: &FileInfo) -> bool {
    if file.is_generated {
        return false;
    }
    if file.suffix != ".rs" {
        return false;
    }
    let lower = file.rel_path.to_ascii_lowercase();
    let warning_only = [
        "/tests/",
        "/test/",
        "tests/",
        "/fixtures/",
        "fixtures/",
        "/examples/",
        "examples/",
        "/benches/",
        "benches/",
        "/conformance/",
        "conformance/",
    ];
    if warning_only.iter().any(|root| lower.contains(root)) {
        return false;
    }
    // A `#[cfg(test)]`-only file or a file that is mostly tests is excluded: such
    // modules redefine fixtures, not product definitions.
    if file.name.ends_with("_test.rs") || file.name.starts_with("test_") {
        return false;
    }
    true
}

/// Extracts named PUBLIC `enum` declarations from an active Rust file, capturing
/// each declaration's normalized shape for divergence comparison.
///
/// The guard is intentionally narrow: only `pub` (or `pub(...)`) enums — shared
/// domain types where one canonical definition is genuinely expected — are
/// considered. Module-private definitions, and `const`/`static` values, are
/// excluded because same-name locals across modules are a normal Rust idiom
/// (per-module regex statics, default-path constants, per-module rule catalogs,
/// separate CLI `Commands` enums, coincidental constant-name collisions) rather
/// than unnecessary variety. Keeping the surface to public enums makes this
/// advisory guard high-precision and ratchet-ready; const/static variety can be
/// promoted later with its own threshold once a precise signal exists.
fn declarations_in(file: &FileInfo) -> Vec<Declaration> {
    static DECL_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^\s*pub(?:\([^)]*\))?\s+(enum)\s+([A-Za-z_][A-Za-z0-9_]*)\b").expect("regex")
    });
    let lines: Vec<&str> = file.text.lines().collect();
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        let line = lines[idx];
        if let Some(caps) = DECL_RE.captures(line) {
            let kind = match caps.get(1).map(|m| m.as_str()) {
                Some("enum") => "enum",
                _ => {
                    idx += 1;
                    continue;
                }
            };
            let name = caps
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let (end, body) = collect_definition(&lines, idx, kind);
            out.push(Declaration {
                name,
                path: file.rel_path.clone(),
                line: idx + 1,
                kind,
                shape: normalize_definition_shape(&body),
            });
            idx = end + 1;
        } else {
            idx += 1;
        }
    }
    out
}

/// Collects the full text of an enum definition starting at `start`. The body
/// spans the balanced brace block (or terminates on the line for a unit enum like
/// `enum X;`). Returns `(end_line_index, body_text)`. `kind` is always `"enum"`
/// for the current detector and is accepted for forward-compatibility.
fn collect_definition(lines: &[&str], start: usize, _kind: &str) -> (usize, String) {
    let mut depth = 0i32;
    let mut seen_open = false;
    let mut end = start;
    let mut body = String::new();
    for (offset, line) in lines.iter().enumerate().skip(start) {
        body.push_str(line);
        body.push('\n');
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    seen_open = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        end = offset;
        if seen_open && depth <= 0 {
            break;
        }
        // A unit enum without a brace block (`enum X;`) terminates on its line.
        if !seen_open && line.contains(';') {
            break;
        }
    }
    (end, body)
}

/// Produces a divergence key for a definition body. Two same-name declarations
/// are treated as the same canonical definition only when this key matches.
///
/// The key has two parts joined with `||`:
///
/// 1. the shared copy-code token shape (via [`normalize_token_line`]), which
///    masks identifiers/literals so trivial renames and reformatting never read
///    as divergence, and
/// 2. a value-preserving form (comments stripped, whitespace collapsed, tokens
///    kept) so an enum whose *variant set* drifts between modules is correctly
///    flagged — the copy-code masking alone would hide that.
///
/// Combining both means a group fires when definitions diverge in either
/// structure or concrete values, which is the unnecessary-variety smell.
fn normalize_definition_shape(body: &str) -> String {
    static LINE_COMMENT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"//.*$").expect("regex"));
    let token_shape = body
        .lines()
        .map(normalize_token_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let value_shape = body
        .lines()
        .map(|line| LINE_COMMENT_RE.replace(line, "").to_string())
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    format!("{token_shape}||{value_shape}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn file(rel: &str, text: &str) -> FileInfo {
        FileInfo {
            rel_path: rel.into(),
            name: rel.rsplit('/').next().unwrap_or(rel).into(),
            suffix: ".rs".into(),
            size: text.len() as u64,
            line_count: text.lines().count(),
            text: text.into(),
            is_generated: false,
            is_code: true,
        }
    }

    fn ctx_for(files: Vec<FileInfo>) -> AuditContext {
        AuditContext {
            root: PathBuf::from("/nonexistent-variety-root"),
            all_files: files,
            scope_files: vec![],
            scope_paths: vec![],
            self_audit: false,
            boundary_reclassifications: vec![],
            copy_code: None,
        }
    }

    #[test]
    fn flags_divergent_enum_across_modules() {
        let a = file(
            "crates/app/src/order.rs",
            "pub enum Status {\n    Open,\n    Closed,\n}\n",
        );
        let b = file(
            "crates/app/src/invoice.rs",
            "pub enum Status {\n    Draft,\n    Paid,\n    Void,\n}\n",
        );
        let hits = detect_variety(&ctx_for(vec![a, b]));
        assert_eq!(hits.len(), 1, "one divergent enum name expected: {hits:?}");
        assert_eq!(hits[0].matched_term.as_deref(), Some("unnecessary-variety"));
        assert!(hits[0].text.contains("Status"));
        assert!(hits[0].text.contains("enum"));
    }

    #[test]
    fn ignores_identical_copies_left_to_copy_code() {
        // Byte-identical definitions are copy-code's job, not variety's.
        let body = "pub enum Status {\n    Open,\n    Closed,\n}\n";
        let a = file("crates/app/src/order.rs", body);
        let b = file("crates/app/src/invoice.rs", body);
        let hits = detect_variety(&ctx_for(vec![a, b]));
        assert!(
            hits.is_empty(),
            "identical copies must defer to copy-code: {hits:?}"
        );
    }

    #[test]
    fn ignores_single_occurrence() {
        let a = file(
            "crates/app/src/order.rs",
            "pub enum Status {\n    Open,\n    Closed,\n}\n",
        );
        let hits = detect_variety(&ctx_for(vec![a]));
        assert!(
            hits.is_empty(),
            "a single definition is never variety: {hits:?}"
        );
    }

    #[test]
    fn ignores_test_and_fixture_trees() {
        let a = file(
            "crates/app/tests/order_test.rs",
            "pub enum Status {\n    Open,\n}\n",
        );
        let b = file(
            "crates/app/tests/fixtures/invoice.rs",
            "pub enum Status {\n    Draft,\n    Paid,\n}\n",
        );
        let hits = detect_variety(&ctx_for(vec![a, b]));
        assert!(
            hits.is_empty(),
            "test/fixture trees are out of scope: {hits:?}"
        );
    }

    #[test]
    fn ignores_private_enum_and_const_and_static() {
        // Module-private enums and any const/static are out of scope: same-name
        // locals across modules are a normal Rust idiom, not variety.
        let a = file(
            "crates/app/src/order.rs",
            "enum Status {\n    Open,\n}\npub const MAX: u32 = 3;\n",
        );
        let b = file(
            "crates/app/src/invoice.rs",
            "enum Status {\n    Draft,\n    Paid,\n}\npub const MAX: u32 = 5;\n",
        );
        let hits = detect_variety(&ctx_for(vec![a, b]));
        assert!(
            hits.is_empty(),
            "private enums and const/static are out of scope: {hits:?}"
        );
    }

    #[test]
    fn advisory_clean_tree_yields_nothing() {
        // Each public enum appears once => ratchet-ready, never auto-fails.
        let a = file("crates/app/src/a.rs", "pub enum Alpha {\n    One,\n}\n");
        let b = file("crates/app/src/b.rs", "pub enum Beta {\n    Two,\n}\n");
        assert!(detect_variety(&ctx_for(vec![a, b])).is_empty());
    }
}
