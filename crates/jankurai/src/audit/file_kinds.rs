use std::path::Path;

const TEXT_BASENAMES: &[&str] = &[
    "AGENTS.md",
    "CODEOWNERS",
    "Cargo.lock",
    "Cargo.toml",
    "Dockerfile",
    "Gemfile",
    "Gemfile.lock",
    "Justfile",
    "LICENSE",
    "Makefile",
    "Pipfile",
    "Pipfile.lock",
    "Procfile",
    "README",
    "README.md",
    "Taskfile.yaml",
    "Taskfile.yml",
    "build.gradle",
    "build.gradle.kts",
    "bunfig.toml",
    "clippy.toml",
    "go.mod",
    "go.sum",
    "justfile",
    "makefile",
    "package-lock.json",
    "package.json",
    "applypatch-msg",
    "commit-msg",
    "fsmonitor-watchman",
    "post-applypatch",
    "post-checkout",
    "post-commit",
    "post-index-change",
    "post-merge",
    "post-receive",
    "post-rewrite",
    "post-update",
    "pre-applypatch",
    "pre-auto-gc",
    "pre-commit",
    "pre-merge-commit",
    "pre-push",
    "pre-rebase",
    "pre-receive",
    "prepare-commit-msg",
    "proc-receive",
    "push-to-checkout",
    "sendemail-validate",
    "update",
    "pnpm-lock.yaml",
    "poetry.lock",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "tsconfig.json",
    "uv.lock",
    "yarn.lock",
];

const TEXT_EXTS: &[&str] = &[
    ".c",
    ".cc",
    ".cfg",
    ".cjs",
    ".conf",
    ".cpp",
    ".cs",
    ".css",
    ".dart",
    ".d.ts",
    ".dockerfile",
    ".env",
    ".gitattributes",
    ".gitignore",
    ".go",
    ".gql",
    ".graphql",
    ".h",
    ".hh",
    ".hpp",
    ".htm",
    ".html",
    ".ini",
    ".java",
    ".js",
    ".json",
    ".jsx",
    ".kt",
    ".kts",
    ".ex",
    ".exs",
    ".lua",
    ".m",
    ".md",
    ".mk",
    ".mjs",
    ".mm",
    ".ps1",
    ".php",
    ".py",
    ".rb",
    ".rst",
    ".rs",
    ".sh",
    ".sql",
    ".swift",
    ".scala",
    ".tex",
    ".toml",
    ".ts",
    ".tsx",
    ".txt",
    ".zyal",
    ".yaml",
    ".yml",
];

const CODE_EXTS: &[&str] = &[
    ".c", ".cc", ".cpp", ".cs", ".dart", ".go", ".h", ".hh", ".hpp", ".java", ".js", ".jsx", ".kt",
    ".kts", ".ex", ".exs", ".lua", ".m", ".mm", ".py", ".php", ".rb", ".rs", ".sh", ".swift",
    ".scala", ".ts", ".tsx",
];

pub(crate) fn suffix_of(rel_path: &str) -> String {
    let lower = rel_path.to_ascii_lowercase();
    if lower.ends_with(".d.ts") {
        ".d.ts".to_string()
    } else {
        Path::new(rel_path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| format!(".{}", s.to_ascii_lowercase()))
            .unwrap_or_default()
    }
}

pub(crate) fn is_text_candidate(name: &str, suffix: &str, rel_path: &str) -> bool {
    let lower = rel_path.to_ascii_lowercase();
    TEXT_BASENAMES
        .iter()
        .any(|item| item.eq_ignore_ascii_case(name))
        || TEXT_EXTS.iter().any(|ext| lower.ends_with(ext))
        || matches!(suffix, ".dockerfile")
        || matches!(
            name.to_ascii_lowercase().as_str(),
            "dockerfile" | "makefile" | "justfile"
        )
}

pub(crate) fn is_code_file(name: &str, suffix: &str) -> bool {
    matches!(name, "Makefile" | "makefile" | "Justfile" | "justfile") || CODE_EXTS.contains(&suffix)
}
