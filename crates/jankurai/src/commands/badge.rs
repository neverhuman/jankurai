use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const DEFAULT_SCORE_JSON: &str = ".jankurai/repo-score.json";
pub const DEFAULT_SCORE_MD: &str = ".jankurai/repo-score.md";
pub const DEFAULT_BADGE_SVG: &str = "agent/jankurai-badge.svg";
pub const DEFAULT_BADGE_JSON: &str = "agent/jankurai-badge.json";
pub const DEFAULT_README: &str = "README.md";
pub const DEFAULT_CONFIG: &str = "agent/badge.toml";

const START_MARKER: &str = "<!-- jankurai-badge:start -->";
const END_MARKER: &str = "<!-- jankurai-badge:end -->";

#[derive(Debug, Clone)]
pub struct BadgeArgs {
    pub repo: PathBuf,
    pub score: String,
    pub out: String,
    pub json_out: Option<String>,
    pub readme: Option<String>,
    pub link: String,
    pub update_readme: bool,
    pub check: bool,
    pub print_markdown: bool,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BadgeMetadata {
    pub schema_version: String,
    pub standard: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auditor_version: Option<String>,
    pub source_report: String,
    pub source_badge_fingerprint: String,
    pub score: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_score: Option<i32>,
    pub decision: String,
    pub passed: bool,
    pub findings: usize,
    pub hard_findings: usize,
    pub soft_findings: usize,
    pub caps: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conformance_level: Option<String>,
    pub badge_svg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readme: Option<String>,
    pub markdown: String,
}

#[derive(Debug, Deserialize, Default)]
struct BadgeConfig {
    enabled: Option<bool>,
    score: Option<String>,
    svg: Option<String>,
    json: Option<String>,
    readme: Option<String>,
    link: Option<String>,
    update_readme: Option<bool>,
    label: Option<String>,
}

#[derive(Debug, Clone)]
struct ScoreInput {
    source_report: String,
    source_badge_fingerprint: String,
    standard_version: Option<String>,
    auditor_version: Option<String>,
    score: i32,
    raw_score: Option<i32>,
    minimum_score: Option<i32>,
    decision: String,
    passed: bool,
    findings: usize,
    hard_findings: usize,
    soft_findings: usize,
    caps: usize,
    conformance_level: Option<String>,
}

/// Called by `jankurai audit` after writing the score report.
/// Only repos with `agent/badge.toml` get badge mutation from audit.
pub fn run_from_config_after_audit(
    repo: &Path,
    just_written_score_json: &str,
    just_written_score_md: &str,
) -> Result<()> {
    let config_path = repo.join(DEFAULT_CONFIG);
    if !config_path.exists() {
        return Ok(());
    }

    let config_text = fs::read_to_string(&config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    let config: BadgeConfig =
        toml::from_str(&config_text).with_context(|| format!("parse {}", config_path.display()))?;

    if config.enabled == Some(false) {
        return Ok(());
    }

    let score = config
        .score
        .unwrap_or_else(|| just_written_score_json.to_string());
    let svg = config.svg.unwrap_or_else(|| DEFAULT_BADGE_SVG.to_string());
    let json = config
        .json
        .unwrap_or_else(|| DEFAULT_BADGE_JSON.to_string());
    let readme = config.readme.unwrap_or_else(|| DEFAULT_README.to_string());
    let link = config
        .link
        .unwrap_or_else(|| just_written_score_md.to_string());
    let update_readme = config.update_readme.unwrap_or(true);
    let label = config.label.unwrap_or_else(|| "jankurai".to_string());

    run(BadgeArgs {
        repo: repo.to_path_buf(),
        score,
        out: svg,
        json_out: Some(json),
        readme: Some(readme),
        link,
        update_readme,
        check: false,
        print_markdown: false,
        label,
    })
}

pub fn run(args: BadgeArgs) -> Result<()> {
    let input = load_score_input(&args.repo, &args.score)?;
    let message = format!("{}/100", input.score);
    let svg = render_badge_svg(&args.label, &message, &input);

    let readme_block = args.readme.as_ref().map(|readme| {
        let image_path = relative_markdown_path(readme, &args.out);
        let link_path = relative_markdown_path(readme, &args.link);
        render_readme_block(&image_path, &link_path, &input)
    });

    let metadata = BadgeMetadata {
        schema_version: "1.0.0".to_string(),
        standard: "jankurai".to_string(),
        standard_version: input.standard_version.clone(),
        auditor_version: input.auditor_version.clone(),
        source_report: input.source_report.clone(),
        source_badge_fingerprint: input.source_badge_fingerprint.clone(),
        score: input.score,
        raw_score: input.raw_score,
        minimum_score: input.minimum_score,
        decision: input.decision.clone(),
        passed: input.passed,
        findings: input.findings,
        hard_findings: input.hard_findings,
        soft_findings: input.soft_findings,
        caps: input.caps,
        conformance_level: input.conformance_level.clone(),
        badge_svg: normalize_path(&args.repo, &args.out),
        readme: args
            .readme
            .as_ref()
            .filter(|_| args.update_readme)
            .map(|p| normalize_path(&args.repo, p)),
        markdown: readme_block.clone().unwrap_or_default(),
    };

    if args.print_markdown {
        if let Some(block) = &readme_block {
            println!("{}", block.trim_end());
        } else {
            let block = render_readme_block(&args.out, &args.link, &input);
            println!("{}", block.trim_end());
        }
    }

    if args.check {
        return check_outputs(
            &args,
            &svg,
            args.json_out.as_ref(),
            &metadata,
            readme_block.as_ref(),
        );
    }

    let badge_path = resolve_path(&args.repo, &args.out);
    write_if_changed(&badge_path, svg.as_bytes())?;

    if let Some(json_out) = args.json_out.as_ref() {
        crate::validation::validate_serializable(
            &args.repo,
            crate::validation::ArtifactSchema::ReadmeBadge,
            &metadata,
        )?;
        let json_text = format!("{}\n", serde_json::to_string_pretty(&metadata)?);
        let json_path = resolve_path(&args.repo, json_out);
        write_if_changed(&json_path, json_text.as_bytes())?;
    }

    if args.update_readme {
        let readme = args
            .readme
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("--update-readme requires --readme PATH"))?;
        let block = readme_block
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("internal error: missing README block"))?;
        let readme_path = resolve_path(&args.repo, readme);
        let existing = fs::read_to_string(&readme_path).unwrap_or_default();
        let updated = upsert_badge_block(&existing, block)?;
        write_if_changed(&readme_path, updated.as_bytes())?;
    }

    Ok(())
}

fn check_outputs(
    args: &BadgeArgs,
    svg: &str,
    json_out: Option<&String>,
    metadata: &BadgeMetadata,
    readme_block: Option<&String>,
) -> Result<()> {
    let mut failures = Vec::new();

    compare_text_file(
        &resolve_path(&args.repo, &args.out),
        svg,
        "badge SVG",
        &mut failures,
    );

    if let Some(json_out) = json_out {
        let json_text = format!("{}\n", serde_json::to_string_pretty(metadata)?);
        compare_text_file(
            &resolve_path(&args.repo, json_out),
            &json_text,
            "badge JSON",
            &mut failures,
        );
    }

    if args.update_readme {
        let readme = args
            .readme
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("--check --update-readme requires --readme PATH"))?;
        let block =
            readme_block.ok_or_else(|| anyhow::anyhow!("internal error: missing README block"))?;
        let readme_path = resolve_path(&args.repo, readme);
        let existing = fs::read_to_string(&readme_path)
            .with_context(|| format!("read {}", readme_path.display()))?;
        let expected = upsert_badge_block(&existing, block)?;
        if existing != expected {
            failures.push(format!(
                "README badge block is stale or missing in {}",
                readme_path.display()
            ));
        }
    }

    if failures.is_empty() {
        println!("jankurai badge is current");
        Ok(())
    } else {
        bail!("jankurai badge drift:\n- {}", failures.join("\n- "))
    }
}

fn compare_text_file(path: &Path, expected: &str, label: &str, failures: &mut Vec<String>) {
    match fs::read_to_string(path) {
        Ok(actual) if actual == expected => {}
        Ok(_) => failures.push(format!("{label} differs: {}", path.display())),
        Err(_) => failures.push(format!("{label} is missing: {}", path.display())),
    }
}

fn load_score_input(repo: &Path, score_path: &str) -> Result<ScoreInput> {
    let abs = resolve_path(repo, score_path);
    let text = fs::read_to_string(&abs).with_context(|| format!("read {}", abs.display()))?;
    let value: Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", abs.display()))?;
    crate::validation::validate_value(repo, crate::validation::ArtifactSchema::RepoScore, &value)
        .with_context(|| format!("validate source report {}", abs.display()))?;

    let score = get_i32(&value, "score")
        .ok_or_else(|| anyhow::anyhow!("{} is missing integer field `score`", abs.display()))?;

    let raw_score = get_i32(&value, "raw_score");

    let policy_minimum = value
        .get("policy")
        .and_then(|p| get_i32(p, "minimum_score"));

    let decision_value = value.get("decision");
    let decision_minimum = decision_value.and_then(|d| get_i32(d, "minimum_score"));
    let minimum_score = decision_minimum.or(policy_minimum);

    let status = decision_value
        .and_then(|d| d.get("status"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            if Some(score) >= minimum_score {
                "pass"
            } else {
                "fail"
            }
        });

    let explicit_passed = decision_value
        .and_then(|d| d.get("passed"))
        .and_then(Value::as_bool);

    let findings = value
        .get("findings")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);

    let hard_findings_from_decision = decision_value
        .and_then(|d| d.get("hard_findings"))
        .and_then(Value::as_u64)
        .map(|n| n as usize);

    let soft_findings_from_decision = decision_value
        .and_then(|d| d.get("soft_findings"))
        .and_then(Value::as_u64)
        .map(|n| n as usize);

    let hard_findings = hard_findings_from_decision.unwrap_or_else(|| count_hard_findings(&value));
    let soft_findings =
        soft_findings_from_decision.unwrap_or_else(|| findings.saturating_sub(hard_findings));

    let passed = explicit_passed.unwrap_or_else(|| {
        let min_ok = minimum_score.map(|m| score >= m).unwrap_or(true);
        min_ok && hard_findings == 0 && status != "fail"
    });
    if value
        .get("dirty_worktree")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        bail!(
            "{} cannot source a public badge from a dirty report",
            abs.display()
        );
    }
    if status == "advisory" || !passed {
        bail!(
            "{} cannot source a public badge from a {} report",
            abs.display(),
            status
        );
    }

    let decision = if status == "advisory" {
        "advisory"
    } else if passed {
        "pass"
    } else {
        "fail"
    }
    .to_string();

    let caps = value
        .get("caps_applied")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);

    let conformance_level = value
        .get("observed_conformance_level")
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("claimed_conformance_level")
                .and_then(Value::as_str)
        })
        .map(str::to_string);

    let standard_version = value
        .get("standard_version")
        .and_then(Value::as_str)
        .map(str::to_string);

    let auditor_version = value
        .get("auditor_version")
        .and_then(Value::as_str)
        .map(str::to_string);

    let source_report = normalize_path(repo, score_path);

    let fingerprint_value = serde_json::json!({
        "standard_version": standard_version,
        "auditor_version": auditor_version,
        "score": score,
        "raw_score": raw_score,
        "minimum_score": minimum_score,
        "decision": decision,
        "passed": passed,
        "findings": findings,
        "hard_findings": hard_findings,
        "soft_findings": soft_findings,
        "caps": caps,
        "conformance_level": conformance_level,
    });

    let source_badge_fingerprint = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&fingerprint_value)?)
    );

    Ok(ScoreInput {
        source_report,
        source_badge_fingerprint,
        standard_version,
        auditor_version,
        score,
        raw_score,
        minimum_score,
        decision,
        passed,
        findings,
        hard_findings,
        soft_findings,
        caps,
        conformance_level,
    })
}

fn get_i32(value: &Value, key: &str) -> Option<i32> {
    value.get(key)?.as_i64().map(|n| n as i32)
}

fn count_hard_findings(value: &Value) -> usize {
    value
        .get("findings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|finding| {
            finding
                .get("severity")
                .and_then(Value::as_str)
                .map(|s| matches!(s, "high" | "critical"))
                .unwrap_or(false)
        })
        .count()
}

fn render_readme_block(image_path: &str, link_path: &str, input: &ScoreInput) -> String {
    let alt = format!("Jankurai score: {}/100", input.score);
    format!("{START_MARKER}\n[![{alt}]({image_path})]({link_path})\n{END_MARKER}\n")
}

fn render_badge_svg(label: &str, message: &str, input: &ScoreInput) -> String {
    let label = escape_xml(label);
    let message = escape_xml(message);
    let title = escape_xml(&format!(
        "Jankurai score: {}/100 ({})",
        input.score, input.decision
    ));

    let icon_width = 24usize;
    let label_width = text_width(&label, 62);
    let message_width = text_width(&message, 88);
    let width = icon_width + label_width + message_width;
    let label_x = icon_width;
    let message_x = icon_width + label_width;
    let label_center = label_x + label_width / 2;
    let message_center = message_x + message_width / 2;

    let color = badge_color(input);

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="20" role="img" aria-label="{title}">
<title>{title}</title>
<linearGradient id="jankurai-badge-shadow" x2="0" y2="100%">
  <stop offset="0" stop-color="#fff" stop-opacity=".16"/>
  <stop offset="1" stop-color="#000" stop-opacity=".16"/>
</linearGradient>
<clipPath id="jankurai-round">
  <rect width="{width}" height="20" rx="3" fill="#fff"/>
</clipPath>
<g clip-path="url(#jankurai-round)">
  <rect width="{icon_width}" height="20" fill="#111827"/>
  <rect x="{label_x}" width="{label_width}" height="20" fill="#374151"/>
  <rect x="{message_x}" width="{message_width}" height="20" fill="{color}"/>
  <rect width="{width}" height="20" fill="url(#jankurai-badge-shadow)"/>
</g>
<g fill="none" stroke="#f9fafb" stroke-width="1.55" stroke-linecap="round" stroke-linejoin="round">
  <path d="M12 4.3v7.2c0 3.1-1.9 4.4-4.5 4.4"/>
  <path d="M16.8 5.2 12.4 10l4.4 4.8"/>
  <path d="M6.7 6.4h7.6"/>
</g>
<g fill="#fff" text-anchor="middle" font-family="Verdana,DejaVu Sans,sans-serif" font-size="11">
  <text x="{label_center}" y="15" fill="#010101" fill-opacity=".28">{label}</text>
  <text x="{label_center}" y="14">{label}</text>
  <text x="{message_center}" y="15" fill="#010101" fill-opacity=".28">{message}</text>
  <text x="{message_center}" y="14">{message}</text>
</g>
</svg>
"##
    )
}

fn text_width(text: &str, minimum: usize) -> usize {
    ((text.chars().count() * 7) + 18).max(minimum)
}

fn badge_color(input: &ScoreInput) -> &'static str {
    match input.score {
        90..=100 => "#4c1",
        85..=89 => "#97ca00",
        70..=84 => "#dfb317",
        50..=69 => "#fe7d37",
        _ => "#e05d44",
    }
}

pub fn upsert_badge_block(existing: &str, block: &str) -> Result<String> {
    let eol = if existing.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let normalized = existing.replace("\r\n", "\n");
    let block = block.trim_end_matches('\n');

    let updated = if let Some(start) = normalized.find(START_MARKER) {
        let after_start = start + START_MARKER.len();
        let Some(end_rel) = normalized[after_start..].find(END_MARKER) else {
            bail!("README contains `{START_MARKER}` without `{END_MARKER}`");
        };

        let end = after_start + end_rel + END_MARKER.len();
        let before = &normalized[..start];
        let after = normalized[end..].trim_start_matches('\n');

        let mut out = String::new();
        out.push_str(before);
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(block);
        out.push('\n');
        if !after.is_empty() {
            out.push('\n');
            out.push_str(after);
        }
        out
    } else {
        let insert_at = readme_insert_index(&normalized);
        let before = &normalized[..insert_at];
        let after = normalized[insert_at..].trim_start_matches('\n');

        let mut out = String::new();
        out.push_str(before);

        if !out.is_empty() && !out.ends_with("\n\n") {
            if out.ends_with('\n') {
                out.push('\n');
            } else {
                out.push_str("\n\n");
            }
        }

        out.push_str(block);
        out.push('\n');

        if !after.is_empty() {
            out.push('\n');
            out.push_str(after);
        }

        out
    };

    Ok(if eol == "\r\n" {
        updated.replace('\n', "\r\n")
    } else {
        updated
    })
}

fn readme_insert_index(text: &str) -> usize {
    let mut index = 0usize;
    let mut saw_title = false;

    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();

        if index == 0 && trimmed.starts_with("# ") {
            index += line.len();
            saw_title = true;
            continue;
        }

        if saw_title && (trimmed.is_empty() || looks_like_top_badge_line(trimmed)) {
            index += line.len();
            continue;
        }

        break;
    }

    if saw_title {
        index
    } else {
        0
    }
}

fn looks_like_top_badge_line(line: &str) -> bool {
    line.starts_with("[![")
        || line.starts_with("![")
        || line.contains("shields.io")
        || (line.contains("<img") && line.to_ascii_lowercase().contains("badge"))
        || line.starts_with("<p align=")
        || line == "</p>"
}

fn relative_markdown_path(readme: &str, target: &str) -> String {
    if looks_like_url(target) || target.starts_with('#') {
        return target.to_string();
    }

    let target_path = Path::new(target);
    if target_path.is_absolute() {
        return path_to_posix(target_path);
    }

    let readme_path = Path::new(readme);
    let base = readme_path.parent().unwrap_or_else(|| Path::new(""));
    let rel = relative_path(base, target_path);
    if rel.is_empty() {
        ".".to_string()
    } else {
        rel
    }
}

fn relative_path(base: &Path, target: &Path) -> String {
    let base_parts = normal_components(base);
    let target_parts = normal_components(target);

    let mut common = 0usize;
    while common < base_parts.len()
        && common < target_parts.len()
        && base_parts[common] == target_parts[common]
    {
        common += 1;
    }

    let mut parts = Vec::new();
    for _ in common..base_parts.len() {
        parts.push("..".to_string());
    }
    for part in target_parts.iter().skip(common) {
        parts.push(part.clone());
    }

    parts.join("/")
}

fn normal_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            Component::ParentDir => Some("..".to_string()),
            Component::CurDir => None,
            _ => None,
        })
        .collect()
}

fn resolve_path(repo: &Path, path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        repo.join(candidate)
    }
}

fn normalize_path(repo: &Path, path: &str) -> String {
    let candidate = PathBuf::from(path);
    let relative = if candidate.is_absolute() {
        candidate
            .strip_prefix(repo)
            .map(Path::to_path_buf)
            .unwrap_or(candidate)
    } else {
        candidate
    };
    path_to_posix(&relative)
}

fn path_to_posix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn looks_like_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://") || value.starts_with("mailto:")
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn write_if_changed(path: &Path, content: &[u8]) -> Result<bool> {
    if fs::read(path).ok().as_deref() == Some(content) {
        return Ok(false);
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
    }

    fs::write(path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score_input(score: i32) -> ScoreInput {
        ScoreInput {
            source_report: String::new(),
            source_badge_fingerprint: String::new(),
            standard_version: None,
            auditor_version: None,
            score,
            raw_score: None,
            minimum_score: None,
            decision: "pass".to_string(),
            passed: true,
            findings: 0,
            hard_findings: 0,
            soft_findings: 0,
            caps: 0,
            conformance_level: None,
        }
    }

    #[test]
    fn relative_paths_work_from_nested_readme() {
        assert_eq!(
            relative_markdown_path("docs/README.md", "agent/jankurai-badge.svg"),
            "../agent/jankurai-badge.svg"
        );
        assert_eq!(
            relative_markdown_path("README.md", "agent/jankurai-badge.svg"),
            "agent/jankurai-badge.svg"
        );
    }

    #[test]
    fn upsert_replaces_existing_block() {
        let first = "# Demo\n\nhello\n";
        let block = format!("{START_MARKER}\nBADGE\n{END_MARKER}\n");
        let updated = upsert_badge_block(first, &block).unwrap();
        assert!(updated.contains("BADGE"));

        let replacement = format!("{START_MARKER}\nNEW\n{END_MARKER}\n");
        let updated = upsert_badge_block(&updated, &replacement).unwrap();
        assert!(updated.contains("NEW"));
        assert!(!updated.contains("BADGE"));
    }

    #[test]
    fn badge_color_fail_is_red() {
        let mut input = score_input(49);
        input.decision = "fail".to_string();
        input.passed = false;
        input.hard_findings = 1;
        assert_eq!(badge_color(&input), "#e05d44");
    }

    #[test]
    fn badge_color_covers_every_integer_score() {
        for score in 90..=100 {
            assert_eq!(badge_color(&score_input(score)), "#4c1", "score {score}");
        }
        for score in 85..=89 {
            assert_eq!(badge_color(&score_input(score)), "#97ca00", "score {score}");
        }
        for score in 70..=84 {
            assert_eq!(badge_color(&score_input(score)), "#dfb317", "score {score}");
        }
        for score in 50..=69 {
            assert_eq!(badge_color(&score_input(score)), "#fe7d37", "score {score}");
        }
        for score in 0..=49 {
            assert_eq!(badge_color(&score_input(score)), "#e05d44", "score {score}");
        }
    }

    #[test]
    fn badge_color_high_pass_is_brightgreen() {
        assert_eq!(badge_color(&score_input(95)), "#4c1");
    }
}
