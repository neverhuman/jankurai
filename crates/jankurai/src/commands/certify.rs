use crate::commands::release_data::{
    load_release_data, read_repo_score, workspace_root, FindingsSummary,
};
use crate::validation::{self, ArtifactSchema};
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct CertifyArgs {
    pub repo: PathBuf,
    pub out: Option<String>,
    pub md: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Certification {
    pub schema_version: String,
    pub generated_at: String,
    pub repo: String,
    pub standard_version: String,
    pub auditor_version: String,
    pub paper_edition: String,
    pub target_stack_id: String,
    pub score: i32,
    pub conformance_level: String,
    pub caps: Vec<String>,
    pub findings_summary: FindingsSummary,
    pub proof_receipt_index: Vec<String>,
    pub security_receipt_index: Vec<String>,
    pub ux_receipt_index: Vec<String>,
    pub contract_db_receipt_index: Vec<String>,
    pub exceptions: Vec<String>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Serialize)]
pub struct Provenance {
    pub attestation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_url: Option<String>,
}

pub fn run(args: CertifyArgs) -> Result<()> {
    let certification = build_certification(&args.repo)?;
    if let Some(path) = args.out.as_deref() {
        validation::write_json(
            &args.repo,
            ArtifactSchema::Certification,
            path,
            &certification,
        )?;
    } else {
        validation::validate_serializable(
            &args.repo,
            ArtifactSchema::Certification,
            &certification,
        )?;
        println!("{}", serde_json::to_string_pretty(&certification)?);
    }
    if let Some(path) = args.md.as_deref() {
        crate::render::write_markdown(path, &render_markdown(&certification))?;
    }
    Ok(())
}

pub fn build_certification(repo: &Path) -> Result<Certification> {
    let release = load_release_data(repo)?;
    let score_path = repo.join("agent/repo-score.json");
    let score = read_repo_score(repo)?;
    let evidence_root = if score_path.exists() {
        repo.to_path_buf()
    } else {
        workspace_root()
    };

    let (score_value, caps, mut findings_summary) = match score.as_ref() {
        Some(summary) => (
            summary.score,
            summary.caps.clone(),
            FindingsSummary {
                critical: summary.findings.critical,
                high: summary.findings.high,
                medium: summary.findings.medium,
                low: summary.findings.low,
                notes: vec![
                    format!("repo score artifact: {}", score_path.display()),
                    "local evidence artifact only; no external signing service is used".to_string(),
                ],
            },
        ),
        None => (
            0,
            Vec::new(),
            FindingsSummary {
                critical: 0,
                high: 0,
                medium: 0,
                low: 0,
                notes: vec![
                    format!("score artifact missing: {}", score_path.display()),
                    "local evidence artifact only; no external signing service is used".to_string(),
                ],
            },
        ),
    };
    if let Some(summary) = score.as_ref() {
        if summary.caps.is_empty() {
            findings_summary
                .notes
                .push("no hard caps reported in the score artifact".to_string());
        }
    }

    let conformance_level = conformance_level(score_value, &caps, &findings_summary);
    Ok(Certification {
        schema_version: release.schema_version,
        generated_at: now_string(),
        repo: repo.display().to_string(),
        standard_version: release.standard_version,
        auditor_version: release.auditor_version,
        paper_edition: release.paper_edition,
        target_stack_id: release.target_stack_id,
        score: score_value,
        conformance_level,
        caps,
        findings_summary,
        proof_receipt_index: collect_receipts(&evidence_root)?,
        security_receipt_index: collect_security_receipts(&evidence_root)?,
        ux_receipt_index: collect_ux_receipts(&evidence_root)?,
        contract_db_receipt_index: collect_contract_db_receipts(&evidence_root)?,
        exceptions: collect_exceptions(&evidence_root)?,
        provenance: Provenance {
            attestation: "local-artifact-only".to_string(),
            signature: None,
            build_url: None,
        },
    })
}

fn conformance_level(score: i32, caps: &[String], findings: &FindingsSummary) -> String {
    if score >= 93 && caps.is_empty() && findings.high == 0 && findings.critical == 0 {
        "HL3".to_string()
    } else if score >= 85 && findings.critical == 0 {
        "HL2".to_string()
    } else if score > 0 {
        "HL1".to_string()
    } else {
        "HL0".to_string()
    }
}

fn collect_receipts(root: &Path) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    for candidate in [
        "target/jankurai/p10-cell-registry-lane.json",
        "target/jankurai/p10-audit-log-proof.json",
    ] {
        if root.join(candidate).exists() {
            paths.push(candidate.to_string());
        }
    }
    paths.extend(collect_matching_json(root, "target/jankurai", |name| {
        name.contains("proof") && name.ends_with(".json")
    })?);
    Ok(dedup(paths))
}

fn collect_security_receipts(root: &Path) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    let candidate = "target/jankurai/security-evidence.json";
    if root.join(candidate).exists() {
        paths.push(candidate.to_string());
    }
    Ok(dedup(paths))
}

fn collect_ux_receipts(root: &Path) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    let candidate = "target/jankurai/ux-qa.json";
    if root.join(candidate).exists() {
        paths.push(candidate.to_string());
    }
    Ok(dedup(paths))
}

fn collect_contract_db_receipts(root: &Path) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    for candidate in [
        "target/jankurai/migration-report.json",
        "target/jankurai/migration-plan.json",
        "target/jankurai/contract-report.json",
        "examples/perfect-web-api-db/contracts/openapi.json",
        "examples/perfect-web-api-db/db/migrations/001_init.sql",
        "examples/perfect-web-api-db/db/constraints/001_accounts.sql",
    ] {
        if root.join(candidate).exists() {
            paths.push(candidate.to_string());
        }
    }
    Ok(dedup(paths))
}

fn collect_exceptions(root: &Path) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    if root.join("docs/exceptions").exists() {
        paths.push("docs/exceptions/".to_string());
    }
    let candidate = "examples/perfect-web-api-db/docs/exceptions.md";
    if root.join(candidate).exists() {
        paths.push(candidate.to_string());
    }
    Ok(dedup(paths))
}

fn collect_matching_json<F>(root: &Path, rel_dir: &str, predicate: F) -> Result<Vec<String>>
where
    F: Fn(&str) -> bool,
{
    let mut paths = Vec::new();
    let dir = root.join(rel_dir);
    if !dir.exists() {
        return Ok(paths);
    }
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if predicate(&file_name) {
            paths.push(format!("{rel_dir}/{file_name}"));
        }
    }
    Ok(dedup(paths))
}

fn dedup(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn render_markdown(certification: &Certification) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Certification");
    let _ = writeln!(out);
    let _ = writeln!(out, "- local evidence artifact: `true`");
    let _ = writeln!(out, "- repo: `{}`", certification.repo);
    let _ = writeln!(
        out,
        "- standard version: `{}`",
        certification.standard_version
    );
    let _ = writeln!(
        out,
        "- auditor version: `{}`",
        certification.auditor_version
    );
    let _ = writeln!(out, "- paper edition: `{}`", certification.paper_edition);
    let _ = writeln!(
        out,
        "- target stack ID: `{}`",
        certification.target_stack_id
    );
    let _ = writeln!(out, "- score: `{}`", certification.score);
    let _ = writeln!(out, "- conformance: `{}`", certification.conformance_level);
    let _ = writeln!(
        out,
        "- findings: critical=`{}` high=`{}` medium=`{}` low=`{}`",
        certification.findings_summary.critical,
        certification.findings_summary.high,
        certification.findings_summary.medium,
        certification.findings_summary.low
    );
    let _ = writeln!(
        out,
        "- proof receipts: `{}`",
        certification.proof_receipt_index.join(", ")
    );
    let _ = writeln!(
        out,
        "- security receipts: `{}`",
        certification.security_receipt_index.join(", ")
    );
    let _ = writeln!(
        out,
        "- ux receipts: `{}`",
        certification.ux_receipt_index.join(", ")
    );
    let _ = writeln!(
        out,
        "- contract/db receipts: `{}`",
        certification.contract_db_receipt_index.join(", ")
    );
    let _ = writeln!(
        out,
        "- exceptions: `{}`",
        certification.exceptions.join(", ")
    );
    let _ = writeln!(
        out,
        "- provenance: `{}`",
        certification.provenance.attestation
    );
    for note in &certification.findings_summary.notes {
        let _ = writeln!(out, "- note: `{}`", note);
    }
    out
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
