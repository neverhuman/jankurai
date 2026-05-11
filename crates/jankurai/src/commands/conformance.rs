use crate::audit::{run_audit_with_options, AuditOptions};
use crate::commands::witness::{build_witness, MergeWitness, WitnessArgs};
use crate::model::{AUDITOR_VERSION, SCHEMA_VERSION, STANDARD_VERSION};
use crate::validation::{self, ArtifactSchema};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ConformanceRunArgs {
    pub workspace: PathBuf,
    pub fixtures: PathBuf,
    pub expected: PathBuf,
    pub out: String,
    pub md: String,
    pub tex: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FixtureManifest {
    fixture_id: String,
    #[serde(default)]
    changed_paths: Vec<String>,
    expected_audit_decision: String,
    expected_witness_decision: String,
    #[serde(default)]
    expected_rules: Vec<String>,
    #[serde(default)]
    expected_missing_evidence: Vec<String>,
    #[serde(default)]
    proof_receipts: Option<String>,
    #[serde(default)]
    baseline: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConformanceRunReport {
    pub schema_url: String,
    pub schema_version: String,
    pub standard_version: String,
    pub auditor_version: String,
    pub command: String,
    pub generated_at: String,
    pub fixture_root: String,
    pub expected_root: String,
    pub fixture_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub results: Vec<FixtureResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FixtureResult {
    pub fixture_id: String,
    pub expected_audit_decision: String,
    pub observed_audit_decision: String,
    pub expected_witness_decision: String,
    pub observed_witness_decision: String,
    pub expected_rules: Vec<String>,
    pub observed_rules: Vec<String>,
    pub rule_matches: Vec<RuleMatch>,
    pub missing_expected_rules: Vec<String>,
    pub expected_missing_evidence: Vec<String>,
    pub missing_evidence: Vec<String>,
    pub missing_expected_evidence: Vec<String>,
    pub result: String,
    pub elapsed_ms: u128,
    pub artifact_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleMatch {
    pub rule_id: String,
    pub expected: bool,
    pub observed: bool,
}

pub fn run(args: ConformanceRunArgs) -> Result<()> {
    let report = build_report(&args)?;
    validation::write_json(
        &args.workspace,
        ArtifactSchema::ConformanceResults,
        &args.out,
        &report,
    )?;
    crate::render::write_markdown(&args.md, &render_markdown(&report))?;
    crate::render::write_markdown(&args.tex, &render_tex_table(&report, &args))?;
    if report.fail_count > 0 {
        anyhow::bail!("{} conformance fixture(s) mismatched", report.fail_count);
    }
    Ok(())
}

pub fn build_report(args: &ConformanceRunArgs) -> Result<ConformanceRunReport> {
    let workspace = args
        .workspace
        .canonicalize()
        .unwrap_or(args.workspace.clone());
    let fixture_root = resolve(&workspace, &args.fixtures);
    let expected_root = resolve(&workspace, &args.expected);
    let manifests = load_manifests(&fixture_root)?;
    let mut results = Vec::new();
    for manifest in manifests {
        results.push(run_fixture(
            &workspace,
            &fixture_root,
            &expected_root,
            &manifest,
        )?);
    }
    let pass_count = results
        .iter()
        .filter(|result| result.result == "pass")
        .count();
    let fail_count = results.len().saturating_sub(pass_count);
    Ok(ConformanceRunReport {
        schema_url: "schemas/conformance-results.schema.json".into(),
        schema_version: SCHEMA_VERSION.into(),
        standard_version: STANDARD_VERSION.into(),
        auditor_version: AUDITOR_VERSION.into(),
        command: "jankurai conformance run".into(),
        generated_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        fixture_root: rel_display(&workspace, &fixture_root),
        expected_root: rel_display(&workspace, &expected_root),
        fixture_count: results.len(),
        pass_count,
        fail_count,
        results,
    })
}

fn run_fixture(
    workspace: &Path,
    fixture_root: &Path,
    expected_root: &Path,
    manifest: &FixtureManifest,
) -> Result<FixtureResult> {
    let started = Instant::now();
    let repo = fixture_root.join(&manifest.fixture_id);
    let changed = manifest
        .changed_paths
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let proof_receipts = manifest.proof_receipts.clone();
    let report = run_audit_with_options(
        &repo,
        &changed,
        AuditOptions {
            self_audit: false,
            proof_receipts: proof_receipts.clone(),
            changed_fast: false,
        },
    )
    .with_context(|| format!("audit fixture {}", manifest.fixture_id))?;
    let observed_audit_decision = audit_decision(&report);
    let baseline = baseline_path(
        workspace,
        &repo,
        &manifest.fixture_id,
        &manifest.baseline,
        &report,
    )?;
    let witness = build_witness(&WitnessArgs {
        repo: repo.clone(),
        changed,
        changed_from: None,
        baseline,
        proof_receipts,
        out: String::new(),
        md: String::new(),
    })
    .with_context(|| format!("build witness for fixture {}", manifest.fixture_id))?;
    let observed_rules = observed_rules(&report, &witness);
    let rule_matches = rule_matches(&manifest.expected_rules, &observed_rules);
    let missing_expected_rules = manifest
        .expected_rules
        .iter()
        .filter(|rule| !observed_rules.contains(*rule))
        .cloned()
        .collect::<Vec<_>>();
    let missing_expected_evidence = manifest
        .expected_missing_evidence
        .iter()
        .filter(|needle| {
            !witness
                .missing_evidence
                .iter()
                .any(|evidence| evidence.contains(needle.as_str()))
        })
        .cloned()
        .collect::<Vec<_>>();
    let observed_witness_decision = decision_bucket(&witness.decision).to_string();
    let mut artifact_paths = Vec::new();
    if expected_root
        .join(format!("{}.repo-score.json", manifest.fixture_id))
        .exists()
    {
        artifact_paths.push(rel_display(
            workspace,
            &expected_root.join(format!("{}.repo-score.json", manifest.fixture_id)),
        ));
    }
    if let Some(receipts) = &manifest.proof_receipts {
        artifact_paths.push(rel_display(workspace, &repo.join(receipts)));
    }
    if matches!(manifest.baseline.as_deref(), Some("auto-current")) {
        artifact_paths.push(format!(
            "target/jankurai/conformance-baselines/{}.json",
            manifest.fixture_id
        ));
    }
    let result = if observed_audit_decision == manifest.expected_audit_decision
        && observed_witness_decision == manifest.expected_witness_decision
        && missing_expected_rules.is_empty()
        && missing_expected_evidence.is_empty()
    {
        "pass"
    } else {
        "fail"
    };
    Ok(FixtureResult {
        fixture_id: manifest.fixture_id.clone(),
        expected_audit_decision: manifest.expected_audit_decision.clone(),
        observed_audit_decision,
        expected_witness_decision: manifest.expected_witness_decision.clone(),
        observed_witness_decision,
        expected_rules: manifest.expected_rules.clone(),
        observed_rules,
        rule_matches,
        missing_expected_rules,
        expected_missing_evidence: manifest.expected_missing_evidence.clone(),
        missing_evidence: witness.missing_evidence,
        missing_expected_evidence,
        result: result.into(),
        elapsed_ms: started.elapsed().as_millis(),
        artifact_paths,
    })
}

fn load_manifests(fixture_root: &Path) -> Result<Vec<FixtureManifest>> {
    let mut manifests = Vec::new();
    for entry in fs::read_dir(fixture_root)
        .with_context(|| format!("read fixture root {}", fixture_root.display()))?
    {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }
        let manifest_path = entry.path().join("jankurai-fixture.toml");
        let text = fs::read_to_string(&manifest_path)
            .with_context(|| format!("read {}", manifest_path.display()))?;
        let manifest: FixtureManifest =
            toml::from_str(&text).with_context(|| format!("parse {}", manifest_path.display()))?;
        manifests.push(manifest);
    }
    manifests.sort_by(|a, b| a.fixture_id.cmp(&b.fixture_id));
    Ok(manifests)
}

fn baseline_path(
    workspace: &Path,
    repo: &Path,
    fixture_id: &str,
    baseline: &Option<String>,
    report: &crate::model::Report,
) -> Result<Option<String>> {
    let Some(baseline) = baseline else {
        return Ok(None);
    };
    if baseline == "auto-current" {
        let path = workspace
            .join("target/jankurai/conformance-baselines")
            .join(format!("{fixture_id}.json"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(report)?;
        fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
        return Ok(Some(path.display().to_string()));
    }
    Ok(Some(repo.join(baseline).display().to_string()))
}

fn audit_decision(report: &crate::model::Report) -> String {
    if report
        .decision
        .as_ref()
        .map(|decision| decision.passed)
        .unwrap_or(false)
    {
        "pass".into()
    } else {
        "block".into()
    }
}

fn decision_bucket(decision: &str) -> &str {
    if decision == "pass" {
        "pass"
    } else {
        "block"
    }
}

fn observed_rules(report: &crate::model::Report, witness: &MergeWitness) -> Vec<String> {
    let mut rules = BTreeSet::new();
    for finding in &report.findings {
        if let Some(rule_id) = &finding.rule_id {
            rules.insert(rule_id.clone());
        }
    }
    for route in &witness.route_decisions {
        if route.owner == "unmapped" {
            rules.insert("HLT-003-OWNERLESS-PATH".into());
        }
        if route.test_command == "unmapped" {
            rules.insert("HLT-004-UNMAPPED-PROOF".into());
        }
    }
    if !witness.generated_zone_touches.is_empty() {
        rules.insert("HLT-002-GENERATED-MUTATION".into());
    }
    rules.into_iter().collect()
}

fn rule_matches(expected: &[String], observed: &[String]) -> Vec<RuleMatch> {
    let expected_set = expected.iter().cloned().collect::<BTreeSet<_>>();
    let observed_set = observed.iter().cloned().collect::<BTreeSet<_>>();
    expected_set
        .union(&observed_set)
        .map(|rule_id| RuleMatch {
            rule_id: rule_id.clone(),
            expected: expected_set.contains(rule_id),
            observed: observed_set.contains(rule_id),
        })
        .collect()
}

pub fn render_markdown(report: &ConformanceRunReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Conformance Results");
    let _ = writeln!(out);
    let _ = writeln!(out, "- generated_at: `{}`", report.generated_at);
    let _ = writeln!(out, "- fixtures: `{}`", report.fixture_count);
    let _ = writeln!(
        out,
        "- result pass/fail: `{}`/`{}`",
        report.pass_count, report.fail_count
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "| Fixture | Audit | Witness | Rules | Result | ms |");
    let _ = writeln!(out, "| --- | --- | --- | --- | --- | ---: |");
    for result in &report.results {
        let rules = if result.expected_rules.is_empty() {
            "none".to_string()
        } else {
            result.expected_rules.join(", ")
        };
        let _ = writeln!(
            out,
            "| `{}` | `{}` -> `{}` | `{}` -> `{}` | `{}` | `{}` | {} |",
            result.fixture_id,
            result.expected_audit_decision,
            result.observed_audit_decision,
            result.expected_witness_decision,
            result.observed_witness_decision,
            rules,
            result.result,
            result.elapsed_ms
        );
    }
    out
}

pub fn render_tex_table(report: &ConformanceRunReport, args: &ConformanceRunArgs) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "% Generated by: jankurai {}", AUDITOR_VERSION);
    let _ = writeln!(
        out,
        "% Source: {}; {}",
        args.fixtures.display(),
        args.expected.display()
    );
    let _ = writeln!(
        out,
        "% Command: cargo run -p jankurai -- conformance run --fixtures {} --expected {} --out {} --md {} --tex {}",
        args.fixtures.display(),
        args.expected.display(),
        args.out,
        args.md,
        args.tex
    );
    let _ = writeln!(out, "% DO NOT EDIT BY HAND.");
    let _ = writeln!(out, "\\begin{{table*}}[t]");
    let _ = writeln!(out, "\\caption{{Observed conformance fixture results.}}");
    let _ = writeln!(out, "\\label{{tab:conformance-results}}");
    let _ = writeln!(out, "\\scriptsize");
    let _ = writeln!(
        out,
        "\\begin{{tabularx}}{{\\textwidth}}{{@{{}}L{{0.22\\textwidth}} L{{0.27\\textwidth}} L{{0.12\\textwidth}} Y r@{{}}}}"
    );
    let _ = writeln!(out, "\\toprule");
    let _ = writeln!(
        out,
        "Fixture class & Expected $\\rightarrow$ observed & Rule & Result & ms\\\\"
    );
    let _ = writeln!(out, "\\midrule");
    for result in &report.results {
        let rule = if result.expected_rules.is_empty() {
            "none".to_string()
        } else {
            short_rule_label(&result.expected_rules[0])
        };
        let _ = writeln!(out, "% Fixture id: {}", result.fixture_id);
        let _ = writeln!(
            out,
            "{} & audit {}$\\rightarrow${}; witness {}$\\rightarrow${} & \\texttt{{{}}} & {} & {}\\\\",
            tex_escape(&fixture_display_name(&result.fixture_id)),
            tex_escape(&result.expected_audit_decision),
            tex_escape(&result.observed_audit_decision),
            tex_escape(&result.expected_witness_decision),
            tex_escape(&result.observed_witness_decision),
            tex_escape(&rule),
            tex_escape(&result.result),
            result.elapsed_ms
        );
    }
    let _ = writeln!(out, "\\bottomrule");
    let _ = writeln!(out, "\\end{{tabularx}}");
    let _ = writeln!(out, "\\end{{table*}}");
    out
}

fn fixture_display_name(fixture_id: &str) -> String {
    fixture_id
        .strip_suffix("-fail")
        .unwrap_or(fixture_id)
        .replace('-', " ")
}

fn short_rule_label(rule_id: &str) -> String {
    let mut parts = rule_id.split('-');
    match (parts.next(), parts.next()) {
        (Some(prefix), Some(number)) => format!("{prefix}-{number}"),
        _ => rule_id.to_string(),
    }
}

fn tex_escape(value: &str) -> String {
    value
        .replace('\\', "\\textbackslash{}")
        .replace('&', "\\&")
        .replace('%', "\\%")
        .replace('$', "\\$")
        .replace('#', "\\#")
        .replace('_', "\\_")
        .replace('{', "\\{")
        .replace('}', "\\}")
}

fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn rel_display(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
