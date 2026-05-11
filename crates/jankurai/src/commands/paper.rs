use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

pub struct PublicRepoScoresArgs {
    pub source: PathBuf,
    pub out: PathBuf,
}

const REQUIRED_TOP_LEVEL_KEYS: &[&str] = &[
    "generated_at",
    "jankurai_version",
    "repo_count",
    "successful",
    "failed",
    "rows",
];

const REQUIRED_OLD_ROW_KEYS: &[&str] = &[
    "rank",
    "repo",
    "score",
    "finding_count",
    "hard_findings",
    "status",
    "weak_dimensions",
];

const REQUIRED_NEW_ROW_KEYS: &[&str] = &[
    "star_rank",
    "repo",
    "stars",
    "score",
    "issues",
    "hard_findings",
    "soft_findings",
    "severity",
    "top_categories",
];

pub fn run_public_repo_scores(args: PublicRepoScoresArgs) -> Result<()> {
    let data = load_source(&args.source)?;
    let output = render(&data, &args.source, &args.out)?;
    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, output)?;
    Ok(())
}

fn load_source(path: &PathBuf) -> Result<Value> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let data: Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    let Some(object) = data.as_object() else {
        bail!("{} must contain a JSON object", path.display());
    };
    for key in REQUIRED_TOP_LEVEL_KEYS {
        if !object.contains_key(*key) {
            bail!("{} is missing top-level key `{}`", path.display(), key);
        }
    }
    let rows = rows(&data)?;
    if rows.len() as u64 != integer(&data["repo_count"])? {
        bail!(
            "{} repo_count={} but rows={}",
            path.display(),
            integer(&data["repo_count"])?,
            rows.len()
        );
    }
    let new_shape = rows.iter().any(|row| row.get("star_rank").is_some());
    if !object.contains_key("run_root") && !object.contains_key("source_artifact") {
        bail!(
            "{} is missing top-level key `run_root` or `source_artifact`",
            path.display()
        );
    }
    for (idx, row) in rows.iter().enumerate() {
        let Some(row_object) = row.as_object() else {
            bail!("{} row {} must be an object", path.display(), idx + 1);
        };
        let required_keys = if new_shape {
            REQUIRED_NEW_ROW_KEYS
        } else {
            REQUIRED_OLD_ROW_KEYS
        };
        for key in required_keys {
            if !row_object.contains_key(*key) {
                bail!(
                    "{} row {} is missing key `{}`",
                    path.display(),
                    idx + 1,
                    key
                );
            }
        }
        if new_shape && !row["top_categories"].is_array() {
            bail!(
                "{} row {} top_categories must be an array",
                path.display(),
                idx + 1
            );
        }
        if !new_shape && !row["weak_dimensions"].is_array() {
            bail!(
                "{} row {} weak_dimensions must be an array",
                path.display(),
                idx + 1
            );
        }
    }
    Ok(data)
}

fn rows(data: &Value) -> Result<&Vec<Value>> {
    data["rows"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("rows must be a list"))
}

fn text(value: &Value) -> Result<&str> {
    value
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("expected string"))
}

fn integer(value: &Value) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("expected unsigned integer"))
}

fn tex_escape(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\\' => r"\textbackslash{}".into(),
            '&' => r"\&".into(),
            '%' => r"\%".into(),
            '$' => r"\$".into(),
            '#' => r"\#".into(),
            '_' => r"\_".into(),
            '{' => r"\{".into(),
            '}' => r"\}".into(),
            '~' => r"\textasciitilde{}".into(),
            '^' => r"\textasciicircum{}".into(),
            _ => ch.to_string(),
        })
        .collect()
}

fn fmt_int(value: u64) -> String {
    let text = value.to_string();
    let mut out = String::new();
    for (idx, ch) in text.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

fn fmt_float(value: f64) -> String {
    format!("{value:.1}")
}

fn ranked_rows(rows: &[Value]) -> Vec<&Value> {
    let mut ranked = rows.iter().collect::<Vec<_>>();
    if rows.iter().all(|row| row.get("star_rank").is_some()) {
        ranked.sort_by(|a, b| {
            row_rank(a)
                .unwrap_or(0)
                .cmp(&row_rank(b).unwrap_or(0))
                .then(
                    text(&a["repo"])
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                        .cmp(&text(&b["repo"]).unwrap_or_default().to_ascii_lowercase()),
                )
        });
    } else {
        ranked.sort_by(|a, b| {
            integer(&b["score"])
                .unwrap_or(0)
                .cmp(&integer(&a["score"]).unwrap_or(0))
                .then(
                    finding_count(a)
                        .unwrap_or(0)
                        .cmp(&finding_count(b).unwrap_or(0)),
                )
                .then(
                    text(&a["repo"])
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                        .cmp(&text(&b["repo"]).unwrap_or_default().to_ascii_lowercase()),
                )
        });
    }
    ranked
}

fn row_rank(row: &Value) -> Result<u64> {
    if row.get("star_rank").is_some() {
        integer(&row["star_rank"])
    } else {
        integer(&row["rank"])
    }
}

fn stars(row: &Value) -> Result<String> {
    if row.get("stars").is_some() {
        Ok(tex_escape(text(&row["stars"])?))
    } else {
        Ok("n/a".into())
    }
}

fn finding_count(row: &Value) -> Result<u64> {
    if row.get("issues").is_some() {
        integer(&row["issues"])
    } else {
        integer(&row["finding_count"])
    }
}

fn soft_findings(row: &Value) -> Result<u64> {
    if row.get("soft_findings").is_some() {
        integer(&row["soft_findings"])
    } else {
        Ok(finding_count(row)?.saturating_sub(integer(&row["hard_findings"])?))
    }
}

fn severity_value(row: &Value) -> Option<&Value> {
    row.get("severity").or_else(|| {
        row.get("shortcomings")
            .and_then(|shortcomings| shortcomings.get("finding_summary"))
            .and_then(|summary| summary.get("by_severity"))
    })
}

fn severity_summary(row: &Value) -> String {
    let Some(severity) = severity_value(row).and_then(Value::as_object) else {
        return "none reported".into();
    };
    let labels = [
        ("low", "low"),
        ("medium", "med"),
        ("high", "high"),
        ("critical", "crit"),
    ]
    .into_iter()
    .filter_map(|(key, label)| {
        severity
            .get(key)
            .and_then(Value::as_u64)
            .filter(|count| *count > 0)
            .map(|count| format!("{label} {}", fmt_int(count)))
    })
    .collect::<Vec<_>>();
    if labels.is_empty() {
        "none reported".into()
    } else {
        labels.join("; ")
    }
}

fn top_categories(row: &Value) -> String {
    if let Some(categories) = row.get("top_categories").and_then(Value::as_array) {
        let labels = categories
            .iter()
            .filter_map(|item| {
                let name = item.get("name").and_then(Value::as_str)?;
                let count = item.get("count").and_then(Value::as_u64)?;
                Some(format!("{} {}", tex_escape(name), fmt_int(count)))
            })
            .collect::<Vec<_>>();
        if labels.is_empty() {
            "none reported".into()
        } else {
            labels.join("; ")
        }
    } else if let Some(categories) = row
        .get("shortcomings")
        .and_then(|shortcomings| shortcomings.get("finding_summary"))
        .and_then(|summary| summary.get("by_category"))
        .and_then(Value::as_object)
    {
        let mut labels = categories
            .iter()
            .filter_map(|(name, count)| count.as_u64().map(|count| (name, count)))
            .collect::<Vec<_>>();
        labels.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        labels
            .into_iter()
            .take(3)
            .map(|(name, count)| format!("{} {}", tex_escape(name), fmt_int(count)))
            .collect::<Vec<_>>()
            .join("; ")
    } else {
        dimension_labels(row)
    }
}

fn dimension_label(name: &str) -> String {
    match name {
        "Jankurai tool adoption and CI replacement" => "audit CI replacement".into(),
        "Code shape and semantic surface" => "code shape/semantic surface".into(),
        "Context economy and agent instructions" => "context routing".into(),
        "Python containment and polyglot hygiene" => r"Python/\allowbreak{}polyglot hygiene".into(),
        "Proof lanes and test routing" => "proof routing".into(),
        "Security and supply-chain posture" => "security/supply chain".into(),
        "Build speed signals" => "build speed".into(),
        _ => tex_escape(name),
    }
}

fn dimension_labels(row: &Value) -> String {
    let labels = row["weak_dimensions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("name").and_then(Value::as_str))
        .map(dimension_label)
        .collect::<Vec<_>>();
    if labels.is_empty() {
        "none reported".into()
    } else {
        labels.join("; ")
    }
}

fn repo_path(repo: &str) -> String {
    format!(r"\path{{{repo}}}")
}

fn score_cell(score: u64, best_observed: bool) -> String {
    if best_observed {
        format!(r"\JKScoreCellBest{{{score}}}")
    } else {
        format!(r"\JKScoreCell{{{score}}}")
    }
}

fn aggregate_rows(data: &Value) -> Result<Vec<(String, String)>> {
    let rows = rows(data)?;
    let mut scores = rows
        .iter()
        .map(|row| integer(&row["score"]))
        .collect::<Result<Vec<_>>>()?;
    scores.sort_unstable();
    let findings_total = rows
        .iter()
        .map(finding_count)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .sum::<u64>();
    let hard_total = rows
        .iter()
        .map(|row| integer(&row["hard_findings"]))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .sum::<u64>();
    let hard_share = if findings_total == 0 {
        0.0
    } else {
        hard_total as f64 / findings_total as f64 * 100.0
    };
    let average = scores.iter().sum::<u64>() as f64 / scores.len() as f64;
    let upper_middle = scores[scores.len() / 2];
    Ok(vec![
        (
            "Repositories scanned".into(),
            fmt_int(integer(&data["repo_count"])?),
        ),
        (
            "Successful scans".into(),
            fmt_int(integer(&data["successful"])?),
        ),
        ("Failed scans".into(), fmt_int(integer(&data["failed"])?)),
        (
            "Minimum score".into(),
            fmt_int(*scores.first().unwrap_or(&0)),
        ),
        (
            "Maximum score".into(),
            format!("{} (best observed)", fmt_int(*scores.last().unwrap_or(&0))),
        ),
        ("Average score".into(), fmt_float(average)),
        ("Median score (upper middle)".into(), fmt_int(upper_middle)),
        ("Findings total".into(), fmt_int(findings_total)),
        ("Hard findings total".into(), fmt_int(hard_total)),
        (
            "Hard findings share".into(),
            format!("{}\\%", fmt_float(hard_share)),
        ),
    ])
}

fn category_rows(rows: &[Value]) -> Vec<(String, u64)> {
    if rows.iter().all(|row| row.get("top_categories").is_some()) {
        let mut counter = BTreeMap::<String, u64>::new();
        for row in rows {
            for item in row["top_categories"].as_array().into_iter().flatten() {
                if let (Some(name), Some(count)) = (
                    item.get("name").and_then(Value::as_str),
                    item.get("count").and_then(Value::as_u64),
                ) {
                    *counter.entry(name.to_string()).or_default() += count;
                }
            }
        }
        let mut rows = counter.into_iter().collect::<Vec<_>>();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        rows.truncate(6);
        return rows;
    }
    let mut counter = BTreeMap::<String, u64>::new();
    for row in rows {
        for item in row["weak_dimensions"].as_array().into_iter().flatten() {
            if let Some(name) = item.get("name").and_then(Value::as_str) {
                *counter.entry(name.to_string()).or_default() += 1;
            }
        }
    }
    [
        "Jankurai tool adoption and CI replacement",
        "Code shape and semantic surface",
        "Context economy and agent instructions",
        "Python containment and polyglot hygiene",
        "Proof lanes and test routing",
    ]
    .into_iter()
    .map(|name| (name.into(), counter.get(name).copied().unwrap_or(0)))
    .collect()
}

fn render_top_table(rows: &[Value]) -> Result<String> {
    let ranked = ranked_rows(rows);
    let mut out = String::new();
    for line in [
        r"\newcommand{\PublicRepoScoreTopTable}{%",
        r"\begin{table*}[t]",
        r"\caption{Star-rank ordered advisory public-repository sample. Scores are advisory posture signals, not certification results.}",
        r"\label{tab:public-repo-top-scores}",
        r"\centering",
        r"\scriptsize",
        r"\setlength{\tabcolsep}{3pt}",
        r"\begin{tabularx}{\textwidth}{R{0.035\textwidth} L{0.205\textwidth} R{0.060\textwidth} R{0.050\textwidth} R{0.065\textwidth} R{0.070\textwidth} L{0.150\textwidth} Y}",
        r"\toprule",
        r"\JKTableHeader",
        r"\textbf{Rank} & \textbf{Repo} & \textbf{Stars} & \textbf{Score} & \textbf{Issues} & \textbf{Hard/Soft} & \textbf{Severity} & \textbf{Top categories} \\",
        r"\midrule",
        r"\JKDenseRows",
    ] {
        writeln!(out, "{line}")?;
    }
    let best_score = ranked
        .iter()
        .map(|row| integer(&row["score"]).unwrap_or(0))
        .max()
        .unwrap_or(0);
    for row in ranked.iter().take(10) {
        writeln!(
            out,
            "{} & {} & {} & {} & {} & {}/{} & {} & {} \\\\",
            row_rank(row)?,
            repo_path(text(&row["repo"])?),
            stars(row)?,
            score_cell(
                integer(&row["score"])?,
                integer(&row["score"])? == best_score
            ),
            fmt_int(finding_count(row)?),
            fmt_int(integer(&row["hard_findings"])?),
            fmt_int(soft_findings(row)?),
            severity_summary(row),
            top_categories(row)
        )?;
    }
    for line in [
        r"\bottomrule",
        r"\end{tabularx}",
        r"\JKResetRows",
        r"\end{table*}",
        r"}",
    ] {
        writeln!(out, "{line}")?;
    }
    Ok(out.trim_end().into())
}

fn render_aggregate_table(data: &Value) -> Result<String> {
    let mut out = String::new();
    for line in [
        r"\newcommand{\PublicRepoScoreAggregateTable}{%",
        r"\begin{table}[t]",
        r"\caption{Aggregate advisory scan posture.}",
        r"\label{tab:public-repo-aggregate}",
        r"\centering",
        r"\scriptsize",
        r"\setlength{\tabcolsep}{4pt}",
        r"\begin{tabularx}{\columnwidth}{L{0.60\columnwidth} R{0.28\columnwidth}}",
        r"\toprule",
        r"\JKTableHeader",
        r"\textbf{Metric} & \textbf{Value} \\",
        r"\midrule",
        r"\JKDenseRows",
    ] {
        writeln!(out, "{line}")?;
    }
    for (label, value) in aggregate_rows(data)? {
        writeln!(out, "{} & {} \\\\", tex_escape(&label), value)?;
    }
    for line in [
        r"\bottomrule",
        r"\end{tabularx}",
        r"\JKResetRows",
        r"\end{table}",
        r"}",
    ] {
        writeln!(out, "{line}")?;
    }
    Ok(out.trim_end().into())
}

fn render_category_table(rows: &[Value]) -> Result<String> {
    let mut out = String::new();
    for line in [
        r"\newcommand{\PublicRepoWeakDimensionTable}{%",
        r"\begin{table}[t]",
        r"\caption{Most frequent top finding categories in the advisory scan.}",
        r"\label{tab:public-repo-weak-dimensions}",
        r"\centering",
        r"\scriptsize",
        r"\setlength{\tabcolsep}{4pt}",
        r"\begin{tabularx}{\columnwidth}{Y R{0.18\columnwidth}}",
        r"\toprule",
        r"\JKTableHeader",
        r"\textbf{Category} & \textbf{Findings} \\",
        r"\midrule",
        r"\JKDenseRows",
    ] {
        writeln!(out, "{line}")?;
    }
    for (label, count) in category_rows(rows) {
        writeln!(out, "{} & {} \\\\", tex_escape(&label), fmt_int(count))?;
    }
    for line in [
        r"\bottomrule",
        r"\end{tabularx}",
        r"\JKResetRows",
        r"\end{table}",
        r"}",
    ] {
        writeln!(out, "{line}")?;
    }
    Ok(out.trim_end().into())
}

fn render_appendix_table(rows: &[Value]) -> Result<String> {
    let ranked = ranked_rows(rows);
    let mut out = String::new();
    for line in [
        r"\newcommand{\PublicRepoScoreAppendixTable}{%",
        r"\begingroup",
        r"\scriptsize",
        r"\setlength{\tabcolsep}{3pt}",
        r"\setlength{\LTleft}{0pt}",
        r"\setlength{\LTright}{0pt}",
        r"\JKDenseRows",
        r"\begin{longtable}{@{}R{0.035\textwidth} L{0.180\textwidth} R{0.055\textwidth} R{0.045\textwidth} R{0.060\textwidth} R{0.070\textwidth} L{0.145\textwidth} L{0.300\textwidth}@{}}",
        r"\caption{Full 30-repository advisory scoring run.}\label{tab:public-repo-full}\\",
        r"\toprule",
        r"\JKTableHeader",
        r"\textbf{Rank} & \textbf{Repo} & \textbf{Stars} & \textbf{Score} & \textbf{Issues} & \textbf{Hard/Soft} & \textbf{Severity} & \textbf{Top categories} \\",
        r"\midrule",
        r"\endfirsthead",
        r"\toprule",
        r"\JKTableHeader",
        r"\textbf{Rank} & \textbf{Repo} & \textbf{Stars} & \textbf{Score} & \textbf{Issues} & \textbf{Hard/Soft} & \textbf{Severity} & \textbf{Top categories} \\",
        r"\midrule",
        r"\endhead",
    ] {
        writeln!(out, "{line}")?;
    }
    let best_score = ranked
        .iter()
        .map(|row| integer(&row["score"]).unwrap_or(0))
        .max()
        .unwrap_or(0);
    for row in ranked.iter() {
        writeln!(
            out,
            "{} & {} & {} & {} & {} & {}/{} & {} & {} \\\\",
            row_rank(row)?,
            repo_path(text(&row["repo"])?),
            stars(row)?,
            score_cell(
                integer(&row["score"])?,
                integer(&row["score"])? == best_score
            ),
            fmt_int(finding_count(row)?),
            fmt_int(integer(&row["hard_findings"])?),
            fmt_int(soft_findings(row)?),
            severity_summary(row),
            top_categories(row)
        )?;
    }
    for line in [
        r"\bottomrule",
        r"\end{longtable}",
        r"\JKResetRows",
        r"\endgroup",
        r"}",
    ] {
        writeln!(out, "{line}")?;
    }
    Ok(out.trim_end().into())
}

fn plot_label(repo: &str) -> String {
    match repo {
        "jankurai" | "Jankurai" => r"\textbf{Jankurai}".into(),
        _ => format!(r"\texttt{{{}}}", tex_escape(repo)),
    }
}

fn render_score_plot(rows: &[Value]) -> Result<String> {
    let mut plot_rows = rows
        .iter()
        .map(|row| Ok((text(&row["repo"])?.to_string(), integer(&row["score"])?)))
        .collect::<Result<Vec<_>>>()?;
    plot_rows.push(("Jankurai".into(), 100));
    plot_rows.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(a.0.to_ascii_lowercase().cmp(&b.0.to_ascii_lowercase()))
    });

    let mut out = String::new();
    for line in [
        r"\newcommand{\PublicRepoScoreAppendixPlot}{%",
        r"\begin{figure*}[t]",
        r"\centering",
        r"\definecolor{JKPlotRed}{HTML}{B54B4B}",
        r"\definecolor{JKPlotAmber}{HTML}{D8A33A}",
        r"\definecolor{JKPlotGreen}{HTML}{2F8F5B}",
        r"\resizebox{\textwidth}{!}{%",
        r"\begin{tikzpicture}[x=0.36cm,y=0.050cm]",
        r"\fill[JKHeader!55] (-0.85,-31) rectangle (25.25,106);",
        r"\draw[JKLine] (-0.45,0) -- (24.95,0);",
        r"\draw[JKLine] (-0.45,50) -- (24.95,50);",
        r"\draw[JKLine] (-0.45,85) -- (24.95,85);",
        r"\draw[JKLine] (-0.45,100) -- (24.95,100);",
        r"\node[anchor=east,scale=0.72,text=JKNavy] at (-0.55,0) {0};",
        r"\node[anchor=east,scale=0.72,text=JKNavy] at (-0.55,50) {50};",
        r"\node[anchor=east,scale=0.72,text=JKNavy] at (-0.55,85) {85};",
        r"\node[anchor=east,scale=0.72,text=JKNavy] at (-0.55,100) {100};",
        r"\node[anchor=south,scale=0.78,text=JKNavy] at (12.2,103) {Advisory Jankurai score};",
    ] {
        writeln!(out, "{line}")?;
    }
    for (idx, (repo, score)) in plot_rows.iter().enumerate() {
        let x = idx as f64 * 0.78;
        let label = plot_label(repo);
        let score_color = if *score >= 85 {
            "JKPlotGreen".to_string()
        } else {
            format!("JKPlotRed!{}!JKPlotAmber", score * 2)
        };
        writeln!(
            out,
            r"\filldraw[draw=JKNavy!45,fill={}] ({:.2},0) rectangle ({:.2},{});",
            score_color,
            x,
            x + 0.52,
            score
        )?;
        writeln!(
            out,
            r"\node[anchor=south,rotate=90,scale=0.62,text=JKNavy] at ({:.2},{}) {{{}}};",
            x + 0.26,
            score + 1,
            score
        )?;
        writeln!(
            out,
            r"\node[anchor=east,rotate=45,scale=0.52,text=black] at ({:.2},-3) {{{}}};",
            x + 0.40,
            label
        )?;
    }
    for line in [
        r"\end{tikzpicture}%",
        r"}",
        r"\caption{Advisory public-repository score ranking with the Jankurai repository baseline included. Bars are sorted by score; color moves from red through amber to green as scores approach the merge-witness-ready band.}",
        r"\label{fig:public-repo-score-ranking}",
        r"\end{figure*}",
        r"}",
    ] {
        writeln!(out, "{line}")?;
    }
    Ok(out.trim_end().into())
}

fn render(data: &Value, source: &Path, out: &Path) -> Result<String> {
    let source_posix = source.to_string_lossy().replace('\\', "/");
    let out_posix = out.to_string_lossy().replace('\\', "/");
    let source_artifact = data
        .get("source_artifact")
        .and_then(Value::as_str)
        .or_else(|| data.get("run_root").and_then(Value::as_str))
        .unwrap_or(&source_posix);
    let command = format!(
        "cargo run -p jankurai -- paper public-repo-scores --source {source_posix} --out {out_posix}"
    );
    Ok(format!(
        "% Generated by: cargo run -p jankurai -- paper public-repo-scores\n% Source: {source_posix}\n% Command: {command}\n% DO NOT EDIT BY HAND.\n% Source artifact: {}\n% Generated at: {}\n% Jankurai version: {}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n",
        tex_escape(source_artifact),
        text(&data["generated_at"])?,
        text(&data["jankurai_version"])?,
        render_top_table(rows(data)?)?,
        render_aggregate_table(data)?,
        render_category_table(rows(data)?)?,
        render_appendix_table(rows(data)?)?,
        render_score_plot(rows(data)?)?,
    ))
}
