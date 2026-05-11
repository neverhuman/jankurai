use std::path::Path;

pub fn existing_standard_files(repo: &Path) -> Vec<String> {
    [
        "AGENTS.md",
        "agent/JANKURAI_STANDARD.md",
        "agent/owner-map.json",
        "agent/test-map.json",
        "agent/boundaries.toml",
    ]
    .into_iter()
    .filter(|rel| repo.join(rel).exists())
    .map(str::to_string)
    .collect()
}

pub fn detect_surfaces(repo: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for (surface, markers) in [
        ("rust", &["Cargo.toml"][..]),
        ("node", &["package.json"][..]),
        (
            "typescript",
            &["tsconfig.json", "packages/ux-qa/tsconfig.json"][..],
        ),
        (
            "vite-react",
            &[
                "vite.config.ts",
                "vite.config.js",
                "apps/web/vite.config.ts",
            ][..],
        ),
        ("postgres", &["db", "migrations"][..]),
        ("python", &["python"][..]),
        ("github-ci", &[".github/workflows"][..]),
        ("agent-files", &["AGENTS.md", "CLAUDE.md", "GEMINI.md"][..]),
    ] {
        if markers.iter().any(|marker| repo.join(marker).exists()) {
            out.push(surface.to_string());
        }
    }
    out
}
