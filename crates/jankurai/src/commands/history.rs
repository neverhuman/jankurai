use crate::render::{write_json, write_markdown};
use crate::score_history::{
    build_history_export, build_history_latest, compact_history_file,
    render_history_export_markdown, restore_history_file, HistorySource,
};
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct LatestArgs {
    pub history: PathBuf,
    pub source: String,
    pub out: String,
}

#[derive(Debug, Clone)]
pub struct ExportArgs {
    pub history: PathBuf,
    pub window: usize,
    pub source: String,
    pub out: String,
    pub md: String,
}

#[derive(Debug, Clone)]
pub struct CompactArgs {
    pub history: PathBuf,
    pub max_rows: usize,
    pub max_bytes: usize,
    pub source: String,
    pub json: String,
    pub md: String,
}

#[derive(Debug, Clone)]
pub struct RestoreArgs {
    pub mirror: PathBuf,
    pub repo_id: String,
    pub out: String,
    pub max_rows: usize,
    pub max_bytes: usize,
    pub source: String,
    pub json: String,
    pub md: String,
}

pub fn run_latest(args: LatestArgs) -> Result<()> {
    let latest = build_history_latest(&args.history)?;
    let _ = history_source(&args.source);
    write_json(&args.out, &serde_json::to_string_pretty(&latest)?)?;
    Ok(())
}

pub fn run_export(args: ExportArgs) -> Result<()> {
    let source = history_source(&args.source);
    let report = build_history_export(&args.history, args.window, source)?;
    write_json(&args.out, &serde_json::to_string_pretty(&report)?)?;
    write_markdown(&args.md, &render_history_export_markdown(&report))?;
    Ok(())
}

pub fn run_compact(args: CompactArgs) -> Result<()> {
    let source = history_source(&args.source);
    let compacted = compact_history_file(&args.history, args.max_rows, args.max_bytes)?;
    let report = build_history_export(&args.history, compacted.len(), source)?;
    write_json(&args.json, &serde_json::to_string_pretty(&report)?)?;
    write_markdown(&args.md, &render_history_export_markdown(&report))?;
    Ok(())
}

pub fn run_restore(args: RestoreArgs) -> Result<()> {
    let source = history_source(&args.source);
    let out_path = PathBuf::from(&args.out);
    let restored = restore_history_file(
        &args.mirror,
        &args.repo_id,
        &out_path,
        args.max_rows,
        args.max_bytes,
    )?;
    let report = build_history_export(&out_path, restored.len(), source)?;
    write_json(&args.json, &serde_json::to_string_pretty(&report)?)?;
    write_markdown(&args.md, &render_history_export_markdown(&report))?;
    Ok(())
}

fn history_source(source: &str) -> HistorySource {
    match source {
        "local" => HistorySource::Local,
        "mirror" => HistorySource::Mirror,
        _ => HistorySource::Auto,
    }
}
