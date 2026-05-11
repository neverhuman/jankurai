use regex::Regex;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_yaml::Number;
use std::collections::BTreeMap;
use std::fmt;

pub const SUPPORTED_ZYAL_CONTRACT_VERSION: &str = "2.4.0";
pub const SUPPORTED_ZYAL_RELEASE_TAG: &str = "v1.0.0";
pub const SUPPORTED_ZYAL_RUNTIME_SENTINEL_VERSION: &str = "v1";
pub const SUPPORTED_ZYAL_BODY_VERSION: &str = "v1";
pub const SUPPORTED_ZYAL_RESEARCH_VERSION: &str = "v1";

#[derive(Debug, Clone)]
pub(crate) struct IssueDraft {
    pub line: Option<usize>,
    pub problem: String,
    pub fix: String,
    pub evidence: Vec<String>,
    pub matched_term: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum YamlNode {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Sequence(Vec<YamlNode>),
    Mapping(BTreeMap<String, YamlNode>),
}

impl YamlNode {
    pub fn as_map(&self) -> Option<&BTreeMap<String, YamlNode>> {
        match self {
            Self::Mapping(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_seq(&self) -> Option<&[YamlNode]> {
        match self {
            Self::Sequence(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<&Number> {
        match self {
            Self::Number(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_number().and_then(Number::as_i64)
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_number().and_then(Number::as_u64)
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_number().and_then(Number::as_f64)
    }
}

impl<'de> Deserialize<'de> for YamlNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(YamlNodeVisitor)
    }
}

struct YamlNodeVisitor;

impl<'de> Visitor<'de> for YamlNodeVisitor {
    type Value = YamlNode;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a YAML value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(YamlNode::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(YamlNode::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(YamlNode::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        Ok(YamlNode::Number(Number::from(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(YamlNode::String(value.to_string()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(YamlNode::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(YamlNode::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(YamlNode::Null)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element::<YamlNode>()? {
            values.push(value);
        }
        Ok(YamlNode::Sequence(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = BTreeMap::new();
        while let Some((key, value)) = map.next_entry::<String, YamlNode>()? {
            if values.insert(key.clone(), value).is_some() {
                return Err(de::Error::custom(format!("duplicate YAML key `{key}`")));
            }
        }
        Ok(YamlNode::Mapping(values))
    }
}

pub fn parse_yaml_node(text: &str) -> Result<YamlNode, serde_yaml::Error> {
    serde_yaml::from_str::<YamlNode>(text)
}

macro_rules! simple_block {
    ($issues:ident, $map:ident, $name:literal, [$($allowed:literal),* $(,)?]) => {{
        if let Some(node) = $map.get($name) {
            if let Some(block) = node.as_map() {
                $issues.extend(validate_keys($name, block, &[$($allowed),*], &[], None, SUPPORTED_ZYAL_CONTRACT_VERSION, SUPPORTED_ZYAL_RELEASE_TAG));
            } else {
                $issues.push(issue(
                    Some(1),
                    format!("`{}` must be a YAML mapping", $name),
                    format!("rewrite `{}` as a mapping block", $name),
                    vec![format!("block={}", $name)],
                    Some($name.into()),
                    Some("non-mapping block".into()),
                ));
            }
        }
    }};
}

pub fn validate_body(
    root: &YamlNode,
    body_start_line: usize,
    open_id: &str,
    supported_contract_version: &str,
    supported_release_tag: &str,
) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(map) = root.as_map() else {
        issues.push(issue(
            Some(body_start_line),
            "ZYAL body must be a YAML mapping",
            "rewrite the body as a single top-level mapping",
            vec![
                format!("supported_contract_version={supported_contract_version}"),
                format!("release_tag={supported_release_tag}"),
                format!("open_id={open_id}"),
            ],
            Some("mapping".into()),
            Some("non-mapping body".into()),
        ));
        return issues;
    };

    issues.extend(validate_top_level(
        map,
        body_start_line,
        supported_contract_version,
        supported_release_tag,
    ));
    if !issues.is_empty() {
        return issues;
    }

    issues.extend(validate_blocks(map, body_start_line));
    issues
}

fn validate_top_level(
    map: &BTreeMap<String, YamlNode>,
    body_start_line: usize,
    supported_contract_version: &str,
    supported_release_tag: &str,
) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let allowed = [
        "version",
        "intent",
        "confirm",
        "id",
        "job",
        "loop",
        "stop",
        "context",
        "checkpoint",
        "tasks",
        "incubator",
        "agents",
        "mcp",
        "permissions",
        "ui",
        "on",
        "fan_out",
        "guardrails",
        "assertions",
        "retry",
        "hooks",
        "constraints",
        "workflow",
        "memory",
        "evidence",
        "approvals",
        "skills",
        "sandbox",
        "security",
        "observability",
        "arming",
        "capabilities",
        "quality",
        "experiments",
        "models",
        "budgets",
        "triggers",
        "rollback",
        "done",
        "repo_intelligence",
        "fleet",
        "research",
        "taint",
        "interaction",
        "interop",
        "runtime",
        "capability_negotiation",
        "memory_kernel",
        "evidence_graph",
        "trust",
        "requirements",
        "evaluation",
        "release",
        "roles",
        "channels",
        "imports",
        "reasoning_privacy",
        "unsupported_feature_policy",
    ];
    issues.extend(validate_keys(
        "root",
        map,
        &allowed,
        &["version", "intent", "confirm", "job", "stop"],
        Some(body_start_line),
        supported_contract_version,
        supported_release_tag,
    ));

    if let Some(version) = map.get("version").and_then(YamlNode::as_str) {
        if version != SUPPORTED_ZYAL_BODY_VERSION {
            issues.push(issue(
                Some(body_start_line),
                format!("unsupported ZYAL body version `{version}`"),
                "keep `version: v1` for the currently supported ZYAL contract",
                vec![
                    format!("supported_contract_version={supported_contract_version}"),
                    format!("release_tag={supported_release_tag}"),
                    format!("body_version={version}"),
                ],
                Some("version".into()),
                Some("future body version".into()),
            ));
        }
    } else {
        issues.push(issue(
            Some(body_start_line),
            "missing required top-level key `version`",
            "add `version: v1` to the top-level mapping",
            vec![format!(
                "supported_contract_version={supported_contract_version}"
            )],
            Some("version".into()),
            Some("required key".into()),
        ));
    }

    match map.get("intent").and_then(YamlNode::as_str) {
        Some("daemon") => {}
        Some(other) => issues.push(issue(
            Some(body_start_line),
            format!("unsupported intent `{other}`"),
            "set `intent: daemon` for ZYAL runbooks",
            vec![format!("intent={other}")],
            Some("intent".into()),
            Some("unsupported intent".into()),
        )),
        None => issues.push(issue(
            Some(body_start_line),
            "missing required top-level key `intent`",
            "add `intent: daemon` to the top-level mapping",
            vec![format!(
                "supported_contract_version={supported_contract_version}"
            )],
            Some("intent".into()),
            Some("required key".into()),
        )),
    }

    match map.get("confirm").and_then(YamlNode::as_str) {
        Some("RUN_FOREVER") => {}
        Some(other) => issues.push(issue(
            Some(body_start_line),
            format!("unsupported confirm value `{other}`"),
            "set `confirm: RUN_FOREVER` for ZYAL runbooks",
            vec![format!("confirm={other}")],
            Some("confirm".into()),
            Some("unsupported confirm".into()),
        )),
        None => issues.push(issue(
            Some(body_start_line),
            "missing required top-level key `confirm`",
            "add `confirm: RUN_FOREVER` to the top-level mapping",
            vec![format!(
                "supported_contract_version={supported_contract_version}"
            )],
            Some("confirm".into()),
            Some("required key".into()),
        )),
    }

    for required in ["job", "stop"] {
        if !map.contains_key(required) {
            issues.push(issue(
                Some(body_start_line),
                format!("missing required top-level key `{required}`"),
                format!("add `{required}` to the top-level mapping"),
                vec![format!(
                    "supported_contract_version={supported_contract_version}"
                )],
                Some(required.into()),
                Some("required key".into()),
            ));
        }
    }

    issues
}

fn validate_blocks(map: &BTreeMap<String, YamlNode>, body_start_line: usize) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    simple_block!(issues, map, "job", ["name", "objective", "risk"]);
    simple_block!(
        issues,
        map,
        "loop",
        [
            "policy",
            "sleep",
            "continue_on",
            "pause_on",
            "circuit_breaker"
        ]
    );
    simple_block!(
        issues,
        map,
        "context",
        ["strategy", "compact_every", "hard_clear_every", "preserve"]
    );
    simple_block!(
        issues,
        map,
        "checkpoint",
        ["when", "noop_if_clean", "verify", "git"]
    );
    simple_block!(issues, map, "tasks", ["ledger", "discover"]);
    simple_block!(issues, map, "agents", ["supervisor", "workers"]);
    simple_block!(issues, map, "mcp", ["profiles"]);
    simple_block!(
        issues,
        map,
        "permissions",
        [
            "read",
            "list",
            "glob",
            "grep",
            "external_directory",
            "shell",
            "edit",
            "git_commit",
            "git_push",
            "workers",
            "mcp",
            "research",
            "websearch",
            "webfetch",
        ]
    );
    simple_block!(issues, map, "ui", ["theme", "banner"]);
    simple_block!(
        issues,
        map,
        "fan_out",
        [
            "strategy",
            "split",
            "worker",
            "reduce",
            "on_partial_failure"
        ]
    );
    simple_block!(issues, map, "guardrails", ["input", "output", "iteration"]);
    simple_block!(
        issues,
        map,
        "assertions",
        [
            "require_structured_output",
            "schema",
            "on_invalid",
            "max_retries"
        ]
    );
    simple_block!(issues, map, "retry", ["default", "overrides"]);
    simple_block!(
        issues,
        map,
        "hooks",
        [
            "on_start",
            "before_iteration",
            "after_iteration",
            "before_checkpoint",
            "after_checkpoint",
            "on_promote",
            "on_exhaust",
            "on_stop",
        ]
    );
    simple_block!(
        issues,
        map,
        "constraints",
        ["name", "check", "baseline", "invariant", "on_violation"]
    );
    simple_block!(
        issues,
        map,
        "workflow",
        ["type", "initial", "states", "on_stuck", "max_total_time"]
    );
    simple_block!(issues, map, "memory", ["stores", "redaction", "provenance"]);
    simple_block!(
        issues,
        map,
        "evidence",
        ["require_before_promote", "bundle_format", "sign", "archive"]
    );
    simple_block!(issues, map, "approvals", ["gates", "escalation"]);
    simple_block!(
        issues,
        map,
        "skills",
        ["registry", "allow_creation", "max_skills"]
    );
    simple_block!(
        issues,
        map,
        "sandbox",
        ["paths", "network", "resources", "env_inherit", "env_deny"]
    );
    simple_block!(
        issues,
        map,
        "security",
        ["trust_zones", "injection", "secrets"]
    );
    simple_block!(
        issues,
        map,
        "observability",
        ["spans", "metrics", "cost", "report"]
    );
    simple_block!(
        issues,
        map,
        "arming",
        [
            "preview_hash_required",
            "host_nonce_required",
            "reject_inside_code_fence",
            "reject_from",
            "accepted_origins",
            "preview_expires_after",
            "arm_token_single_use",
            "bound_to",
        ]
    );
    simple_block!(
        issues,
        map,
        "capabilities",
        ["default", "rules", "command_floor"]
    );
    simple_block!(
        issues,
        map,
        "quality",
        ["anti_vibe", "diff_budget", "checks"]
    );
    simple_block!(
        issues,
        map,
        "experiments",
        [
            "strategy",
            "diversity",
            "lanes",
            "fork_from",
            "max_parallel",
            "scoring",
            "reduce",
            "on_partial_failure",
            "preserve_failed_lanes_as_negative_memory",
        ]
    );
    simple_block!(
        issues,
        map,
        "models",
        ["profiles", "routes", "critic", "fallback", "confidence_cap"]
    );
    simple_block!(
        issues,
        map,
        "budgets",
        ["run", "task", "iteration", "experiment_lane"]
    );
    simple_block!(issues, map, "triggers", ["list", "anti_recursion"]);
    simple_block!(
        issues,
        map,
        "rollback",
        [
            "required_when",
            "plan_required",
            "verify_command",
            "on_failure_after_merge"
        ]
    );
    simple_block!(issues, map, "done", ["require", "forbid"]);
    simple_block!(
        issues,
        map,
        "repo_intelligence",
        [
            "scale",
            "indexes",
            "generated_zones",
            "scope_control",
            "blast_radius"
        ]
    );
    simple_block!(
        issues,
        map,
        "fleet",
        ["max_workers", "isolation", "jnoccio", "telemetry"]
    );
    simple_block!(
        issues,
        map,
        "research",
        [
            "version",
            "mode",
            "autonomy",
            "max_parallel",
            "timeout_seconds",
            "provider_policy",
            "extraction",
            "evidence",
            "safety",
            "budgets"
        ]
    );
    simple_block!(
        issues,
        map,
        "taint",
        ["default_label", "labels", "forbid", "prompt_injection"]
    );
    simple_block!(issues, map, "interaction", ["mode", "policy", "notes"]);
    simple_block!(
        issues,
        map,
        "interop",
        ["protocols", "adapters", "compile_to", "notes"]
    );
    simple_block!(
        issues,
        map,
        "runtime",
        ["mode", "image", "workspace", "network", "env", "resources"]
    );
    simple_block!(
        issues,
        map,
        "capability_negotiation",
        ["host", "required", "optional", "fail_closed", "degrade_to"]
    );
    simple_block!(
        issues,
        map,
        "memory_kernel",
        ["stores", "redaction", "provenance"]
    );
    simple_block!(
        issues,
        map,
        "evidence_graph",
        ["nodes", "edges", "merge_witness"]
    );
    simple_block!(issues, map, "trust", ["zones", "on_taint", "notes"]);
    simple_block!(issues, map, "requirements", ["must", "should", "avoid"]);
    simple_block!(issues, map, "evaluation", ["metrics", "compare"]);
    simple_block!(
        issues,
        map,
        "release",
        ["channel", "version", "gates", "notes"]
    );
    simple_block!(issues, map, "roles", ["list"]);
    simple_block!(issues, map, "channels", ["list"]);
    simple_block!(issues, map, "imports", ["list"]);
    simple_block!(
        issues,
        map,
        "reasoning_privacy",
        [
            "store_reasoning",
            "redact_chain_of_thought",
            "summaries_only"
        ]
    );
    simple_block!(
        issues,
        map,
        "unsupported_feature_policy",
        ["required", "optional", "fail_closed", "on_missing"]
    );

    issues.extend(validate_stop(map, body_start_line));
    issues.extend(validate_checkpoint(map));
    issues.extend(validate_agents(map));
    issues.extend(validate_mcp(map));
    issues.extend(validate_fan_out(map));
    issues.extend(validate_guardrails(map));
    issues.extend(validate_hooks(map));
    issues.extend(validate_constraints(map));
    issues.extend(validate_incubator(map));
    issues.extend(validate_workflow(map));
    issues.extend(validate_evidence(map));
    issues.extend(validate_approvals(map));
    issues.extend(validate_skills(map));
    issues.extend(validate_sandbox(map));
    issues.extend(validate_security(map));
    issues.extend(validate_observability(map));
    issues.extend(validate_capabilities(map));
    issues.extend(validate_quality(map));
    issues.extend(validate_experiments(map));
    issues.extend(validate_models(map));
    issues.extend(validate_budgets(map));
    issues.extend(validate_triggers(map));
    issues.extend(validate_rollback(map));
    issues.extend(validate_done(map));
    issues.extend(validate_repo_intelligence(map));
    issues.extend(validate_fleet(map));
    issues.extend(validate_research(map));
    issues.extend(validate_taint(map));
    issues.extend(validate_interop(map));
    issues.extend(validate_runtime(map));
    issues.extend(validate_capability_negotiation(map));
    issues.extend(validate_memory_kernel(map));
    issues.extend(validate_evidence_graph(map));
    issues.extend(validate_trust(map));
    issues.extend(validate_requirements(map));
    issues.extend(validate_evaluation(map));
    issues.extend(validate_release(map));
    issues.extend(validate_roles(map));
    issues.extend(validate_channels(map));
    issues.extend(validate_imports(map));
    issues.extend(validate_reasoning_privacy(map));
    issues.extend(validate_unsupported_feature_policy(map));

    issues
}

fn validate_keys(
    path: &str,
    map: &BTreeMap<String, YamlNode>,
    allowed: &[&str],
    required: &[&str],
    line: Option<usize>,
    supported_contract_version: &str,
    supported_release_tag: &str,
) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    if let Some(key) = map.keys().find(|key| !allowed.contains(&key.as_str())) {
        issues.push(issue(
            line,
            format!("Unknown ZYAL key: {path}.{key}"),
            "remove the unknown key or move the data into a supported block",
            vec![
                format!("path={path}"),
                format!("key={key}"),
                format!("supported_contract_version={supported_contract_version}"),
                format!("release_tag={supported_release_tag}"),
            ],
            Some(key.clone()),
            Some("unknown key".into()),
        ));
    }
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|key| !map.contains_key(*key))
        .collect();
    if !missing.is_empty() {
        issues.push(issue(
            line,
            format!(
                "missing required key(s) in `{path}`: {}",
                missing.join(", ")
            ),
            format!("add the required keys to `{path}`"),
            vec![
                format!("path={path}"),
                format!("missing={}", missing.join(",")),
                format!("supported_contract_version={supported_contract_version}"),
            ],
            Some(path.into()),
            Some("missing key".into()),
        ));
    }
    issues
}

fn validate_stop(map: &BTreeMap<String, YamlNode>, body_start_line: usize) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(stop) = map.get("stop").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "stop",
        stop,
        &["all", "any"],
        &[],
        Some(body_start_line),
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    for mode in ["all", "any"] {
        let Some(value) = stop.get(mode) else {
            continue;
        };
        let Some(seq) = value.as_seq() else {
            issues.push(issue(
                Some(body_start_line),
                format!("stop.{mode} must be a list"),
                "rewrite the stop conditions as a YAML list",
                vec![format!("path=stop.{mode}")],
                Some(mode.into()),
                Some("non-list stop condition".into()),
            ));
            continue;
        };
        for (index, item) in seq.iter().enumerate() {
            let Some(record) = item.as_map() else {
                issues.push(issue(
                    Some(body_start_line),
                    format!("stop.{mode}[{index}] must be a YAML mapping"),
                    "rewrite the stop condition as a mapping",
                    vec![format!("path=stop.{mode}[{index}]")],
                    Some(mode.into()),
                    Some("non-mapping stop condition".into()),
                ));
                continue;
            };
            let has_shell = record.get("shell").is_some();
            let has_git_clean = record.get("git_clean").is_some();
            if has_shell == has_git_clean {
                issues.push(issue(
                    Some(body_start_line),
                    format!("stop.{mode}[{index}] must contain exactly one of shell or git_clean"),
                    "choose either `shell` or `git_clean` for each stop condition",
                    vec![format!("path=stop.{mode}[{index}]")],
                    Some("stop".into()),
                    Some("invalid stop condition".into()),
                ));
            }
            if let Some(shell) = record.get("shell").and_then(YamlNode::as_map) {
                issues.extend(validate_keys(
                    &format!("stop.{mode}[{index}].shell"),
                    shell,
                    &["command", "timeout", "cwd", "assert"],
                    &["command"],
                    Some(body_start_line),
                    SUPPORTED_ZYAL_CONTRACT_VERSION,
                    SUPPORTED_ZYAL_RELEASE_TAG,
                ));
            }
            if let Some(git_clean) = record.get("git_clean").and_then(YamlNode::as_map) {
                issues.extend(validate_keys(
                    &format!("stop.{mode}[{index}].git_clean"),
                    git_clean,
                    &["allow_untracked"],
                    &[],
                    Some(body_start_line),
                    SUPPORTED_ZYAL_CONTRACT_VERSION,
                    SUPPORTED_ZYAL_RELEASE_TAG,
                ));
            }
        }
    }
    issues
}

fn validate_checkpoint(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(checkpoint) = map.get("checkpoint").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "checkpoint",
        checkpoint,
        &["when", "noop_if_clean", "verify", "git"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    if let Some(verify) = checkpoint.get("verify").and_then(YamlNode::as_seq) {
        for (index, item) in verify.iter().enumerate() {
            if let Some(record) = item.as_map() {
                issues.extend(validate_keys(
                    &format!("checkpoint.verify[{index}]"),
                    record,
                    &["command", "timeout", "cwd", "assert"],
                    &["command"],
                    None,
                    SUPPORTED_ZYAL_CONTRACT_VERSION,
                    SUPPORTED_ZYAL_RELEASE_TAG,
                ));
            }
        }
    }
    if let Some(git) = checkpoint.get("git").and_then(YamlNode::as_map) {
        issues.extend(validate_keys(
            "checkpoint.git",
            git,
            &["add", "commit_message", "push"],
            &[],
            None,
            SUPPORTED_ZYAL_CONTRACT_VERSION,
            SUPPORTED_ZYAL_RELEASE_TAG,
        ));
    }
    issues
}

fn validate_agents(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(agents) = map.get("agents").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "agents",
        agents,
        &["supervisor", "workers"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    if let Some(supervisor) = agents.get("supervisor").and_then(YamlNode::as_map) {
        issues.extend(validate_keys(
            "agents.supervisor",
            supervisor,
            &["agent"],
            &[],
            None,
            SUPPORTED_ZYAL_CONTRACT_VERSION,
            SUPPORTED_ZYAL_RELEASE_TAG,
        ));
    }
    if let Some(workers) = agents.get("workers").and_then(YamlNode::as_seq) {
        for (index, worker) in workers.iter().enumerate() {
            if let Some(record) = worker.as_map() {
                issues.extend(validate_keys(
                    &format!("agents.workers[{index}]"),
                    record,
                    &["id", "count", "agent", "isolation"],
                    &["id", "agent"],
                    None,
                    SUPPORTED_ZYAL_CONTRACT_VERSION,
                    SUPPORTED_ZYAL_RELEASE_TAG,
                ));
            }
        }
    }
    issues
}

fn validate_mcp(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(mcp) = map.get("mcp").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "mcp",
        mcp,
        &["profiles"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    if let Some(profiles) = mcp.get("profiles").and_then(YamlNode::as_map) {
        for (profile_name, profile) in profiles {
            if let Some(record) = profile.as_map() {
                issues.extend(validate_keys(
                    &format!("mcp.profiles.{profile_name}"),
                    record,
                    &["servers", "tools", "resources"],
                    &[],
                    None,
                    SUPPORTED_ZYAL_CONTRACT_VERSION,
                    SUPPORTED_ZYAL_RELEASE_TAG,
                ));
            }
        }
    }
    issues
}

fn validate_fan_out(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(fo) = map.get("fan_out").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "fan_out",
        fo,
        &[
            "strategy",
            "split",
            "worker",
            "reduce",
            "on_partial_failure",
        ],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    if let Some(split) = fo.get("split").and_then(YamlNode::as_map) {
        issues.extend(validate_keys(
            "fan_out.split",
            split,
            &["shell", "items"],
            &[],
            None,
            SUPPORTED_ZYAL_CONTRACT_VERSION,
            SUPPORTED_ZYAL_RELEASE_TAG,
        ));
    }
    if let Some(worker) = fo.get("worker").and_then(YamlNode::as_map) {
        issues.extend(validate_keys(
            "fan_out.worker",
            worker,
            &["agent", "isolation", "timeout", "max_parallel"],
            &["agent"],
            None,
            SUPPORTED_ZYAL_CONTRACT_VERSION,
            SUPPORTED_ZYAL_RELEASE_TAG,
        ));
    }
    if let Some(reduce) = fo.get("reduce").and_then(YamlNode::as_map) {
        issues.extend(validate_keys(
            "fan_out.reduce",
            reduce,
            &["strategy", "score_key", "command"],
            &["strategy"],
            None,
            SUPPORTED_ZYAL_CONTRACT_VERSION,
            SUPPORTED_ZYAL_RELEASE_TAG,
        ));
    }
    issues
}

fn validate_guardrails(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(gr) = map.get("guardrails").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "guardrails",
        gr,
        &["input", "output", "iteration"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_hooks(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(hooks) = map.get("hooks").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "hooks",
        hooks,
        &[
            "on_start",
            "before_iteration",
            "after_iteration",
            "before_checkpoint",
            "after_checkpoint",
            "on_promote",
            "on_exhaust",
            "on_stop",
        ],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_constraints(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(constraints) = map.get("constraints").and_then(YamlNode::as_seq) else {
        return issues;
    };
    let mut names = std::collections::BTreeSet::new();
    for (index, item) in constraints.iter().enumerate() {
        if let Some(record) = item.as_map() {
            issues.extend(validate_keys(
                &format!("constraints[{index}]"),
                record,
                &["name", "check", "baseline", "invariant", "on_violation"],
                &["name", "check", "invariant"],
                None,
                SUPPORTED_ZYAL_CONTRACT_VERSION,
                SUPPORTED_ZYAL_RELEASE_TAG,
            ));
            if let Some(name) = record.get("name").and_then(YamlNode::as_str) {
                if !names.insert(name.to_string()) {
                    issues.push(issue(
                        None,
                        format!("constraints[{index}].name `{name}` is duplicated"),
                        "keep each constraint name unique",
                        vec![format!("name={name}")],
                        Some(name.into()),
                        Some("duplicate name".into()),
                    ));
                }
            }
        }
    }
    issues
}

fn validate_incubator(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(root) = map.get("incubator").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "incubator",
        root,
        &[
            "enabled",
            "strategy",
            "route_when",
            "exclude_when",
            "budget",
            "scratch",
            "cleanup",
            "readiness",
            "passes",
            "promotion",
        ],
        &["budget", "promotion", "passes"],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    if let Some(budget) = root.get("budget").and_then(YamlNode::as_map) {
        issues.extend(validate_keys(
            "incubator.budget",
            budget,
            &[
                "max_passes_per_task",
                "max_rounds_per_task",
                "max_active_tasks",
                "max_parallel_idea_passes",
            ],
            &["max_passes_per_task", "max_rounds_per_task"],
            None,
            SUPPORTED_ZYAL_CONTRACT_VERSION,
            SUPPORTED_ZYAL_RELEASE_TAG,
        ));
        for key in [
            "max_passes_per_task",
            "max_rounds_per_task",
            "max_active_tasks",
            "max_parallel_idea_passes",
        ] {
            if let Some(value) = budget.get(key) {
                issues.extend(require_positive_integer(
                    value,
                    &format!("incubator.budget.{key}"),
                ));
            }
        }
    }
    if let Some(readiness) = root.get("readiness").and_then(YamlNode::as_map) {
        issues.extend(validate_keys(
            "incubator.readiness",
            readiness,
            &[
                "promote_at",
                "tests_identified_gte",
                "scope_bounded_gte",
                "plan_reviewed_gte",
                "prototype_validated_gte",
                "rollback_known_gte",
                "affected_files_known_gte",
                "critical_objections_resolved_gte",
                "model_confidence_cap",
            ],
            &[],
            None,
            SUPPORTED_ZYAL_CONTRACT_VERSION,
            SUPPORTED_ZYAL_RELEASE_TAG,
        ));
        for key in ["promote_at", "model_confidence_cap"] {
            if let Some(value) = readiness.get(key) {
                issues.extend(require_score(value, &format!("incubator.readiness.{key}")));
            }
        }
    }
    if let Some(promotion) = root.get("promotion").and_then(YamlNode::as_map) {
        issues.extend(validate_keys(
            "incubator.promotion",
            promotion,
            &[
                "promote_at",
                "require",
                "block_on",
                "on_promote",
                "on_exhausted",
            ],
            &["promote_at"],
            None,
            SUPPORTED_ZYAL_CONTRACT_VERSION,
            SUPPORTED_ZYAL_RELEASE_TAG,
        ));
        if let Some(promote_at) = promotion.get("promote_at") {
            issues.extend(require_score(promote_at, "incubator.promotion.promote_at"));
        }
    }
    issues
}

fn validate_workflow(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(workflow) = map.get("workflow").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "workflow",
        workflow,
        &["type", "initial", "states", "on_stuck", "max_total_time"],
        &["type", "initial", "states"],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_evidence(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(evidence) = map.get("evidence").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "evidence",
        evidence,
        &["require_before_promote", "bundle_format", "sign", "archive"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_approvals(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(approvals) = map.get("approvals").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "approvals",
        approvals,
        &["gates", "escalation"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_skills(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(skills) = map.get("skills").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "skills",
        skills,
        &["registry", "allow_creation", "max_skills"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    if let Some(max_skills) = skills.get("max_skills") {
        issues.extend(require_positive_integer(max_skills, "skills.max_skills"));
    }
    issues
}

fn validate_sandbox(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(sandbox) = map.get("sandbox").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "sandbox",
        sandbox,
        &["paths", "network", "resources", "env_inherit", "env_deny"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_security(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(security) = map.get("security").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "security",
        security,
        &["trust_zones", "injection", "secrets"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_observability(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(obs) = map.get("observability").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "observability",
        obs,
        &["spans", "metrics", "cost", "report"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_capabilities(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(capabilities) = map.get("capabilities").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "capabilities",
        capabilities,
        &["default", "rules", "command_floor"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    let mut ids = std::collections::BTreeSet::new();
    if let Some(rules) = capabilities.get("rules").and_then(YamlNode::as_seq) {
        for (index, rule) in rules.iter().enumerate() {
            if let Some(record) = rule.as_map() {
                issues.extend(validate_keys(
                    &format!("capabilities.rules[{index}]"),
                    record,
                    &[
                        "id",
                        "tool",
                        "mcp_profile",
                        "paths",
                        "command_regex",
                        "decision",
                        "require_gate",
                        "expires",
                        "reason",
                    ],
                    &["id"],
                    None,
                    SUPPORTED_ZYAL_CONTRACT_VERSION,
                    SUPPORTED_ZYAL_RELEASE_TAG,
                ));
                if let Some(id) = record.get("id").and_then(YamlNode::as_str) {
                    if !ids.insert(id.to_string()) {
                        issues.push(issue(
                            None,
                            format!("capabilities.rules[{index}].id `{id}` is duplicated"),
                            "keep capability rule ids unique",
                            vec![format!("id={id}")],
                            Some(id.into()),
                            Some("duplicate capability id".into()),
                        ));
                    }
                }
                if let Some(regex) = record.get("command_regex").and_then(YamlNode::as_str) {
                    if Regex::new(regex).is_err() {
                        issues.push(issue(
                            None,
                            format!(
                                "capabilities.rules[{index}].command_regex is not a valid regex"
                            ),
                            "fix the regex pattern or remove the capability rule",
                            vec![format!("regex={regex}")],
                            Some("command_regex".into()),
                            Some("invalid regex".into()),
                        ));
                    }
                }
            }
        }
    }
    issues
}

fn validate_quality(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(quality) = map.get("quality").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "quality",
        quality,
        &["anti_vibe", "diff_budget", "checks"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    let mut names = std::collections::BTreeSet::new();
    if let Some(checks) = quality.get("checks").and_then(YamlNode::as_seq) {
        for (index, check) in checks.iter().enumerate() {
            if let Some(record) = check.as_map() {
                issues.extend(validate_keys(
                    &format!("quality.checks[{index}]"),
                    record,
                    &["name", "pattern", "shell", "scope", "on_violation"],
                    &["name"],
                    None,
                    SUPPORTED_ZYAL_CONTRACT_VERSION,
                    SUPPORTED_ZYAL_RELEASE_TAG,
                ));
                if let Some(name) = record.get("name").and_then(YamlNode::as_str) {
                    if !names.insert(name.to_string()) {
                        issues.push(issue(
                            None,
                            format!("quality.checks[{index}].name `{name}` is duplicated"),
                            "keep quality check names unique",
                            vec![format!("name={name}")],
                            Some(name.into()),
                            Some("duplicate quality check".into()),
                        ));
                    }
                }
                if let Some(pattern) = record.get("pattern").and_then(YamlNode::as_str) {
                    if Regex::new(pattern).is_err() {
                        issues.push(issue(
                            None,
                            format!("quality.checks[{index}].pattern is not a valid regex"),
                            "fix the regex pattern or remove the quality check",
                            vec![format!("pattern={pattern}")],
                            Some("pattern".into()),
                            Some("invalid regex".into()),
                        ));
                    }
                }
            }
        }
    }
    issues
}

fn validate_experiments(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(experiments) = map.get("experiments").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "experiments",
        experiments,
        &[
            "strategy",
            "diversity",
            "lanes",
            "fork_from",
            "max_parallel",
            "scoring",
            "reduce",
            "on_partial_failure",
            "preserve_failed_lanes_as_negative_memory",
        ],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    if let Some(max_parallel) = experiments.get("max_parallel") {
        issues.extend(require_positive_integer(
            max_parallel,
            "experiments.max_parallel",
        ));
    }
    issues
}

fn validate_models(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(models) = map.get("models").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "models",
        models,
        &["profiles", "routes", "critic", "allback", "confidence_cap"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    if let Some(routes) = models.get("routes").and_then(YamlNode::as_map) {
        let profile_names: std::collections::BTreeSet<String> = models
            .get("profiles")
            .and_then(YamlNode::as_map)
            .map(|profiles| profiles.keys().cloned().collect())
            .unwrap_or_default();
        for (route, profile) in routes {
            if let Some(name) = profile.as_str() {
                if !profile_names.contains(name) {
                    issues.push(issue(
                        None,
                        format!("models.routes.{route} references unknown profile `{name}`"),
                        "add the referenced profile under `models.profiles` or change the route target",
                        vec![format!("route={route}"), format!("profile={name}")],
                        Some(route.clone()),
                        Some("unknown profile".into()),
                    ));
                }
            }
        }
    }
    issues
}

fn validate_budgets(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(budgets) = map.get("budgets").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "budgets",
        budgets,
        &["run", "task", "iteration", "experiment_lane"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    for (scope, budget) in budgets {
        let Some(record) = budget.as_map() else {
            continue;
        };
        for key in [
            "iterations",
            "tokens",
            "tool_calls",
            "diff_lines",
            "cost_usd",
        ] {
            if let Some(value) = record.get(key) {
                let path = format!("budgets.{scope}.{key}");
                if key == "cost_usd" {
                    if value
                        .as_f64()
                        .map(|n| n.is_finite() && n > 0.0)
                        .unwrap_or(false)
                    {
                        continue;
                    }
                } else if value
                    .as_i64()
                    .map(|n| n > 0)
                    .or_else(|| value.as_u64().map(|n| n > 0))
                    .unwrap_or(false)
                {
                    continue;
                }
                issues.push(issue(
                    None,
                    format!("{path} must be a finite positive integer"),
                    "use a positive whole number for each budget field",
                    vec![format!("path={path}")],
                    Some(path.clone()),
                    Some("invalid budget".into()),
                ));
            }
        }
    }
    issues
}

fn validate_triggers(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(triggers) = map.get("triggers").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "triggers",
        triggers,
        &["list", "anti_recursion"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_rollback(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(rollback) = map.get("rollback").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "rollback",
        rollback,
        &[
            "required_when",
            "plan_required",
            "verify_command",
            "on_failure_after_merge",
        ],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_done(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(done) = map.get("done").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "done",
        done,
        &["require", "forbid"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_repo_intelligence(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(repo) = map.get("repo_intelligence").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "repo_intelligence",
        repo,
        &[
            "scale",
            "indexes",
            "generated_zones",
            "scope_control",
            "blast_radius",
        ],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_fleet(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(fleet) = map.get("fleet").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "fleet",
        fleet,
        &["max_workers", "isolation", "jnoccio", "telemetry"],
        &["max_workers"],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    if let Some(max_workers) = fleet.get("max_workers") {
        issues.extend(require_positive_integer(max_workers, "fleet.max_workers"));
    }
    issues
}

fn validate_research(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(research) = map.get("research").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "research",
        research,
        &[
            "version",
            "mode",
            "autonomy",
            "max_parallel",
            "timeout_seconds",
            "provider_policy",
            "extraction",
            "evidence",
            "safety",
            "budgets",
        ],
        &["version"],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    match research.get("version").and_then(YamlNode::as_str) {
        Some(version) if version == SUPPORTED_ZYAL_RESEARCH_VERSION => {}
        Some(other) => issues.push(issue(
            None,
            format!(
                "research.version must be `{}`",
                SUPPORTED_ZYAL_RESEARCH_VERSION
            ),
            "set `research.version: v1` to match the supported research block contract",
            vec![format!("research.version={other}")],
            Some("research.version".into()),
            Some("unsupported research version".into()),
        )),
        None => issues.push(issue(
            None,
            "research.version is required",
            "set `research.version: v1`",
            vec![format!(
                "supported_contract_version={SUPPORTED_ZYAL_CONTRACT_VERSION}"
            )],
            Some("research.version".into()),
            Some("required research version".into()),
        )),
    }
    if let Some(max_parallel) = research.get("max_parallel") {
        issues.extend(require_positive_integer(
            max_parallel,
            "research.max_parallel",
        ));
    }
    if let Some(timeout) = research.get("timeout_seconds") {
        issues.extend(require_positive_integer(
            timeout,
            "research.timeout_seconds",
        ));
    }
    if let Some(provider_policy) = research.get("provider_policy").and_then(YamlNode::as_map) {
        issues.extend(validate_keys(
            "research.provider_policy",
            provider_policy,
            &["prefer", "allow", "missing_provider"],
            &[],
            None,
            SUPPORTED_ZYAL_CONTRACT_VERSION,
            SUPPORTED_ZYAL_RELEASE_TAG,
        ));
    }
    if let Some(extraction) = research.get("extraction").and_then(YamlNode::as_map) {
        issues.extend(validate_keys(
            "research.extraction",
            extraction,
            &["enabled", "max_pages", "allowed_extractors"],
            &[],
            None,
            SUPPORTED_ZYAL_CONTRACT_VERSION,
            SUPPORTED_ZYAL_RELEASE_TAG,
        ));
        if let Some(max_pages) = extraction.get("max_pages") {
            issues.extend(require_positive_integer(
                max_pages,
                "research.extraction.max_pages",
            ));
        }
    }
    if let Some(budgets) = research.get("budgets").and_then(YamlNode::as_map) {
        issues.extend(validate_keys(
            "research.budgets",
            budgets,
            &["max_queries", "max_pages", "max_cost_usd"],
            &[],
            None,
            SUPPORTED_ZYAL_CONTRACT_VERSION,
            SUPPORTED_ZYAL_RELEASE_TAG,
        ));
        for key in ["max_queries", "max_pages"] {
            if let Some(value) = budgets.get(key) {
                issues.extend(require_positive_integer(
                    value,
                    &format!("research.budgets.{key}"),
                ));
            }
        }
        if let Some(cost) = budgets.get("max_cost_usd") {
            issues.extend(require_positive_cost(cost, "research.budgets.max_cost_usd"));
        }
    }
    issues
}

fn validate_taint(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(taint) = map.get("taint").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "taint",
        taint,
        &["default_label", "labels", "forbid", "prompt_injection"],
        &["labels"],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_interop(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(interop) = map.get("interop").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "interop",
        interop,
        &["protocols", "adapters", "compile_to", "notes"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_runtime(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(runtime) = map.get("runtime").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "runtime",
        runtime,
        &["mode", "image", "workspace", "network", "env", "resources"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_capability_negotiation(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(negotiation) = map.get("capability_negotiation").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "capability_negotiation",
        negotiation,
        &["host", "required", "optional", "fail_closed", "degrade_to"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_memory_kernel(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(memory) = map.get("memory_kernel").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "memory_kernel",
        memory,
        &["stores", "redaction", "provenance"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_evidence_graph(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(evidence_graph) = map.get("evidence_graph").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "evidence_graph",
        evidence_graph,
        &["nodes", "edges", "merge_witness"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_trust(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(trust) = map.get("trust").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "trust",
        trust,
        &["zones", "on_taint", "notes"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_requirements(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(requirements) = map.get("requirements").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "requirements",
        requirements,
        &["must", "should", "avoid"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_evaluation(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(evaluation) = map.get("evaluation").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "evaluation",
        evaluation,
        &["metrics", "compare"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_release(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(release) = map.get("release").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "release",
        release,
        &["channel", "version", "gates", "notes"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_roles(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(roles) = map.get("roles").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "roles",
        roles,
        &["list"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_channels(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(channels) = map.get("channels").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "channels",
        channels,
        &["list"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_imports(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(imports) = map.get("imports").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "imports",
        imports,
        &["list"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_reasoning_privacy(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(privacy) = map.get("reasoning_privacy").and_then(YamlNode::as_map) else {
        return issues;
    };
    issues.extend(validate_keys(
        "reasoning_privacy",
        privacy,
        &[
            "store_reasoning",
            "redact_chain_of_thought",
            "summaries_only",
        ],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    issues
}

fn validate_unsupported_feature_policy(map: &BTreeMap<String, YamlNode>) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let Some(policy) = map
        .get("unsupported_feature_policy")
        .and_then(YamlNode::as_map)
    else {
        return issues;
    };
    issues.extend(validate_keys(
        "unsupported_feature_policy",
        policy,
        &["required", "optional", "fail_closed", "on_missing"],
        &[],
        None,
        SUPPORTED_ZYAL_CONTRACT_VERSION,
        SUPPORTED_ZYAL_RELEASE_TAG,
    ));
    if let Some(required) = policy.get("required").and_then(YamlNode::as_seq) {
        let fail_closed = policy
            .get("fail_closed")
            .and_then(YamlNode::as_bool)
            .unwrap_or(true);
        let supported = [
            "version",
            "intent",
            "confirm",
            "job",
            "loop",
            "stop",
            "context",
            "checkpoint",
            "tasks",
            "incubator",
            "agents",
            "mcp",
            "permissions",
            "ui",
            "on",
            "fan_out",
            "guardrails",
            "assertions",
            "retry",
            "hooks",
            "constraints",
            "workflow",
            "memory",
            "evidence",
            "approvals",
            "skills",
            "sandbox",
            "security",
            "observability",
            "arming",
            "capabilities",
            "quality",
            "experiments",
            "models",
            "budgets",
            "triggers",
            "rollback",
            "done",
            "repo_intelligence",
            "fleet",
            "research",
            "taint",
            "interaction",
            "interop",
            "runtime",
            "capability_negotiation",
            "memory_kernel",
            "evidence_graph",
            "trust",
            "requirements",
            "evaluation",
            "release",
            "roles",
            "channels",
            "imports",
            "reasoning_privacy",
            "unsupported_feature_policy",
        ];
        for (index, item) in required.iter().enumerate() {
            match item.as_str() {
                Some(feature) if !feature.trim().is_empty() => {
                    if fail_closed && !supported.contains(&feature) {
                        issues.push(issue(
                            None,
                            format!("unsupported_feature_policy.required[{index}] `{feature}` is not supported"),
                            "remove the required feature or set `fail_closed: false` if preview fallback is acceptable",
                            vec![format!("feature={feature}"), format!("fail_closed={fail_closed}")],
                            Some(feature.into()),
                            Some("unknown required feature".into()),
                        ));
                    }
                }
                _ => issues.push(issue(
                    None,
                    format!(
                        "unsupported_feature_policy.required[{index}] must be a non-empty string"
                    ),
                    "use non-empty strings for required features",
                    vec![format!("index={index}")],
                    Some("required".into()),
                    Some("invalid required feature".into()),
                )),
            }
        }
    }
    issues
}

fn require_positive_integer(value: &YamlNode, path: &str) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let valid = value
        .as_i64()
        .map(|n| n > 0)
        .or_else(|| value.as_u64().map(|n| n > 0))
        .unwrap_or(false);
    if !valid {
        issues.push(issue(
            None,
            format!("{path} must be a finite positive integer"),
            "use a positive whole number",
            vec![format!("path={path}")],
            Some(path.into()),
            Some("non-positive integer".into()),
        ));
    }
    issues
}

fn require_score(value: &YamlNode, path: &str) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let valid = value
        .as_f64()
        .map(|n| n.is_finite() && n > 0.0 && n <= 1.0)
        .unwrap_or(false);
    if !valid {
        issues.push(issue(
            None,
            format!("{path} must be a finite score in (0, 1]"),
            "use a fractional score between 0 and 1",
            vec![format!("path={path}")],
            Some(path.into()),
            Some("invalid score".into()),
        ));
    }
    issues
}

fn require_positive_cost(value: &YamlNode, path: &str) -> Vec<IssueDraft> {
    let mut issues = Vec::new();
    let valid = value
        .as_f64()
        .map(|n| n.is_finite() && n > 0.0)
        .unwrap_or(false);
    if !valid {
        issues.push(issue(
            None,
            format!("{path} must be a finite positive number"),
            "use a positive numeric budget amount",
            vec![format!("path={path}")],
            Some(path.into()),
            Some("invalid cost".into()),
        ));
    }
    issues
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
