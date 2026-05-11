//! Phase 12: consume validated bench / certification / governance JSON and emit a public evidence bundle.

use crate::commands::release_data::load_release_data;
use crate::validation::{self, ArtifactSchema};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct PublishArgs {
    pub repo: PathBuf,
    pub certification: String,
    pub benchmark: String,
    pub governance: String,
    pub out: Option<String>,
    pub md: Option<String>,
    pub badge_json: Option<String>,
    pub badge_svg: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CertificationBadge {
    pub schema_version: String,
    pub label: String,
    pub message: String,
    pub color: String,
    pub score: i32,
    pub conformance_level: String,
    pub claimed_conformance_level: String,
    pub observed_conformance_level: String,
    pub conformance_decision: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub conformance_blockers: Vec<String>,
    pub standard_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublishedArtifact {
    pub path: String,
    pub role: String,
    pub present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub public: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicAttestation {
    pub kind: String,
    pub predicate_type: String,
    pub subject_digest: String,
    pub signature: String,
    pub signing_key_hint: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_head: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicEvidenceBundle {
    pub schema_version: String,
    pub generated_at: String,
    pub repo: String,
    pub standard_version: String,
    pub auditor_version: String,
    pub paper_edition: String,
    pub target_stack_id: String,
    pub public_status: String,
    pub publishable: bool,
    pub score: i32,
    pub conformance_level: String,
    pub claimed_conformance_level: String,
    pub observed_conformance_level: String,
    pub conformance_decision: String,
    pub conformance_blockers: Vec<String>,
    pub findings_summary: Value,
    pub benchmark_summary: Value,
    pub governance: Value,
    pub badge: CertificationBadge,
    pub artifacts: Vec<PublishedArtifact>,
    pub attestation: PublicAttestation,
    pub validation_commands: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub blocking_reasons: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub limitations: Vec<String>,
}

pub fn run(args: PublishArgs) -> Result<()> {
    let bundle = build_public_evidence_bundle(
        &args.repo,
        &args.certification,
        &args.benchmark,
        &args.governance,
    )?;

    if let Some(path) = args.out.as_deref() {
        validation::write_json(
            &args.repo,
            ArtifactSchema::PublicEvidenceBundle,
            path,
            &bundle,
        )?;
    } else {
        validation::validate_serializable(
            &args.repo,
            ArtifactSchema::PublicEvidenceBundle,
            &bundle,
        )?;
        println!("{}", serde_json::to_string_pretty(&bundle)?);
    }

    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(&bundle))?;
    }

    validation::validate_serializable(
        &args.repo,
        ArtifactSchema::CertificationBadge,
        &bundle.badge,
    )?;
    if let Some(path) = args.badge_json.as_deref() {
        validation::write_json(
            &args.repo,
            ArtifactSchema::CertificationBadge,
            path,
            &bundle.badge,
        )?;
    }
    if let Some(path) = args.badge_svg.as_deref() {
        write_bytes(path, render_badge_svg(&bundle.badge).as_bytes())?;
    }

    Ok(())
}

pub fn build_public_evidence_bundle(
    repo: &Path,
    certification_path: &str,
    benchmark_path: &str,
    governance_path: &str,
) -> Result<PublicEvidenceBundle> {
    let release = load_release_data(repo)?;

    let cert_abs = resolve_path(repo, certification_path);
    let bench_abs = resolve_path(repo, benchmark_path);
    let gov_abs = resolve_path(repo, governance_path);

    let cert_bytes = fs::read(&cert_abs)
        .with_context(|| format!("read certification {}", cert_abs.display()))?;
    let bench_bytes = fs::read(&bench_abs)
        .with_context(|| format!("read benchmark report {}", bench_abs.display()))?;
    let gov_bytes =
        fs::read(&gov_abs).with_context(|| format!("read governance {}", gov_abs.display()))?;

    let certification_value: Value =
        serde_json::from_slice(&cert_bytes).context("parse certification JSON")?;
    let benchmark_value: Value =
        serde_json::from_slice(&bench_bytes).context("parse benchmark JSON")?;
    let governance_value: Value =
        serde_json::from_slice(&gov_bytes).context("parse governance JSON")?;

    validation::validate_value(repo, ArtifactSchema::Certification, &certification_value)?;
    validation::validate_value(repo, ArtifactSchema::BenchmarkReport, &benchmark_value)?;
    validation::validate_value(repo, ArtifactSchema::GovernancePolicy, &governance_value)?;

    let cert_std = certification_value["standard_version"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("certification missing standard_version"))?;
    let gov_std = governance_value["standard_version"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("governance missing standard_version"))?;
    if cert_std != release.standard_version {
        bail!(
            "certification standard_version `{cert_std}` does not match manifest `{}`",
            release.standard_version
        );
    }
    if gov_std != release.standard_version {
        bail!(
            "governance standard_version `{gov_std}` does not match manifest `{}`",
            release.standard_version
        );
    }

    let gov_summary = serde_json::json!({
        "standard_version": governance_value["standard_version"],
        "minimum_score": governance_value["minimum_score"],
        "fail_on": governance_value["fail_on"],
        "advisory_on": governance_value["advisory_on"],
        "update_channel": governance_value["update_channel"],
        "rfc_path": governance_value["rfc_path"],
    });

    let (publishable, public_status, blocking_reasons) =
        evaluate_publishability(&certification_value, &benchmark_value, &governance_value);

    let mut artifacts = Vec::new();
    add_artifact(
        &mut artifacts,
        repo,
        certification_path,
        "certification",
        Some(ArtifactSchema::Certification.rel_path()),
        true,
    );
    add_artifact(
        &mut artifacts,
        repo,
        benchmark_path,
        "benchmark-report",
        Some(ArtifactSchema::BenchmarkReport.rel_path()),
        true,
    );
    add_artifact(
        &mut artifacts,
        repo,
        governance_path,
        "governance-policy",
        Some(ArtifactSchema::GovernancePolicy.rel_path()),
        true,
    );
    add_artifact(
        &mut artifacts,
        repo,
        "agent/standard-version.toml",
        "version-manifest",
        Some(ArtifactSchema::StandardVersion.rel_path()),
        true,
    );

    for path in certification_value["proof_receipt_index"]
        .as_array()
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter_map(Value::as_str)
    {
        add_artifact(
            &mut artifacts,
            repo,
            path,
            "proof-receipt",
            Some(ArtifactSchema::ProofReceipt.rel_path()),
            false,
        );
    }
    for path in certification_value["security_receipt_index"]
        .as_array()
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter_map(Value::as_str)
    {
        add_artifact(
            &mut artifacts,
            repo,
            path,
            "security-receipt",
            Some(ArtifactSchema::SecurityEvidence.rel_path()),
            false,
        );
    }
    for path in certification_value["ux_receipt_index"]
        .as_array()
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter_map(Value::as_str)
    {
        add_artifact(
            &mut artifacts,
            repo,
            path,
            "ux-receipt",
            Some(ArtifactSchema::UxQaReport.rel_path()),
            false,
        );
    }
    for path in certification_value["contract_db_receipt_index"]
        .as_array()
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter_map(Value::as_str)
    {
        add_artifact(
            &mut artifacts,
            repo,
            path,
            "contract-db-receipt",
            None,
            false,
        );
    }
    for path in certification_value["exceptions"]
        .as_array()
        .into_iter()
        .flat_map(|rows| rows.iter())
        .filter_map(Value::as_str)
    {
        add_artifact(
            &mut artifacts,
            repo,
            path,
            "exception-inventory",
            None,
            false,
        );
    }

    artifacts.sort_by(|a, b| (&a.path, &a.role).cmp(&(&b.path, &b.role)));
    artifacts.dedup_by(|a, b| a.path == b.path && a.role == b.role);

    let score_i = certification_value["score"].as_i64().unwrap_or(0) as i32;
    let conformance_level = certification_value["conformance_level"]
        .as_str()
        .unwrap_or("HL0")
        .to_string();
    let claimed_conformance_level = certification_value["claimed_conformance_level"]
        .as_str()
        .unwrap_or(&conformance_level)
        .to_string();
    let observed_conformance_level = certification_value["observed_conformance_level"]
        .as_str()
        .unwrap_or(&conformance_level)
        .to_string();
    let conformance_decision = if publishable { "pass" } else { "block" }.to_string();
    let conformance_blockers = blocking_reasons.clone();
    let badge_std = certification_value["standard_version"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let badge = CertificationBadge {
        schema_version: release.schema_version.clone(),
        label: "jankurai".to_string(),
        message: format!("{} score {}", conformance_level, score_i),
        color: badge_color(publishable, &public_status),
        score: score_i,
        conformance_level: conformance_level.clone(),
        claimed_conformance_level: claimed_conformance_level.clone(),
        observed_conformance_level: observed_conformance_level.clone(),
        conformance_decision: conformance_decision.clone(),
        conformance_blockers: conformance_blockers.clone(),
        standard_version: badge_std,
        artifact: Some("target/jankurai/public/p12-public-evidence.json".to_string()),
    };

    let subject_digest = triple_file_subject_digest(
        cert_bytes.as_slice(),
        bench_bytes.as_slice(),
        gov_bytes.as_slice(),
    );

    let mut limitations: Vec<String> = benchmark_value["limitations"]
        .as_array()
        .into_iter()
        .flat_map(|xs| xs.iter())
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    limitations.push(
        "Attestation is local digest evidence; external Sigstore or GitHub artifact attestation can wrap the same files without changing this bundle."
            .to_string(),
    );
    limitations.push(
        "Hosted dashboards are optional; CI artifacts and badge outputs are the publishable surface."
        .to_string(),
    );

    let bundle = PublicEvidenceBundle {
        schema_version: release.schema_version.clone(),
        generated_at: now_string(),
        repo: certification_value["repo"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        standard_version: release.standard_version.clone(),
        auditor_version: certification_value["auditor_version"]
            .as_str()
            .unwrap_or(&release.auditor_version)
            .to_string(),
        paper_edition: certification_value["paper_edition"]
            .as_str()
            .unwrap_or(&release.paper_edition)
            .to_string(),
        target_stack_id: certification_value["target_stack_id"]
            .as_str()
            .unwrap_or(&release.target_stack_id)
            .to_string(),
        public_status: public_status.clone(),
        publishable,
        score: score_i,
        conformance_level,
        claimed_conformance_level,
        observed_conformance_level,
        conformance_decision,
        conformance_blockers,
        findings_summary: certification_value["findings_summary"].clone(),
        benchmark_summary: benchmark_value["summary"].clone(),
        governance: gov_summary,
        badge: badge.clone(),
        artifacts,
        attestation: PublicAttestation {
            kind: "local-sha256-attestation".to_string(),
            predicate_type: "https://jankurai.dev/attestations/phase12-public-evidence/v1".to_string(),
            subject_digest: subject_digest.clone(),
            signature: format!(
                "local-sha256-attestation:{}",
                subject_digest.trim_start_matches("sha256:")
            ),
            signing_key_hint:
                "local triple-file digest only; attach external identity if you need non-repudiation"
                    .to_string(),
            command: "jankurai publish".to_string(),
            git_head: git_head(repo),
            build_url: build_url(),
        },
        validation_commands: vec![
            "cargo run -p jankurai -- bench . --out target/jankurai/p12-benchmark-report.json --md target/jankurai/p12-benchmark-report.md".to_string(),
            "cargo run -p jankurai -- certify . --out target/jankurai/p12-certification.json --md target/jankurai/p12-certification.md".to_string(),
            "cargo run -p jankurai -- govern . --out target/jankurai/p12-governance-policy.json --md target/jankurai/p12-governance-policy.md".to_string(),
            "cargo run -p jankurai -- publish . --certification target/jankurai/p12-certification.json --benchmark target/jankurai/p12-benchmark-report.json --governance target/jankurai/p12-governance-policy.json --out target/jankurai/public/p12-public-evidence.json --md target/jankurai/public/p12-public-evidence.md --badge-json target/jankurai/public/jankurai-badge.json --badge-svg target/jankurai/public/jankurai-badge.svg".to_string(),
        ],
        blocking_reasons,
        limitations,
    };

    validation::validate_serializable(repo, ArtifactSchema::PublicEvidenceBundle, &bundle)?;
    Ok(bundle)
}

fn triple_file_subject_digest(cert: &[u8], bench: &[u8], gov: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"jankurai phase12 publish subject v1\n");
    hasher.update(Sha256::digest(cert));
    hasher.update(Sha256::digest(bench));
    hasher.update(Sha256::digest(gov));
    format!("sha256:{:x}", hasher.finalize())
}

fn evaluate_publishability(
    certification: &Value,
    benchmark: &Value,
    governance: &Value,
) -> (bool, String, Vec<String>) {
    let mut reasons = Vec::new();

    let score = certification["score"].as_i64().unwrap_or(0) as i32;
    let min_score = governance["minimum_score"].as_i64().unwrap_or(85) as i32;
    if score < min_score {
        reasons.push(format!(
            "score {score} is below governance minimum {min_score}"
        ));
    }

    let crit = certification["findings_summary"]["critical"]
        .as_i64()
        .unwrap_or(0);
    if crit > 0 {
        reasons.push(format!("{crit} critical finding(s) present"));
    }
    let high = certification["findings_summary"]["high"]
        .as_i64()
        .unwrap_or(0);
    if high > 0 {
        reasons.push(format!("{high} high finding(s) present"));
    }

    let caps = certification["caps"].as_array().map(Vec::len).unwrap_or(0);
    if caps > 0 {
        reasons.push("hard caps are present".to_string());
    }

    let failed = benchmark["summary"]["failed"].as_i64().unwrap_or(0);
    if failed > 0 {
        reasons.push(format!("{failed} benchmark task(s) failed"));
    }
    let inconclusive = benchmark["summary"]["inconclusive"].as_i64().unwrap_or(0);
    if inconclusive > 0 {
        reasons.push(format!("{inconclusive} benchmark task(s) inconclusive"));
    }

    let publishable = reasons.is_empty();
    let status = if publishable {
        "publishable"
    } else if score > 0 {
        "advisory"
    } else {
        "blocked"
    };

    (publishable, status.to_string(), reasons)
}

fn badge_color(publishable: bool, status: &str) -> String {
    if publishable {
        "brightgreen".to_string()
    } else if status == "advisory" {
        "yellow".to_string()
    } else {
        "red".to_string()
    }
}

fn add_artifact(
    artifacts: &mut Vec<PublishedArtifact>,
    repo: &Path,
    path: &str,
    role: &str,
    schema: Option<&str>,
    public: bool,
) {
    let normalized = normalize_path(repo, path);
    if artifacts
        .iter()
        .any(|a| a.path == normalized && a.role == role)
    {
        return;
    }
    let abs = resolve_path(repo, path);
    let present = abs.exists();
    let sha256 = if present && abs.is_file() {
        fs::read(&abs)
            .ok()
            .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
    } else {
        None
    };
    artifacts.push(PublishedArtifact {
        path: normalized,
        role: role.to_string(),
        present,
        sha256,
        schema: schema.map(str::to_string),
        public,
    });
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
    let rel = if candidate.is_absolute() {
        candidate
            .strip_prefix(repo)
            .map(Path::to_path_buf)
            .unwrap_or(candidate)
    } else {
        candidate
    };
    rel.to_string_lossy().replace('\\', "/")
}

fn git_head(repo: &Path) -> Option<String> {
    Command::new("git")
        .current_dir(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn build_url() -> Option<String> {
    match (
        env::var("GITHUB_SERVER_URL"),
        env::var("GITHUB_REPOSITORY"),
        env::var("GITHUB_RUN_ID"),
    ) {
        (Ok(server), Ok(repository), Ok(run_id)) => {
            Some(format!("{server}/{repository}/actions/runs/{run_id}"))
        }
        _ => env::var("BUILD_URL").ok(),
    }
}

fn write_bytes(path: &str, content: &[u8]) -> Result<()> {
    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, content).with_context(|| format!("write {}", path))?;
    Ok(())
}

fn render_markdown(bundle: &PublicEvidenceBundle) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Public Evidence");
    let _ = writeln!(out);
    let _ = writeln!(out, "- public status: `{}`", bundle.public_status);
    let _ = writeln!(out, "- publishable: `{}`", bundle.publishable);
    let _ = writeln!(out, "- repo: `{}`", bundle.repo);
    let _ = writeln!(out, "- standard version: `{}`", bundle.standard_version);
    let _ = writeln!(out, "- score: `{}`", bundle.score);
    let _ = writeln!(out, "- conformance: `{}`", bundle.conformance_level);
    let _ = writeln!(
        out,
        "- benchmark: passed=`{}` failed=`{}` inconclusive=`{}`",
        bundle.benchmark_summary["passed"].as_i64().unwrap_or(0),
        bundle.benchmark_summary["failed"].as_i64().unwrap_or(0),
        bundle.benchmark_summary["inconclusive"]
            .as_i64()
            .unwrap_or(0),
    );
    if !bundle.blocking_reasons.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "## Blocking reasons");
        for r in &bundle.blocking_reasons {
            let _ = writeln!(out, "- {}", r);
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Badge");
    let _ = writeln!(out, "- message: `{}`", bundle.badge.message);
    let _ = writeln!(out, "- color: `{}`", bundle.badge.color);
    let _ = writeln!(out);
    let _ = writeln!(out, "## Attestation");
    let _ = writeln!(
        out,
        "- subject digest: `{}`",
        bundle.attestation.subject_digest
    );
    if let Some(url) = &bundle.attestation.build_url {
        let _ = writeln!(out, "- build URL: `{}`", url);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Reproduction");
    for c in &bundle.validation_commands {
        let _ = writeln!(out, "- `{}`", c);
    }
    out
}

fn render_badge_svg(badge: &CertificationBadge) -> String {
    let label = escape_xml(&badge.label);
    let message = escape_xml(&badge.message);
    let color = badge_hex_color(&badge.color);
    let label_width = ((label.chars().count() as i32 * 7) + 22).max(64);
    let message_width = ((message.chars().count() as i32 * 7) + 22).max(86);
    let width = label_width + message_width;
    let label_center = label_width / 2;
    let message_center = label_width + (message_width / 2);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="20" role="img" aria-label="{label}: {message}">
  <title>{label}: {message}</title>
  <linearGradient id="s" x2="0" y2="100%">
    <stop offset="0" stop-color="#fff" stop-opacity=".7"/>
    <stop offset=".1" stop-color="#aaa" stop-opacity=".1"/>
    <stop offset=".9" stop-color="#000" stop-opacity=".3"/>
    <stop offset="1" stop-color="#000" stop-opacity=".5"/>
  </linearGradient>
  <clipPath id="r">
    <rect width="{width}" height="20" rx="3" fill="#fff"/>
  </clipPath>
  <g clip-path="url(#r)">
    <rect width="{label_width}" height="20" fill="#555"/>
    <rect x="{label_width}" width="{message_width}" height="20" fill="{color}"/>
    <rect width="{width}" height="20" fill="url(#s)"/>
  </g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="11">
    <text x="{label_center}" y="15" fill="#010101" fill-opacity=".3">{label}</text>
    <text x="{label_center}" y="14">{label}</text>
    <text x="{message_center}" y="15" fill="#010101" fill-opacity=".3">{message}</text>
    <text x="{message_center}" y="14">{message}</text>
  </g>
</svg>
"##
    )
}

fn badge_hex_color(color: &str) -> &'static str {
    match color {
        "brightgreen" => "#4c1",
        "green" => "#97ca00",
        "yellowgreen" => "#a4a61d",
        "yellow" => "#dfb317",
        "orange" => "#fe7d37",
        "red" => "#e05d44",
        "blue" => "#007ec6",
        _ => "#9f9f9f",
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
