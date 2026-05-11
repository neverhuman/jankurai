use crate::model::{Finding, Report};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueFormat {
    Markdown,
    Jsonl,
    Beads,
    Github,
}

impl IssueFormat {
    pub fn parse(value: &str) -> anyhow::Result<Self> {
        Ok(match value {
            "markdown" | "md" => Self::Markdown,
            "jsonl" => Self::Jsonl,
            "beads" => Self::Beads,
            "github" => Self::Github,
            other => anyhow::bail!(
                "unknown issue export format `{other}`; expected markdown, jsonl, beads, or github"
            ),
        })
    }
}

pub fn render_issues(report: &Report, format: IssueFormat) -> String {
    match format {
        IssueFormat::Markdown => markdown(report),
        IssueFormat::Jsonl => jsonl(report),
        IssueFormat::Beads => beads(report),
        IssueFormat::Github => github(report),
    }
}

pub fn repair_queue_jsonl(report: &Report) -> String {
    report
        .agent_fix_queue
        .iter()
        .map(|item| serde_json::to_string(item).unwrap_or_else(|_| "{}".into()))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn markdown(report: &Report) -> String {
    let mut out = String::new();
    use std::fmt::Write;
    let _ = writeln!(out, "# jankurai Issues");
    for finding in &report.findings {
        let _ = writeln!(
            out,
            "\n## {} {}",
            finding
                .rule_id
                .as_deref()
                .unwrap_or("HLT-000-SCORE-DIMENSION"),
            finding.path
        );
        write_finding_body(&mut out, finding);
    }
    out
}

fn jsonl(report: &Report) -> String {
    report
        .findings
        .iter()
        .map(|finding| serde_json::to_string(finding).unwrap_or_else(|_| "{}".into()))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn beads(report: &Report) -> String {
    report
        .findings
        .iter()
        .map(|finding| {
            serde_json::json!({
                "title": format!("{} {}", finding.rule_id.as_deref().unwrap_or("HLT-000"), finding.path),
                "description": finding.problem,
                "acceptance": finding.agent_fix,
                "labels": ["jankurai", finding.severity.as_str(), finding.category.as_str()],
                "fingerprint": finding.fingerprint,
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn github(report: &Report) -> String {
    report
        .findings
        .iter()
        .map(|finding| {
            format!(
                "gh issue create --title {} --body {} --label jankurai --label {}",
                shell_quote(&format!(
                    "{} {}",
                    finding.rule_id.as_deref().unwrap_or("HLT-000"),
                    finding.path
                )),
                shell_quote(&format!(
                    "{}\n\nFix: {}",
                    finding.problem, finding.agent_fix
                )),
                shell_quote(&finding.severity)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn write_finding_body(out: &mut String, finding: &Finding) {
    use std::fmt::Write;
    let _ = writeln!(out, "- Severity: `{}`", finding.severity);
    let _ = writeln!(out, "- Check: `{}`", finding.check_id);
    let _ = writeln!(out, "- Hardness: `{}`", finding.hardness);
    let _ = writeln!(
        out,
        "- Lane: `{}`",
        finding.lane.as_deref().unwrap_or("audit")
    );
    let _ = writeln!(out, "- Rerun: `{}`", finding.rerun_command);
    let _ = writeln!(out, "- Problem: {}", finding.problem);
    let _ = writeln!(out, "- Fix: {}", finding.agent_fix);
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
