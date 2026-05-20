use std::path::{Path, PathBuf};

pub const LOCAL_ROOT: &str = ".jankurai";
pub const SCORE_JSON: &str = ".jankurai/repo-score.json";
pub const SCORE_MD: &str = ".jankurai/repo-score.md";
pub const SCORE_HISTORY_JSONL: &str = ".jankurai/score-history.jsonl";
pub const SCORE_HISTORY_CSV: &str = ".jankurai/score-history.csv";
pub const POSTMORTEM_ROOT: &str = ".jankurai/postmortems";

pub const LEGACY_SCORE_JSON: &str = "agent/repo-score.json";
pub const LEGACY_SCORE_MD: &str = "agent/repo-score.md";
pub const LEGACY_SCORE_HISTORY_JSONL: &str = "agent/score-history.jsonl";
pub const LEGACY_SCORE_HISTORY_CSV: &str = "agent/score-history.csv";

pub fn preferred_repo_path(repo: &Path, local: &str, legacy: Option<&str>) -> PathBuf {
    let local_path = repo.join(local);
    if local_path.exists() {
        return local_path;
    }
    if let Some(legacy) = legacy {
        let legacy_path = repo.join(legacy);
        if legacy_path.exists() {
            return legacy_path;
        }
    }
    local_path
}

pub fn preferred_repo_path_str(repo: &Path, local: &str, legacy: Option<&str>) -> String {
    preferred_repo_path(repo, local, legacy)
        .display()
        .to_string()
}
