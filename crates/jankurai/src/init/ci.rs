pub fn audit_command(mode: &str) -> String {
    format!("jankurai audit . --mode {mode} --json .jankurai/repo-score.json --md .jankurai/repo-score.md")
}
