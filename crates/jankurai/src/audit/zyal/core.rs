use super::schema::{
    parse_yaml_node, validate_body, IssueDraft, SUPPORTED_ZYAL_CONTRACT_VERSION,
    SUPPORTED_ZYAL_RELEASE_TAG, SUPPORTED_ZYAL_RUNTIME_SENTINEL_VERSION,
};
use crate::audit::helpers::AuditContext;
use crate::model::FileInfo;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, Clone, Default)]
pub struct ZyalSummary {
    pub hard_findings: usize,
    pub advisory_signals: usize,
}

#[derive(Debug, Clone)]
pub struct ZyalFinding {
    pub path: String,
    pub line: Option<usize>,
    pub problem: String,
    pub fix: String,
    pub evidence: Vec<String>,
    pub matched_term: Option<String>,
    pub reason: Option<String>,
}

static OPEN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^<<<ZYAL (?P<version>[A-Za-z0-9._-]+):daemon id=(?P<id>[A-Za-z0-9._-]+)>>>$")
        .expect("ZYAL open regex is valid")
});
static CLOSE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^<<<END_ZYAL id=(?P<id>[A-Za-z0-9._-]+)>>>$").expect("ZYAL close regex is valid")
});
static ARM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^ZYAL_ARM RUN_FOREVER id=(?P<id>[A-Za-z0-9._-]+)$")
        .expect("ZYAL arm regex is valid")
});

pub fn summary(ctx: &AuditContext) -> ZyalSummary {
    ZyalSummary {
        hard_findings: findings(ctx).len(),
        advisory_signals: 0,
    }
}

pub fn findings(ctx: &AuditContext) -> Vec<ZyalFinding> {
    let mut out = Vec::new();
    for file in &ctx.all_files {
        out.extend(scan_file(file));
    }
    out
}

fn scan_file(file: &FileInfo) -> Vec<ZyalFinding> {
    if excluded(file) || file.rel_path == "agent/zyal/README.md" {
        return Vec::new();
    }

    let placement = placement_issue(file);
    if file.is_code && placement.is_none() {
        return Vec::new();
    }

    let ext_candidate = is_extension_candidate(file);
    let mut out = Vec::new();
    if let Some(issue) = placement {
        out.push(issue);
    }

    if !is_candidate(file) {
        return out;
    }

    match parse_envelope(&file.text) {
        Ok(envelope) => {
            let root = match parse_yaml_node(&envelope.body) {
                Ok(root) => root,
                Err(error) => {
                    out.push(yaml_parse_issue(file, envelope.body_start_line, error));
                    return out;
                }
            };
            out.extend(
                validate_body(
                    &root,
                    envelope.body_start_line,
                    envelope.id.as_str(),
                    SUPPORTED_ZYAL_CONTRACT_VERSION,
                    SUPPORTED_ZYAL_RELEASE_TAG,
                )
                .into_iter()
                .map(|issue| issue.into_finding(&file.rel_path)),
            );
        }
        Err(issue) => {
            let ignore_non_runbook = !ext_candidate
                && matches!(
                    issue.reason.as_deref(),
                    Some("open sentinel not found")
                        | Some("non-open sentinel")
                        | Some("code fence detected")
                );
            if !ignore_non_runbook {
                out.push(issue.into_finding(&file.rel_path));
            }
        }
    }

    out
}

fn excluded(file: &FileInfo) -> bool {
    let lower = file.rel_path.to_ascii_lowercase();
    lower.starts_with("docs/")
        || lower.starts_with("paper/")
        || lower.starts_with("reference/")
        || lower.starts_with("tips/")
        || lower.starts_with("target/")
        || lower.contains("/fixtures/")
        || lower.starts_with("tests/")
        || lower.contains("/tests/")
        || lower.starts_with("examples/")
        || lower.contains("/examples/")
        || lower.ends_with("_test.rs")
        || lower.ends_with(".test.rs")
        || lower.ends_with(".spec.rs")
        || lower.ends_with(".test.ts")
        || lower.ends_with(".spec.ts")
        || lower.starts_with("crates/jankurai/src/audit/zyal/")
}

fn is_candidate(file: &FileInfo) -> bool {
    let lower = file.rel_path.to_ascii_lowercase();
    lower.ends_with(".zyal")
        || lower.ends_with(".zyal.yml")
        || lower.ends_with(".zyal.yaml")
        || file.text.contains("<<<ZYAL ")
        || file.text.contains("ZYAL_ARM RUN_FOREVER")
}

fn is_extension_candidate(file: &FileInfo) -> bool {
    let lower = file.rel_path.to_ascii_lowercase();
    lower.ends_with(".zyal") || lower.ends_with(".zyal.yml") || lower.ends_with(".zyal.yaml")
}

fn placement_issue(file: &FileInfo) -> Option<ZyalFinding> {
    let lower = file.rel_path.to_ascii_lowercase();
    let under_canonical = lower.starts_with("agent/zyal/");
    let legacy_ext = lower.ends_with(".zyal.yml") || lower.ends_with(".zyal.yaml");
    let is_readme = lower == "agent/zyal/readme.md";
    if is_readme {
        return None;
    }
    if under_canonical {
        if lower.ends_with(".zyal") {
            return None;
        }
        return Some(ZyalFinding {
            path: file.rel_path.clone(),
            line: Some(1),
            problem: "ZYAL runbooks under agent/zyal must use the .zyal extension".into(),
            fix: "rename the file to `*.zyal` and keep it under `agent/zyal/`".into(),
            evidence: vec![
                format!("path={}", file.rel_path),
                format!("supported_contract_version={SUPPORTED_ZYAL_CONTRACT_VERSION}"),
                format!("release_tag={SUPPORTED_ZYAL_RELEASE_TAG}"),
            ],
            matched_term: Some(file.name.clone()),
            reason: Some("canonical ZYAL placement".into()),
        });
    }
    if legacy_ext || lower.ends_with(".zyal") || lower.contains("/zyal/") {
        let mut evidence = vec![
            format!("path={}", file.rel_path),
            format!("supported_contract_version={SUPPORTED_ZYAL_CONTRACT_VERSION}"),
            format!("release_tag={SUPPORTED_ZYAL_RELEASE_TAG}"),
        ];
        if lower.ends_with(".zyal.yml") || lower.ends_with(".zyal.yaml") {
            evidence.push("legacy_extension=.zyal.yml/.zyal.yaml".into());
        }
        if lower.contains("/zyal/") && !under_canonical {
            evidence.push("canonical_root=agent/zyal".into());
        }
        return Some(ZyalFinding {
            path: file.rel_path.clone(),
            line: Some(1),
            problem: if legacy_ext {
                "legacy ZYAL extension is not allowed; use canonical agent/zyal/*.zyal placement"
                    .into()
            } else {
                "ZYAL runbooks must live under agent/zyal as *.zyal files".into()
            },
            fix: "move the runbook to `agent/zyal/` and rename it to `*.zyal`".into(),
            evidence,
            matched_term: if legacy_ext {
                Some(".zyal.yml".into())
            } else {
                Some("agent/zyal".into())
            },
            reason: Some("canonical ZYAL repository root".into()),
        });
    }
    None
}

struct ParsedEnvelope {
    id: String,
    body: String,
    body_start_line: usize,
}

#[allow(clippy::result_large_err)]
fn parse_envelope(text: &str) -> Result<ParsedEnvelope, IssueDraft> {
    let normalized = normalize_text(text);
    if normalized.contains("```") {
        return Err(issue(
            Some(1),
            "ZYAL runbooks cannot be wrapped in code fences",
            "remove code fences and keep the `<<<ZYAL ...>>>` block as raw file content",
            vec![
                format!("supported_contract_version={SUPPORTED_ZYAL_CONTRACT_VERSION}"),
                format!("runtime_sentinel_version={SUPPORTED_ZYAL_RUNTIME_SENTINEL_VERSION}"),
            ],
            Some("```".to_string()),
            Some("code fence detected".into()),
        ));
    }

    let lines: Vec<&str> = normalized.lines().collect();
    let mut first_content = None;
    let mut open_lines = Vec::new();
    let mut close_line = None;
    let mut close_id = None;
    let mut arm_line = None;
    let mut arm_id = None;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if OPEN_RE.is_match(trimmed) {
            open_lines.push(idx + 1);
            if first_content.is_none() {
                first_content = Some(idx + 1);
            }
            continue;
        }
        if let Some(caps) = CLOSE_RE.captures(trimmed) {
            close_line = Some(idx + 1);
            close_id = caps.name("id").map(|m| m.as_str().to_string());
            continue;
        }
        if let Some(caps) = ARM_RE.captures(trimmed) {
            arm_line = Some(idx + 1);
            arm_id = caps.name("id").map(|m| m.as_str().to_string());
            continue;
        }
        if first_content.is_none() {
            first_content = Some(idx + 1);
        }
    }

    if let Some(line_no) = first_content {
        let first = lines[line_no - 1].trim();
        if !OPEN_RE.is_match(first) {
            return Err(issue(
                Some(line_no),
                "first non-comment line is not a ZYAL open sentinel",
                "keep the runbook envelope at the top of the file after optional comments",
                vec![
                    format!("supported_contract_version={SUPPORTED_ZYAL_CONTRACT_VERSION}"),
                    format!("runtime_sentinel_version={SUPPORTED_ZYAL_RUNTIME_SENTINEL_VERSION}"),
                ],
                Some("<<<ZYAL".into()),
                Some("non-open sentinel".into()),
            ));
        }
    }

    if open_lines.is_empty() {
        return Err(issue(
            Some(first_content.unwrap_or(1)),
            "missing ZYAL open sentinel",
            "start the file with `<<<ZYAL v1:daemon id=<id>>>` after optional blank or comment lines",
            vec![
                format!("supported_contract_version={SUPPORTED_ZYAL_CONTRACT_VERSION}"),
                format!("runtime_sentinel_version={SUPPORTED_ZYAL_RUNTIME_SENTINEL_VERSION}"),
            ],
            Some("<<<ZYAL".into()),
            Some("open sentinel not found".into()),
        ));
    }
    if open_lines.len() > 1 {
        return Err(issue(
            Some(open_lines[1]),
            "duplicate ZYAL open sentinel detected",
            "keep exactly one `<<<ZYAL ...>>>` block in the file",
            vec![format!("open_sentinels={}", open_lines.len())],
            Some("<<<ZYAL".into()),
            Some("duplicate open".into()),
        ));
    }

    let open_line = open_lines[0];
    let open_text = lines[open_line - 1].trim();
    let Some(open_caps) = OPEN_RE.captures(open_text) else {
        return Err(issue(
            Some(open_line),
            "ZYAL open sentinel is malformed",
            "use `<<<ZYAL v1:daemon id=<id>>>` exactly",
            vec![format!("line={open_line}")],
            Some("<<<ZYAL".into()),
            Some("malformed open".into()),
        ));
    };
    let runtime_version = open_caps
        .name("version")
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();
    let id = open_caps
        .name("id")
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();

    if runtime_version != SUPPORTED_ZYAL_RUNTIME_SENTINEL_VERSION {
        return Err(issue(
            Some(open_line),
            format!("unsupported ZYAL runtime sentinel version `{runtime_version}`"),
            "keep the runtime sentinel at `v1` until the scanner supports a newer contract",
            vec![
                format!("supported_contract_version={SUPPORTED_ZYAL_CONTRACT_VERSION}"),
                format!(
                    "supported_runtime_sentinel_version={SUPPORTED_ZYAL_RUNTIME_SENTINEL_VERSION}"
                ),
                format!("release_tag={SUPPORTED_ZYAL_RELEASE_TAG}"),
            ],
            Some(runtime_version),
            Some("future runtime version".into()),
        ));
    }

    let Some(close_line) = close_line else {
        return Err(issue(
            Some(open_line),
            "missing ZYAL close sentinel",
            "append `<<<END_ZYAL id=<id>>>` after the YAML body",
            vec![format!("open_id={id}")],
            Some("<<<END_ZYAL".into()),
            Some("close sentinel missing".into()),
        ));
    };
    let Some(close_id) = close_id else {
        return Err(issue(
            Some(close_line),
            "ZYAL close sentinel is malformed",
            "use `<<<END_ZYAL id=<id>>>` exactly",
            vec![format!("line={close_line}")],
            Some("<<<END_ZYAL".into()),
            Some("malformed close".into()),
        ));
    };
    if close_id != id {
        return Err(issue(
            Some(close_line),
            "ZYAL close id does not match the open id",
            "keep the open and close sentinel ids identical",
            vec![format!("open_id={id}"), format!("close_id={close_id}")],
            Some("id".into()),
            Some("mismatched open/close ids".into()),
        ));
    }

    let body_start_line = open_line + 1;
    let body = if close_line > open_line + 1 {
        lines[open_line..close_line - 1]
            .join("\n")
            .trim()
            .to_string()
    } else {
        String::new()
    };
    if body.is_empty() {
        return Err(issue(
            Some(body_start_line),
            "ZYAL body is empty",
            "add the YAML runbook body between the open and close sentinels",
            vec![format!("open_id={id}")],
            Some("version".into()),
            Some("empty body".into()),
        ));
    }

    if let Some(arm_line) = arm_line {
        let arm_text = lines[arm_line - 1].trim();
        let Some(arm_caps) = ARM_RE.captures(arm_text) else {
            return Err(issue(
                Some(arm_line),
                "ZYAL arm sentinel is malformed",
                "use `ZYAL_ARM RUN_FOREVER id=<id>` exactly",
                vec![format!("line={arm_line}")],
                Some("ZYAL_ARM".into()),
                Some("malformed arm".into()),
            ));
        };
        let arm_id = arm_id.unwrap_or_default();
        if arm_id != id {
            return Err(issue(
                Some(arm_line),
                "ZYAL arm id does not match the open id",
                "keep the open, close, and arm ids identical",
                vec![format!("open_id={id}"), format!("arm_id={arm_id}")],
                Some("id".into()),
                Some("mismatched arm id".into()),
            ));
        }
        if arm_line <= close_line {
            return Err(issue(
                Some(arm_line),
                "ZYAL arm sentinel must come after the close sentinel",
                "place `ZYAL_ARM RUN_FOREVER id=<id>` after `<<<END_ZYAL ...>>>`",
                vec![format!("close_line={close_line}")],
                Some("ZYAL_ARM".into()),
                Some("arm before close".into()),
            ));
        }
        if lines[arm_line..].iter().any(|line| !line.trim().is_empty()) {
            return Err(issue(
                Some(arm_line),
                "trailing content after ZYAL_ARM is not allowed",
                "remove any text after the `ZYAL_ARM RUN_FOREVER` sentinel",
                vec![format!("arm_line={arm_line}")],
                Some("ZYAL_ARM".into()),
                Some("trailing content after arm".into()),
            ));
        }
        let _ = arm_caps;
    } else {
        return Err(issue(
            Some(close_line),
            "missing trailing ZYAL_ARM sentinel",
            "append `ZYAL_ARM RUN_FOREVER id=<id>` on the final line",
            vec![format!("open_id={id}")],
            Some("ZYAL_ARM".into()),
            Some("missing arm".into()),
        ));
    }

    let body_lines = lines[open_line..close_line - 1].join("\n");
    Ok(ParsedEnvelope {
        id,
        body: body_lines,
        body_start_line,
    })
}

fn normalize_text(text: &str) -> String {
    let mut out = text
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    out.retain(|ch| ch == '\n' || !is_stray_control(ch));
    out
}

fn is_stray_control(ch: char) -> bool {
    let code = ch as u32;
    (code <= 0x08 || code == 0x0b || code == 0x0c || (0x0e..=0x1f).contains(&code) || code == 0x7f)
        && ch != '\n'
        && ch != '\t'
}

fn yaml_parse_issue(
    file: &FileInfo,
    body_start_line: usize,
    error: serde_yaml::Error,
) -> ZyalFinding {
    let line = error
        .location()
        .map(|loc| body_start_line + loc.line().saturating_sub(1));
    ZyalFinding {
        path: file.rel_path.clone(),
        line,
        problem: "ZYAL YAML body could not be parsed".into(),
        fix: "repair the YAML syntax and keep the body as a single mapping".into(),
        evidence: vec![
            format!("supported_contract_version={SUPPORTED_ZYAL_CONTRACT_VERSION}"),
            format!("release_tag={SUPPORTED_ZYAL_RELEASE_TAG}"),
            error.to_string(),
        ],
        matched_term: Some("yaml".into()),
        reason: Some("parser error".into()),
    }
}

fn issue(
    line: Option<usize>,
    problem: impl Into<String>,
    fix: impl Into<String>,
    evidence: Vec<String>,
    matched_term: Option<String>,
    reason: Option<String>,
) -> IssueDraft {
    IssueDraft {
        line,
        problem: problem.into(),
        fix: fix.into(),
        evidence,
        matched_term,
        reason,
    }
}

trait IntoFinding {
    fn into_finding(self, path: &str) -> ZyalFinding;
}

impl IntoFinding for IssueDraft {
    fn into_finding(self, path: &str) -> ZyalFinding {
        ZyalFinding {
            path: path.to_string(),
            line: self.line,
            problem: self.problem,
            fix: self.fix,
            evidence: self.evidence,
            matched_term: self.matched_term,
            reason: self.reason,
        }
    }
}

impl IntoFinding for ZyalFinding {
    fn into_finding(self, _path: &str) -> ZyalFinding {
        self
    }
}
