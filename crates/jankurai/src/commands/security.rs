use crate::model::STANDARD_VERSION;
use crate::validation::{self, ArtifactSchema};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct SecurityRunArgs {
    pub repo: PathBuf,
    pub script: String,
    pub out: String,
    pub strict: bool,
    pub profile: String,
}

#[derive(Debug, Deserialize, Default)]
struct SecurityPolicyFile {
    #[serde(default)]
    schema_version: String,
    #[serde(default)]
    enabled_tools: Vec<String>,
    #[serde(default)]
    required_tools: Vec<String>,
    #[serde(default)]
    advisory_tools: Vec<String>,
    #[serde(default)]
    severity_thresholds: SecuritySeverityThresholds,
    #[serde(default)]
    profiles: BTreeMap<String, SecurityProfilePolicy>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct SecurityProfilePolicy {
    #[serde(default)]
    enabled_tools: Vec<String>,
    #[serde(default)]
    required_tools: Vec<String>,
    #[serde(default)]
    advisory_tools: Vec<String>,
    #[serde(default)]
    require_one_of: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
struct SecuritySeverityThresholds {
    #[serde(default = "default_fail_lane_on")]
    fail_lane_on: String,
}

fn default_fail_lane_on() -> String {
    "high".into()
}

#[derive(Debug, Deserialize)]
struct ParsedSecurityStep {
    label: String,
    shell_command: String,
    #[serde(default)]
    tool: Option<String>,
    status: String,
    #[serde(default)]
    advisory: bool,
    #[serde(default)]
    exit_code: Option<i32>,
}

#[derive(Debug, Serialize)]
struct SecurityWrapper {
    kind: String,
    path: String,
    strict: bool,
}

#[derive(Debug, Serialize)]
struct SecurityLaneStep {
    label: String,
    shell_command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool: Option<String>,
    status: String,
    required_by_policy: bool,
    blocking: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    advisory: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    finding_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    highest_severity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    normalized_decision: Option<String>,
}

#[derive(Debug, Serialize)]
struct SecurityEvidence {
    schema_version: String,
    standard_version: String,
    generated_at: String,
    repo_root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    git_head: Option<String>,
    lane: String,
    wrapper: SecurityWrapper,
    exit_code: i32,
    elapsed_ms: u64,
    log_path: String,
    policy: SecurityPolicySnapshot,
    commands: Vec<SecurityLaneStep>,
}

#[derive(Debug, Serialize)]
struct SecurityPolicySnapshot {
    schema_version: String,
    profile: String,
    enabled_tools: Vec<String>,
    required_tools: Vec<String>,
    advisory_tools: Vec<String>,
    require_one_of: Vec<Vec<String>>,
    fail_lane_on: String,
}

pub fn run(args: SecurityRunArgs) -> Result<()> {
    let repo = args.repo;
    let script_rel = args.script.replace('\\', "/");
    let script_path = repo.join(&script_rel);
    if !script_path.is_file() {
        anyhow::bail!(
            "security lane script `{}` does not exist in {}",
            script_rel,
            repo.display()
        );
    }

    let security_dir = repo.join("target/jankurai/security");
    fs::create_dir_all(&security_dir)?;
    let policy = load_policy(&repo)?;
    let selected_policy = select_profile_policy(&policy, &args.profile)?;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let log_name = format!(
        "security-lane-{}-{:03}.log",
        ts.as_secs(),
        ts.subsec_millis()
    );
    let log_abs = security_dir.join(&log_name);
    let log_rel = display_rel(&repo, &log_abs);

    let shell_command = format!("bash {}", script_rel);
    let started = Instant::now();

    let mut cmd = Command::new("bash");
    cmd.arg(&script_rel).current_dir(&repo);
    if args.strict {
        cmd.env("JANKURAI_SECURITY_STRICT", "1");
    } else {
        cmd.env_remove("JANKURAI_SECURITY_STRICT");
    }

    let output = cmd
        .output()
        .with_context(|| format!("run `{shell_command}`"))?;

    let exit_code = output.status.code().unwrap_or(-1);
    let mut log_text = String::new();
    log_text.push_str(&format!(
        "command: {shell_command}\nstatus: {:?}\n\n",
        output.status
    ));
    log_text.push_str(&String::from_utf8_lossy(&output.stdout));
    if !output.stdout.is_empty() && !output.stdout.ends_with(b"\n") {
        log_text.push('\n');
    }
    if !output.stderr.is_empty() {
        log_text.push_str("\n[stderr]\n");
        log_text.push_str(&String::from_utf8_lossy(&output.stderr));
        if !output.stderr.ends_with(b"\n") {
            log_text.push('\n');
        }
    }
    fs::write(&log_abs, &log_text)?;

    let stderr_lossy = String::from_utf8_lossy(&output.stderr);
    let stderr_excerpt = if exit_code != 0 && !output.stderr.is_empty() {
        Some(cap_excerpt(&stderr_lossy, 500))
    } else {
        None
    };

    let step_status = if exit_code == 0 { "ran" } else { "failed" };
    let generated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();

    let parsed_commands = parse_script_steps(&log_text);
    let mut commands = if !parsed_commands.is_empty() {
        parsed_commands
            .into_iter()
            .map(|step| enrich_step(step, &selected_policy))
            .collect()
    } else {
        vec![SecurityLaneStep {
            label: "security-lane".to_string(),
            shell_command,
            tool: Some("bash".to_string()),
            status: step_status.to_string(),
            required_by_policy: true,
            blocking: step_status != "ran",
            exit_code: Some(exit_code),
            advisory: false,
            stderr_excerpt,
            finding_count: None,
            highest_severity: None,
            normalized_decision: None,
        }]
    };

    append_missing_required_steps(&mut commands, &selected_policy);
    let blocking_commands = commands
        .iter()
        .filter(|command| command.blocking)
        .map(|command| command.label.clone())
        .collect::<Vec<_>>();
    let effective_exit_code = if exit_code == 0 && !blocking_commands.is_empty() {
        1
    } else {
        exit_code
    };

    let evidence = SecurityEvidence {
        schema_version: "1.0.0".to_string(),
        standard_version: STANDARD_VERSION.to_string(),
        generated_at,
        repo_root: repo.display().to_string(),
        git_head: git_head(&repo),
        lane: "security".to_string(),
        wrapper: SecurityWrapper {
            kind: "bash_script".to_string(),
            path: script_rel.clone(),
            strict: args.strict,
        },
        exit_code: effective_exit_code,
        elapsed_ms: started.elapsed().as_millis() as u64,
        log_path: log_rel,
        policy: SecurityPolicySnapshot {
            schema_version: policy.schema_version,
            profile: args.profile.clone(),
            enabled_tools: selected_policy.enabled_tools,
            required_tools: selected_policy.required_tools,
            advisory_tools: selected_policy.advisory_tools,
            require_one_of: selected_policy.require_one_of,
            fail_lane_on: policy.severity_thresholds.fail_lane_on,
        },
        commands,
    };

    validation::write_json(
        &repo,
        ArtifactSchema::SecurityEvidence,
        &args.out,
        &evidence,
    )?;

    if effective_exit_code != 0 {
        if blocking_commands.is_empty() {
            anyhow::bail!("security lane exited with status {exit_code}");
        }
        anyhow::bail!(
            "security lane blocked by required tool evidence: {}",
            blocking_commands.join(", ")
        );
    }

    Ok(())
}

fn select_profile_policy(
    policy: &SecurityPolicyFile,
    profile: &str,
) -> Result<SecurityProfilePolicy> {
    let mut selected =
        policy
            .profiles
            .get(profile)
            .cloned()
            .unwrap_or_else(|| SecurityProfilePolicy {
                enabled_tools: policy.enabled_tools.clone(),
                required_tools: policy.required_tools.clone(),
                advisory_tools: policy.advisory_tools.clone(),
                require_one_of: vec![],
            });
    selected.enabled_tools = canonicalize_tools(selected.enabled_tools);
    selected.required_tools = canonicalize_tools(selected.required_tools);
    selected.advisory_tools = canonicalize_tools(selected.advisory_tools);
    selected.require_one_of = selected
        .require_one_of
        .into_iter()
        .map(canonicalize_tools)
        .collect();
    if !matches!(profile, "local" | "ci" | "release") {
        anyhow::bail!("unknown security profile `{profile}`; expected local, ci, or release");
    }
    Ok(selected)
}

fn load_policy(repo: &Path) -> Result<SecurityPolicyFile> {
    let path = repo.join("agent/security-policy.toml");
    if !path.is_file() {
        return Ok(SecurityPolicyFile {
            schema_version: "1.0.0".into(),
            enabled_tools: vec![],
            required_tools: vec![],
            advisory_tools: vec![],
            profiles: BTreeMap::new(),
            severity_thresholds: SecuritySeverityThresholds {
                fail_lane_on: default_fail_lane_on(),
            },
        });
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut policy: SecurityPolicyFile =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if policy.schema_version.is_empty() {
        policy.schema_version = "1.0.0".into();
    }
    if policy.severity_thresholds.fail_lane_on.is_empty() {
        policy.severity_thresholds.fail_lane_on = default_fail_lane_on();
    }
    Ok(policy)
}

fn enrich_step(step: ParsedSecurityStep, policy: &SecurityProfilePolicy) -> SecurityLaneStep {
    let tool = step.tool.as_deref().map(canonical_tool_id);
    let required_by_policy = step
        .tool
        .as_deref()
        .map(|tool| {
            let tool = canonical_tool_id(tool);
            policy
                .required_tools
                .iter()
                .any(|candidate| candidate == &tool)
                || (!policy
                    .advisory_tools
                    .iter()
                    .any(|candidate| candidate == &tool)
                    && !step.advisory)
        })
        .unwrap_or(!step.advisory);
    let blocking = required_by_policy && step.status != "ran";
    SecurityLaneStep {
        label: step.label,
        shell_command: step.shell_command,
        tool,
        status: step.status,
        required_by_policy,
        blocking,
        exit_code: step.exit_code,
        advisory: step.advisory,
        stderr_excerpt: None,
        finding_count: None,
        highest_severity: None,
        normalized_decision: None,
    }
}

fn append_missing_required_steps(
    commands: &mut Vec<SecurityLaneStep>,
    policy: &SecurityProfilePolicy,
) {
    let seen = commands
        .iter()
        .filter_map(|command| command.tool.as_deref().map(canonical_tool_id))
        .collect::<BTreeSet<_>>();
    for tool in &policy.required_tools {
        if seen.contains(tool) {
            continue;
        }
        commands.push(SecurityLaneStep {
            label: tool.clone(),
            shell_command: tool.clone(),
            tool: Some(tool.clone()),
            status: "skipped".into(),
            required_by_policy: true,
            blocking: true,
            exit_code: None,
            advisory: false,
            stderr_excerpt: Some("required security tool did not produce evidence".into()),
            finding_count: None,
            highest_severity: None,
            normalized_decision: Some("block".into()),
        });
    }
    for group in &policy.require_one_of {
        if group.iter().any(|tool| seen.contains(tool)) {
            continue;
        }
        let label = group.join("|");
        commands.push(SecurityLaneStep {
            label: label.clone(),
            shell_command: label.clone(),
            tool: Some(label),
            status: "skipped".into(),
            required_by_policy: true,
            blocking: true,
            exit_code: None,
            advisory: false,
            stderr_excerpt: Some("required security tool group did not produce evidence".into()),
            finding_count: None,
            highest_severity: None,
            normalized_decision: Some("block".into()),
        });
    }
}

fn canonicalize_tools(tools: Vec<String>) -> Vec<String> {
    tools
        .into_iter()
        .map(|tool| canonical_tool_id(&tool))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn canonical_tool_id(tool: &str) -> String {
    match tool.trim().to_ascii_lowercase().as_str() {
        "cargo audit" | "cargo-audit" => "cargo-audit".into(),
        "npm audit" | "npm" => "npm".into(),
        "gitleaks" | "gitleaks detect" => "gitleaks".into(),
        "cargo deny" | "cargo-deny" => "cargo-deny".into(),
        other => other.to_string(),
    }
}

fn parse_script_steps(log: &str) -> Vec<ParsedSecurityStep> {
    let mut steps = Vec::new();
    for line in log.lines() {
        let rest = match line.trim_start().strip_prefix("jankurai-security-step=") {
            Some(r) => r,
            None => continue,
        };
        let Ok(p) = serde_json::from_str::<ParsedSecurityStep>(rest) else {
            continue;
        };
        steps.push(ParsedSecurityStep {
            label: p.label,
            shell_command: p.shell_command,
            tool: p.tool,
            status: p.status,
            exit_code: p.exit_code,
            advisory: p.advisory,
        });
    }
    steps
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod parse_tests {
    use super::*;

    #[test]
    fn parse_security_steps_extracts_prefixed_json_lines() {
        let log = r#"
prefix ignored
jankurai-security-step={"label":"gitleaks","tool":"gitleaks","shell_command":"gitleaks detect","status":"ran","advisory":false,"exit_code":0}
jankurai-security-step={"label":"syft","shell_command":"syft .","status":"skipped","advisory":true}
"#;
        let steps = parse_script_steps(log);
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].label, "gitleaks");
        assert_eq!(steps[0].exit_code, Some(0));
        assert!(!steps[0].advisory);
        assert_eq!(steps[1].status, "skipped");
        assert!(steps[1].advisory);
    }
}

fn cap_excerpt(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        t.to_string()
    } else {
        t[t.len().saturating_sub(max)..].to_string()
    }
}

fn git_head(repo: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn display_rel(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
