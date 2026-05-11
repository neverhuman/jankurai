use crate::model::FileInfo;

pub fn is_free_prose_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".tex") || lower.ends_with(".txt")
}

pub fn is_trusted_policy_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let basename = lower.rsplit('/').next().unwrap_or(&lower);
    basename == "agents.md"
        || basename == "claude.md"
        || basename == "gemini.md"
        || matches!(
            lower.as_str(),
            "db/readme.md"
                | "db/migrations/readme.md"
                | "db/constraints/readme.md"
                | "docs/cost.md"
                | "docs/release.md"
                | "docs/release-plan.md"
                | "docs/provenance.md"
                | "docs/rollback.md"
                | "docs/review.md"
                | "docs/testing.md"
        )
        || lower.starts_with("agent/")
        || lower.starts_with(".agents/")
        || lower.starts_with(".github/")
        || lower.starts_with(".cursor/")
        || lower.starts_with(".claude/")
}

pub fn is_word_neutral_prose_path(path: &str) -> bool {
    is_free_prose_path(path) && !is_trusted_policy_path(path)
}

pub fn allows_word_scan(file: &FileInfo) -> bool {
    !is_word_neutral_prose_path(&file.rel_path)
}

pub fn fingerprint_text(file: &FileInfo) -> Option<&str> {
    allows_word_scan(file).then_some(file.text.as_str())
}
