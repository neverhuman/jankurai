use super::helpers::*;
use super::rule_analyzer::FindingDraft;
use super::rules;
use crate::model::*;
use sha2::{Digest, Sha256};

pub struct FindingBuilder<'a> {
    ctx: &'a AuditContext,
    findings: Vec<Finding>,
    has_any_finding: bool,
    has_context_finding: bool,
}

impl<'a> FindingBuilder<'a> {
    pub fn new(ctx: &'a AuditContext) -> Self {
        Self {
            ctx,
            findings: Vec::new(),
            has_any_finding: false,
            has_context_finding: false,
        }
    }

    // Finding construction mirrors the report schema fields to keep call sites auditable.
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &mut self,
        severity: &str,
        category: &str,
        path: &str,
        problem: &str,
        fix: &str,
        evidence: Vec<String>,
        rule_id: Option<&str>,
        line: Option<usize>,
    ) {
        if category == "context" {
            self.has_context_finding = true;
        }
        self.has_any_finding = true;
        let rule = rule_id.unwrap_or("HLT-000-SCORE-DIMENSION");
        let rule_meta = rule_id.and_then(rules::lookup);
        let lane = rule_meta
            .map(|rule| rule.lane)
            .or_else(|| lane_for(category))
            .map(|s| s.into());
        let owner = owner_for_path(self.ctx, path).or_else(|| {
            rule_meta
                .filter(|rule| !rule.owner_hint.is_empty())
                .map(|rule| rule.owner_hint.to_string())
        });
        let evidence_kind = rule_meta
            .map(|rule| rule.evidence_kind)
            .unwrap_or_else(|| evidence_kind_for_path(path));
        let fingerprint = finding_fingerprint(rule, category, path, problem, &evidence);
        self.findings.push(Finding {
            severity: severity.into(),
            category: category.into(),
            path: path.into(),
            problem: problem.into(),
            agent_fix: fix.into(),
            evidence,
            check_id: format!("{rule}:{category}"),
            hardness: hardness_for_severity(severity).into(),
            confidence: confidence_for_severity(severity),
            evidence_kind: evidence_kind.into(),
            rerun_command: rerun_command_for_lane(lane.as_deref()).into(),
            fingerprint,
            rule_id: rule_id.map(|s| s.into()),
            tlr: rule_meta
                .map(|rule| rule.tlr)
                .or_else(|| tlr_for(category))
                .map(|s| s.into()),
            lane,
            docs_url: rule_id.and_then(rules::docs_for_rule_id).map(|s| s.into()),
            owner,
            line,
            matched_term: None,
            reason: None,
        })
    }

    // Rule-backed findings carry explicit evidence, location, and semantic match data.
    #[allow(clippy::too_many_arguments)]
    pub fn add_with_rule(
        &mut self,
        rule_id: &str,
        path: &str,
        problem: &str,
        fix: &str,
        evidence: Vec<String>,
        line: Option<usize>,
        matched_term: Option<String>,
        reason: Option<String>,
    ) {
        self.add_with_rule_and_rerun(
            rule_id,
            path,
            problem,
            fix,
            evidence,
            line,
            matched_term,
            reason,
            None,
        );
    }

    // Same as `add_with_rule`, with an override for generated repair receipts.
    #[allow(clippy::too_many_arguments)]
    pub fn add_with_rule_and_rerun(
        &mut self,
        rule_id: &str,
        path: &str,
        problem: &str,
        fix: &str,
        evidence: Vec<String>,
        line: Option<usize>,
        matched_term: Option<String>,
        reason: Option<String>,
        rerun_command: Option<&str>,
    ) {
        let rule = rules::lookup(rule_id).expect("rule_id must exist in registry");
        if rule.category == "context" {
            self.has_context_finding = true;
        }
        self.has_any_finding = true;

        let owner = owner_for_path(self.ctx, path).or_else(|| {
            if !rule.owner_hint.is_empty() {
                Some(rule.owner_hint.to_string())
            } else {
                None
            }
        });

        let fingerprint = finding_fingerprint(rule.id, rule.category, path, problem, &evidence);

        let confidence = match rule.confidence_policy {
            rules::ConfidencePolicy::High => 0.95,
            rules::ConfidencePolicy::Medium => 0.88,
            rules::ConfidencePolicy::Low => 0.62,
        };

        self.findings.push(Finding {
            severity: rule.severity.into(),
            category: rule.category.into(),
            path: path.into(),
            problem: problem.into(),
            agent_fix: fix.into(),
            evidence,
            check_id: format!("{}:{}", rule.id, rule.category),
            hardness: hardness_for_severity(rule.severity).into(),
            confidence,
            evidence_kind: rule.evidence_kind.into(),
            rerun_command: rerun_command
                .unwrap_or_else(|| rerun_command_for_lane(Some(rule.lane)))
                .into(),
            fingerprint,
            rule_id: Some(rule.id.into()),
            tlr: Some(rule.tlr.into()),
            lane: Some(rule.lane.into()),
            docs_url: Some(rule.docs_url.into()),
            owner,
            line,
            matched_term,
            reason,
        })
    }

    pub fn add_draft(&mut self, draft: FindingDraft) {
        self.add_with_rule(
            draft.rule_id,
            &draft.path,
            &draft.problem,
            &draft.fix,
            draft.evidence,
            draft.line,
            draft.matched_term,
            draft.reason,
        );
    }

    pub fn has_any_finding(&self) -> bool {
        self.has_any_finding
    }

    pub fn has_context_finding(&self) -> bool {
        self.has_context_finding
    }

    pub fn into_findings(self) -> Vec<Finding> {
        self.findings
    }
}

pub fn hardness_for_severity(severity: &str) -> &'static str {
    match severity {
        "critical" | "high" => "hard",
        _ => "soft",
    }
}

pub fn confidence_for_severity(severity: &str) -> f64 {
    match severity {
        "critical" => 0.95,
        "high" => 0.88,
        "medium" => 0.76,
        _ => 0.62,
    }
}

pub fn evidence_kind_for_path(path: &str) -> &'static str {
    if path.starts_with(".github/workflows") {
        "workflow-command"
    } else if path.starts_with("agent/") {
        "policy-manifest"
    } else if path.starts_with("docs/") {
        "documentation"
    } else {
        "repository-scan"
    }
}

pub fn rerun_command_for_lane(lane: Option<&str>) -> &'static str {
    match lane.unwrap_or("audit") {
        "security" => "just security",
        "contract" => "just fast",
        "db" => "just fast",
        "db-migration-analyze" => {
            "cargo run -p jankurai -- migrate . --analyze --json target/jankurai/migration-report.json"
        }
        "copy-code" => {
            "cargo run -p jankurai -- copy-code . --json target/jankurai/copy-code.json --md target/jankurai/copy-code.md"
        }
        "web" | "e2e" => "just ux-qa",
        "fast" => "just fast",
        "release" => "just check",
        _ => "just score",
    }
}

pub fn finding_fingerprint(
    rule_id: &str,
    category: &str,
    path: &str,
    problem: &str,
    evidence: &[String],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(rule_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(category.as_bytes());
    hasher.update(b"\0");
    hasher.update(path.as_bytes());
    hasher.update(b"\0");
    hasher.update(problem.as_bytes());
    for item in evidence {
        hasher.update(b"\0");
        hasher.update(item.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

pub fn tlr_for(category: &str) -> Option<&'static str> {
    Some(match category {
        "security" => "Security",
        "python" => "Business truth",
        "data" => "Contracts/data",
        "test" => "Verification",
        "audit" => "Context/setup",
        "generated" => "Contracts/data",
        "boundary" => "Contracts/data",
        "context" => "Context/setup",
        "vibe" => "Entropy",
        "shape" => "Entropy",
        "observability" => "Repair",
        "docs" => "Context/setup",
        "proof" => "Verification",
        "stack" => "Context/setup",
        "ux-qa" => "Verification and rendered UX",
        "exceptions" => "Repair",
        _ => return None,
    })
}

pub fn lane_for(category: &str) -> Option<&'static str> {
    Some(match category {
        "security" => "security",
        "python" => "contract",
        "data" => "db",
        "test" => "fast",
        "audit" => "audit",
        "generated" => "contract",
        "boundary" => "contract",
        "context" => "fast",
        "vibe" => "fast",
        "shape" => "fast",
        "observability" => "observability",
        "docs" => "audit",
        "proof" => "fast",
        "stack" => "audit",
        "ux-qa" => "web",
        "exceptions" => "observability",
        _ => return None,
    })
}

fn owner_for_path(ctx: &AuditContext, rel_path: &str) -> Option<String> {
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct OwnerMapFile {
        owners: HashMap<String, String>,
    }

    if let Ok(text) = std::fs::read_to_string(ctx.root.join("agent/owner-map.json")) {
        if let Ok(parsed) = serde_json::from_str::<OwnerMapFile>(&text) {
            let mut entries: Vec<_> = parsed.owners.iter().collect();
            entries.sort_by(|a, b| {
                b.0.len()
                    .cmp(&a.0.len())
                    .then_with(|| a.0.cmp(b.0))
                    .then_with(|| a.1.cmp(b.1))
            });
            for (prefix, owner) in entries {
                if rel_path == prefix || rel_path.starts_with(prefix) {
                    return Some((*owner).clone());
                }
            }
        }
    }
    for (prefix, owner) in OWNER_MAP_PREFIXES {
        if rel_path == *prefix || rel_path.starts_with(prefix) {
            return Some((*owner).to_string());
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::model::FileInfo;
    use tempfile::tempdir;

    fn ctx_with_owner_map(json: &str) -> AuditContext {
        let dir = tempdir().unwrap();
        let agent_dir = dir.path().join("agent");
        std::fs::create_dir_all(&agent_dir).unwrap();
        std::fs::write(agent_dir.join("owner-map.json"), json).unwrap();
        let root = dir.keep();
        AuditContext {
            root,
            all_files: vec![FileInfo {
                rel_path: "README.md".into(),
                name: "README.md".into(),
                suffix: ".md".into(),
                size: 0,
                line_count: 1,
                text: String::new(),
                is_generated: false,
                is_code: false,
            }],
            scope_files: vec![],
            scope_paths: vec![],
            self_audit: false,
            boundary_reclassifications: vec![],
            copy_code: None,
        }
    }

    #[test]
    fn fingerprint_is_stable_for_identical_inputs() {
        let a = finding_fingerprint(
            "HLT-001-DEAD-MARKER",
            "vibe",
            "README.md",
            "problem",
            &["evidence-one".into(), "evidence-two".into()],
        );
        let b = finding_fingerprint(
            "HLT-001-DEAD-MARKER",
            "vibe",
            "README.md",
            "problem",
            &["evidence-one".into(), "evidence-two".into()],
        );
        let c = finding_fingerprint(
            "HLT-001-DEAD-MARKER",
            "vibe",
            "README.md",
            "problem changed",
            &["evidence-one".into(), "evidence-two".into()],
        );

        assert_eq!(a, b);
        assert!(a.starts_with("sha256:"));
        assert_ne!(a, c);
    }

    #[test]
    fn owner_routing_prefers_more_specific_prefixes() {
        let ctx = ctx_with_owner_map(
            r#"{"owners":{"crates/":"workspace","crates/domain/":"domain","README.md":"workspace"}}"#,
        );
        let mut builder = FindingBuilder::new(&ctx);
        builder.add_with_rule(
            "HLT-006-DIRECT-DB-WRONG-LAYER",
            "crates/domain/src/lib.rs",
            "problem",
            "fix",
            vec!["evidence".into()],
            Some(12),
            Some("sqlx::".into()),
            Some("domain layer must not own DB access".into()),
        );
        let finding = builder.into_findings().pop().expect("one finding");
        assert_eq!(finding.owner.as_deref(), Some("domain"));
        assert_eq!(finding.matched_term.as_deref(), Some("sqlx::"));
        assert!(finding.reason.as_deref().unwrap().contains("domain layer"));
    }

    #[test]
    fn add_draft_preserves_semantic_evidence() {
        let ctx = ctx_with_owner_map(r#"{"owners":{"README.md":"workspace"}}"#);
        let mut builder = FindingBuilder::new(&ctx);
        builder.add_draft(FindingDraft {
            rule_id: "HLT-007-HANDWRITTEN-CONTRACT",
            path: "README.md".into(),
            problem: "handwritten mirror detected".into(),
            fix: "regenerate the contract".into(),
            evidence: vec!["import edge".into(), "generated contract absent".into()],
            line: Some(4),
            matched_term: Some("contracts/generated".into()),
            reason: Some("manual mirror is drifting from generated source".into()),
        });
        let finding = builder.into_findings().pop().expect("one finding");
        assert_eq!(
            finding.rule_id.as_deref(),
            Some("HLT-007-HANDWRITTEN-CONTRACT")
        );
        assert_eq!(finding.matched_term.as_deref(), Some("contracts/generated"));
        assert_eq!(
            finding.reason.as_deref(),
            Some("manual mirror is drifting from generated source")
        );
        assert_eq!(finding.owner.as_deref(), Some("workspace"));
    }
}

pub fn dimension_soft_route(
    name: &str,
) -> (&'static str, &'static str, &'static str, &'static str) {
    match name {
        "Ownership and navigation surface" => (
            "context",
            "agent/owner-map.json",
            "HLT-003-OWNERLESS-PATH",
            "tighten owner/test maps and root routing until agents can localize ownership without inference",
        ),
        "Contract and boundary integrity" => (
            "boundary",
            "agent/boundaries.toml",
            "HLT-007-HANDWRITTEN-CONTRACT",
            "add generated contracts and boundary checks for public APIs, data access, and cross-runtime seams",
        ),
        "Proof lanes and test routing" => (
            "proof",
            "agent/test-map.json",
            "HLT-004-UNMAPPED-PROOF",
            "route each owned path to a deterministic proof command and make the lane executable in CI",
        ),
        "Security and supply-chain posture" => (
            "security",
            ".github/workflows/jankurai.yml",
            "HLT-016-SUPPLY-CHAIN-DRIFT",
            "wire secret, dependency, provenance, and workflow scans into an operational CI lane",
        ),
        "Code shape and semantic surface" => (
            "shape",
            ".",
            "HLT-001-DEAD-MARKER",
            "split large or ambiguous authored code into smaller semantic modules with focused tests",
        ),
        "Data truth and workflow safety" => (
            "data",
            "db/",
            "HLT-006-DIRECT-DB-WRONG-LAYER",
            "move durable truth into migrations, constraints, adapters, and application-owned transactions",
        ),
        "Observability and repair evidence" => (
            "observability",
            "docs/testing.md",
            "HLT-017-OPAQUE-OBSERVABILITY",
            "add structured errors, telemetry, and repair receipts that tell the next agent where to rerun proof",
        ),
        "Context economy and agent instructions" => (
            "context",
            "AGENTS.md",
            "HLT-015-CONTEXT-SETUP-GAP",
            "keep root guidance short and route durable detail through agent-readable manifests and docs",
        ),
        "Python containment and polyglot hygiene" => (
            "python",
            "python/ai-service",
            "HLT-005-PYTHON-PRODUCT-TRUTH",
            "remove Python unless it is a dated advanced-ML/data exception and move product truth into Rust, SQL, and generated contracts",
        ),
        "Build speed signals" => (
            "proof",
            "Justfile",
            "HLT-018-PERF-CONCURRENCY-DRIFT",
            "add fast deterministic build/test targets, caches, and narrow proof lanes for agent iteration",
        ),
        "Jankurai tool adoption and CI replacement" => (
            "audit",
            "agent/tool-adoption.toml",
            "HLT-020-CI-HARDENING-GAP",
            "add the next highest-value Jankurai-backed CI lane and upload the expected artifact evidence",
        ),
        _ => (
            "audit",
            "agent/audit-policy.toml",
            "HLT-017-OPAQUE-OBSERVABILITY",
            "add a rule-specific repair route for this below-floor dimension",
        ),
    }
}
