use crate::audit::copy_code::{
    render_markdown, scan_repo, CopyCodeClass, CopyCodeKind, CopyCodeOptions, DEFAULT_JSON_PATH,
    DEFAULT_MD_PATH,
};
use crate::audit::copy_code_cross_check;
use crate::validation::{self, ArtifactSchema};
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args, Debug, Clone)]
pub struct CopyCodeArgs {
    #[command(subcommand)]
    pub action: Option<CopyCodeAction>,

    #[arg(default_value = ".")]
    pub repo: PathBuf,
    #[arg(long, default_value = DEFAULT_JSON_PATH)]
    pub json: String,
    #[arg(long, default_value = DEFAULT_MD_PATH)]
    pub md: String,
    #[arg(long, default_value_t = crate::audit::copy_code::DEFAULT_MIN_LINES)]
    pub min_lines: usize,
    #[arg(long, default_value_t = crate::audit::copy_code::DEFAULT_MIN_TOKENS)]
    pub min_tokens: usize,
    #[arg(long, default_value_t = crate::audit::copy_code::DEFAULT_MAX_FINDINGS)]
    pub max_findings: usize,
    #[arg(long)]
    pub include_tests: bool,
    #[arg(long)]
    pub strict: bool,
    /// Optional external cross-check tool (currently: jscpd). Advisory only; never affects score.
    #[arg(long, value_name = "TOOL")]
    pub cross_check: Option<String>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CopyCodeAction {
    /// Print a stack-rank table of redundancy classes sorted by total redundant volume.
    Rank(RankArgs),
}

#[derive(Args, Debug, Clone)]
pub struct RankArgs {
    #[arg(default_value = ".")]
    pub repo: PathBuf,
    /// Number of classes to show.
    #[arg(long, default_value_t = 20)]
    pub top: usize,
    /// Sort key: lines (default), tokens, or bytes.
    #[arg(long, default_value = "lines")]
    pub by: String,
    /// Filter by kind: all (default), hard-only, exact-file, exact-unit-same-name,
    /// exact-unit-different-name, token-block.
    #[arg(long, default_value = "all")]
    pub kind: String,
    #[arg(long)]
    pub include_tests: bool,
}

impl Default for CopyCodeArgs {
    fn default() -> Self {
        Self {
            action: None,
            repo: PathBuf::from("."),
            json: DEFAULT_JSON_PATH.into(),
            md: DEFAULT_MD_PATH.into(),
            min_lines: crate::audit::copy_code::DEFAULT_MIN_LINES,
            min_tokens: crate::audit::copy_code::DEFAULT_MIN_TOKENS,
            max_findings: crate::audit::copy_code::DEFAULT_MAX_FINDINGS,
            include_tests: false,
            strict: false,
            cross_check: None,
        }
    }
}

pub fn run(args: CopyCodeArgs) -> Result<()> {
    match args.action.clone() {
        Some(CopyCodeAction::Rank(r)) => run_rank(r),
        None => run_full(args),
    }
}

fn run_full(args: CopyCodeArgs) -> Result<()> {
    if args.json == "-" && args.md == "-" {
        anyhow::bail!("use at most one stdout target; JSON and Markdown may not share stdout");
    }
    let repo_root = args.repo.canonicalize()?;
    let mut report = scan_repo(
        &repo_root,
        CopyCodeOptions {
            min_lines: args.min_lines,
            min_tokens: args.min_tokens,
            max_findings: args.max_findings,
            include_tests: args.include_tests,
            strict: args.strict,
        },
    )?;

    if let Some(tool) = args.cross_check.as_deref() {
        let out_dir = repo_root.join("target/jankurai/cross-check");
        let result = match tool {
            "jscpd" => copy_code_cross_check::run_jscpd(&repo_root, &out_dir)?,
            other => anyhow::bail!("unknown --cross-check tool `{other}`; supported: jscpd"),
        };
        let note = if result.available {
            if let Some(n) = result.duplicate_count {
                format!(
                    "cross-check[jscpd]: {n} clone clusters; raw={:?}",
                    result.raw_path
                )
            } else {
                "cross-check[jscpd]: ran but did not produce a clone count".to_string()
            }
        } else {
            format!(
                "cross-check[jscpd]: {}",
                result.note.clone().unwrap_or_default()
            )
        };
        report.notes.push(note);
    }

    validation::write_json(&repo_root, ArtifactSchema::CopyCode, &args.json, &report)?;
    crate::render::write_markdown(&args.md, &render_markdown(&report))?;
    eprintln!(
        "copy-code status={} hard={} warning={} classes={} json={} md={}",
        report.status,
        report.summary.hard_classes,
        report.summary.warning_classes,
        report.classes.len(),
        args.json,
        args.md
    );
    if args.strict && report.summary.hard_classes > 0 {
        anyhow::bail!(
            "copy-code strict mode failed: hard_classes={}",
            report.summary.hard_classes
        );
    }
    Ok(())
}

fn run_rank(args: RankArgs) -> Result<()> {
    let repo_root = args.repo.canonicalize()?;
    let report = scan_repo(
        &repo_root,
        CopyCodeOptions {
            include_tests: args.include_tests,
            ..CopyCodeOptions::default()
        },
    )?;
    let key = parse_rank_key(&args.by)?;
    let mut rows: Vec<&CopyCodeClass> = report
        .classes
        .iter()
        .filter(|c| matches_kind_filter(c, &args.kind))
        .collect();
    rows.sort_by_key(|c| std::cmp::Reverse(rank_value(c, key)));
    rows.truncate(args.top);
    print_rank_table(&rows, key, &args);
    Ok(())
}

fn matches_kind_filter(c: &CopyCodeClass, kind: &str) -> bool {
    match kind {
        "all" => true,
        "hard-only" => c.hard_fail,
        "exact-file" => matches!(c.kind, CopyCodeKind::ExactFile),
        "exact-unit-same-name" => matches!(c.kind, CopyCodeKind::ExactUnitSameName),
        "exact-unit-different-name" => matches!(c.kind, CopyCodeKind::ExactUnitDifferentName),
        "token-block" => matches!(c.kind, CopyCodeKind::TokenBlock),
        _ => true,
    }
}

fn parse_rank_key(s: &str) -> Result<&'static str> {
    match s {
        "lines" => Ok("lines"),
        "tokens" => Ok("tokens"),
        "bytes" => Ok("bytes"),
        other => anyhow::bail!("invalid --by `{other}`; use lines|tokens|bytes"),
    }
}

fn rank_value(c: &CopyCodeClass, key: &str) -> usize {
    match key {
        "tokens" => c.total_redundant_tokens,
        "bytes" => c.total_redundant_bytes,
        _ => c.total_redundant_lines,
    }
}

fn print_rank_table(rows: &[&CopyCodeClass], key: &str, args: &RankArgs) {
    println!(
        "# Copy-code rank — top {} by total_redundant_{key}",
        rows.len()
    );
    println!(
        "# kind filter: {} | include_tests: {}",
        args.kind, args.include_tests
    );
    println!();
    println!(
        "{:>4}  {:<26}  {:<10}  {:>5}  {:>10}  {:<6}  id",
        "#", "kind", "lang", "inst", "vol", "sev"
    );
    for (i, c) in rows.iter().enumerate() {
        let sev = if c.hard_fail { "HARD" } else { "warn" };
        let vol = rank_value(c, key);
        println!(
            "{:>4}  {:<26}  {:<10}  {:>5}  {:>10}  {:<6}  {}",
            i + 1,
            format!("{:?}", c.kind),
            c.language,
            c.instance_count,
            vol,
            sev,
            c.id
        );
    }
    println!();
    println!("Tip: `jankurai copy-code rank --kind hard-only` to focus on inexcusable findings.");
}
