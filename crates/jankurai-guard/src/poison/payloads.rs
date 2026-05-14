//! Language-aware poison-payload generation. A poison payload is an
//! un-ignorable error header prepended to a rejected candidate file. The header
//! is chosen by file extension so that the host language's tooling immediately
//! fails when the agent re-reads the file: the agent cannot silently proceed.

/// The structured content embedded in every poison header.
#[derive(Debug, Clone)]
pub struct PoisonContent {
    /// Repo-relative path of the blocked file.
    pub path: String,
    /// Blocking rule identifiers, e.g. `["HLT-029"]`.
    pub rule_ids: Vec<String>,
    /// One-line problem descriptions from the blocking findings.
    pub problems: Vec<String>,
    /// Numbered fix steps the agent must apply.
    pub fix_steps: Vec<String>,
    /// The command to re-run the audit after fixing.
    pub rerun_command: String,
    /// Path to the full failure report.
    pub report_path: String,
}

/// The sentinel that opens the wrapped original bytes.
pub const BEGIN_SENTINEL: &str = "JANKURAI-POISON-BEGIN";
/// The sentinel that closes the wrapped original bytes.
pub const END_SENTINEL: &str = "JANKURAI-POISON-END";
/// The marker word that identifies any poison header line.
pub const MARKER: &str = "JANKURAI SAVE BLOCKED";

/// The comment style used to wrap the original bytes for a given extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentStyle {
    /// `// ...` line comments (Rust, JS/TS, Go, SQL uses its own).
    DoubleSlash,
    /// `# ...` line comments (Python, TOML, YAML, shell, INI).
    Hash,
    /// `-- ...` line comments (SQL).
    DoubleDash,
    /// No comment syntax available; a delimited plain marker is used.
    Plain,
}

impl CommentStyle {
    /// Returns the begin/end sentinel lines for this comment style.
    pub fn sentinels(self) -> (String, String) {
        match self {
            Self::DoubleSlash => (format!("// {BEGIN_SENTINEL}"), format!("// {END_SENTINEL}")),
            Self::Hash => (format!("# {BEGIN_SENTINEL}"), format!("# {END_SENTINEL}")),
            Self::DoubleDash => (format!("-- {BEGIN_SENTINEL}"), format!("-- {END_SENTINEL}")),
            Self::Plain => (
                format!(">>> {BEGIN_SENTINEL}"),
                format!("<<< {END_SENTINEL}"),
            ),
        }
    }
}

/// The classified language family of a file, derived from its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// Rust source.
    Rust,
    /// JavaScript / TypeScript family.
    Script,
    /// Python source.
    Python,
    /// Go source.
    Go,
    /// JSON document.
    Json,
    /// TOML / YAML / INI / CFG configuration.
    Config,
    /// POSIX shell script.
    Shell,
    /// SQL script.
    Sql,
    /// Markdown / plain text / unknown.
    Text,
}

impl Lang {
    /// Classifies a file by its (lowercased, dot-stripped) extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => Self::Rust,
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Self::Script,
            "py" => Self::Python,
            "go" => Self::Go,
            "json" => Self::Json,
            "toml" | "yaml" | "yml" | "ini" | "cfg" => Self::Config,
            "sh" | "bash" => Self::Shell,
            "sql" => Self::Sql,
            _ => Self::Text,
        }
    }

    /// The comment style used to wrap the original bytes for this language.
    pub fn comment_style(self) -> CommentStyle {
        match self {
            Self::Rust | Self::Script | Self::Go => CommentStyle::DoubleSlash,
            Self::Python | Self::Config | Self::Shell => CommentStyle::Hash,
            Self::Sql => CommentStyle::DoubleDash,
            Self::Json | Self::Text => CommentStyle::Plain,
        }
    }
}

/// Builds the human-readable lines shared by every poison header.
fn message_lines(content: &PoisonContent) -> Vec<String> {
    let mut lines = vec![
        format!("{MARKER}: {}", content.path),
        format!("blocking rules: {}", content.rule_ids.join(", ")),
    ];
    for problem in &content.problems {
        lines.push(format!("problem: {problem}"));
    }
    lines.push("Fix now:".to_string());
    for (idx, step) in content.fix_steps.iter().enumerate() {
        lines.push(format!("  {}. {step}", idx + 1));
    }
    lines.push(format!("re-run: {}", content.rerun_command));
    lines.push(format!("report: {}", content.report_path));
    lines
}

/// Generates the poison header (without the wrapped original bytes) for `lang`.
pub fn header_for(lang: Lang, content: &PoisonContent) -> String {
    let lines = message_lines(content);
    match lang {
        Lang::Rust => {
            let body = lines.join("\n");
            format!("compile_error!(r#\"\n{body}\n\"#);\n")
        }
        Lang::Script => {
            let body = lines.join("\n");
            format!("throw new Error(`\n{body}\n`);\n")
        }
        Lang::Python => {
            let body = lines.join("\n");
            format!("raise RuntimeError(\"\"\"\n{body}\n\"\"\")\n")
        }
        Lang::Go => {
            let mut out = String::from("package _jankurai_save_blocked\n\n");
            for line in &lines {
                out.push_str(&format!("// {line}\n"));
            }
            out
        }
        Lang::Json => {
            let mut out = format!("// {MARKER} -- this file is not valid JSON\n");
            for line in &lines {
                out.push_str(&format!("// {line}\n"));
            }
            out
        }
        Lang::Config => {
            let mut out = String::new();
            for line in &lines {
                out.push_str(&format!("# {line}\n"));
            }
            out.push_str(&format!("!!! {MARKER} -- invalid sentinel line\n"));
            out
        }
        Lang::Shell => {
            let mut out = String::from("#!/usr/bin/env bash\n");
            for line in &lines {
                out.push_str(&format!("# {line}\n"));
            }
            out.push_str("echo 'JANKURAI SAVE BLOCKED' >&2; exit 71\n");
            out
        }
        Lang::Sql => {
            let mut out = String::new();
            for line in &lines {
                out.push_str(&format!("-- {line}\n"));
            }
            out.push_str(&format!(
                "JANKURAI_SAVE_BLOCKED_INVALID_STATEMENT; -- {MARKER}\n"
            ));
            out
        }
        Lang::Text => {
            let bar = "=".repeat(60);
            let mut out = format!("{bar}\n");
            for line in &lines {
                out.push_str(&format!("  {line}\n"));
            }
            out.push_str(&format!("{bar}\n"));
            out
        }
    }
}
