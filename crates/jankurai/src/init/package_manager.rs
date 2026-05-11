use std::path::Path;

pub fn detect_package_manager(repo: &Path) -> &'static str {
    if repo.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if repo.join("yarn.lock").exists() {
        "yarn"
    } else if repo.join("package-lock.json").exists() || repo.join("package.json").exists() {
        "npm"
    } else if repo.join("Cargo.toml").exists() {
        "cargo"
    } else {
        "unknown"
    }
}
