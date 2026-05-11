use jankurai::audit;
use jankurai::model::Finding;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn findings_for(repo: &Path, rule_id: &str) -> Vec<Finding> {
    audit::run_audit(repo, &[])
        .unwrap()
        .findings
        .into_iter()
        .filter(|finding| finding.rule_id.as_deref() == Some(rule_id))
        .collect()
}

fn assert_finding(repo: &Path, rule_id: &str, path: &str, evidence: &str, category: &str) {
    let findings = findings_for(repo, rule_id);
    assert_eq!(findings.len(), 1, "{rule_id} findings: {findings:?}");
    let finding = &findings[0];
    assert_eq!(finding.path, path);
    assert_eq!(finding.category, category);
    assert!(matches!(finding.severity.as_str(), "high" | "medium"));
    assert!(
        finding
            .evidence
            .iter()
            .any(|value| value.to_ascii_lowercase().contains(evidence)),
        "{rule_id} evidence missing `{evidence}`: {:?}",
        finding.evidence
    );
}

#[test]
fn hlt022_authz_isolation_requires_negative_owner_tests() {
    let risky = tempdir().unwrap();
    write(
        &risky.path().join("src/api.ts"),
        "export function readInvoice(owner_id: string) { return db.invoice.findMany({ where: { owner_id } }); }\n",
    );
    assert_finding(
        risky.path(),
        "HLT-022-AUTHZ-ISOLATION-GAP",
        "src/api.ts",
        "owner_id",
        "security",
    );

    let safe = tempdir().unwrap();
    write(
        &safe.path().join("src/api.ts"),
        "export function readInvoice(owner_id: string) { return db.invoice.findMany({ where: { owner_id } }); }\n",
    );
    write(
        &safe.path().join("tests/authz_test.rs"),
        "#[test]\nfn non_owner_is_forbidden() { assert!(\"non-owner forbidden\".contains(\"forbidden\")); }\n",
    );
    assert!(findings_for(safe.path(), "HLT-022-AUTHZ-ISOLATION-GAP").is_empty());
}

#[test]
fn hlt023_input_boundary_flags_unsafe_sinks_but_not_allowlisted_paths() {
    let risky = tempdir().unwrap();
    write(
        &risky.path().join("src/routes.ts"),
        "app.get('/x', (req, res) => { eval(req.query.expr); db.query(`SELECT * FROM users WHERE id = ${req.query.id}`); res.send(req.query.name); });\n",
    );
    assert_finding(
        risky.path(),
        "HLT-023-INPUT-BOUNDARY-GAP",
        "src/routes.ts",
        "eval",
        "security",
    );

    let edge = tempdir().unwrap();
    write(
        &edge.path().join("src/fetch.ts"),
        "const safe_url = allowlist.parse(req.query.url); await fetch(safe_url);\nconst row = await db.query('SELECT * FROM users WHERE id = $1', [id]); // parameterized\n",
    );
    assert!(findings_for(edge.path(), "HLT-023-INPUT-BOUNDARY-GAP").is_empty());

    let git = tempdir().unwrap();
    write(
        &git.path().join("src/run.ts"),
        "import { execFile } from 'node:child_process';\nconst cmd = Command::new(\"git\").args([\"status\"]);\n",
    );
    assert!(findings_for(git.path(), "HLT-023-INPUT-BOUNDARY-GAP").is_empty());
}

#[test]
fn hlt024_agent_tool_supply_flags_broad_unpinned_tool_configs() {
    let risky = tempdir().unwrap();
    write(
        &risky.path().join(".agents/mcp.toml"),
        "[[mcp]]\nname = \"repo-tools\"\nversion = \"latest\"\npermissions = \"all\"\nauto_run = true\n",
    );
    assert_finding(
        risky.path(),
        "HLT-024-AGENT-TOOL-SUPPLY-GAP",
        ".agents/mcp.toml",
        "latest",
        "security",
    );

    let safe = tempdir().unwrap();
    write(
        &safe.path().join(".agents/mcp.toml"),
        "[[mcp]]\nname = \"repo-tools\"\nversion = \"1.2.3\"\npermissions = [\"read\"]\nauto_run = false\n",
    );
    assert!(findings_for(safe.path(), "HLT-024-AGENT-TOOL-SUPPLY-GAP").is_empty());
}

#[test]
fn hlt025_release_readiness_requires_launch_gate_artifacts() {
    let risky = tempdir().unwrap();
    write(
        &risky.path().join("docs/release.md"),
        "Public production launch is approved after the demo.\n",
    );
    write(&risky.path().join("VERSION"), "1.0.0\n");
    write(
        &risky.path().join("CHANGELOG.md"),
        "# Changelog\n\n## 1.0.0\n",
    );
    write(
        &risky.path().join(".github/workflows/release.yml"),
        "name: release\non: workflow_dispatch\njobs:\n  release:\n    steps:\n      - run: echo release gate\n",
    );
    write(
        &risky.path().join("docs/provenance.md"),
        "Release artifacts carry sha256 checksum, SBOM, and provenance evidence.\n",
    );
    write(
        &risky.path().join("docs/rollback.md"),
        "Rollback guidance: restore the previous known-good version if release validation fails.\n",
    );
    assert_finding(
        risky.path(),
        "HLT-025-RELEASE-READINESS-GAP",
        "docs/testing.md",
        "release language",
        "release",
    );

    let safe = tempdir().unwrap();
    write(&safe.path().join("VERSION"), "1.0.0\n");
    write(
        &safe.path().join("CHANGELOG.md"),
        "# Changelog\n\n## 1.0.0\n",
    );
    write(
        &safe.path().join("docs/release.md"),
        "Release gate: backup restore proof, rollback plan, monitoring dashboard, and abuse rate limit receipts are required before production launch. Artifacts carry sha256 checksum, SBOM, and provenance evidence.\n",
    );
    write(
        &safe.path().join(".github/workflows/release.yml"),
        "name: release\non: workflow_dispatch\njobs:\n  release:\n    steps:\n      - run: echo release gate\n",
    );
    assert!(findings_for(safe.path(), "HLT-025-RELEASE-READINESS-GAP").is_empty());
}

#[test]
fn hlt025_release_readiness_requires_release_structure() {
    let risky = tempdir().unwrap();
    write(
        &risky.path().join("package.json"),
        r#"{"name":"ship-me","version":"1.0.0","scripts":{"release":"npm publish"}}"#,
    );

    assert_finding(
        risky.path(),
        "HLT-025-RELEASE-READINESS-GAP",
        "docs/release.md",
        "release structure",
        "release",
    );

    let safe = tempdir().unwrap();
    write(
        &safe.path().join("package.json"),
        r#"{"name":"ship-me","version":"1.0.0","scripts":{"release":"npm publish"}}"#,
    );
    write(
        &safe.path().join("CHANGELOG.md"),
        "# Changelog\n\n## 1.0.0\n",
    );
    write(
        &safe.path().join("docs/release.md"),
        "Release gate: backup restore proof, rollback plan, monitoring dashboard, and abuse rate limit receipts are required before production launch. Artifacts carry sha256 checksum, SBOM, provenance, and attestation evidence.\n",
    );
    write(
        &safe.path().join(".github/workflows/release.yml"),
        "name: release\non: workflow_dispatch\njobs:\n  release:\n    steps:\n      - run: npm publish\n",
    );

    assert!(findings_for(safe.path(), "HLT-025-RELEASE-READINESS-GAP").is_empty());
}

#[test]
fn hlt026_cost_budget_requires_quota_spend_cap_and_stop_conditions() {
    let risky = tempdir().unwrap();
    write(
        &risky.path().join("src/ai.ts"),
        "export async function loop(prompt: string) { return openai.responses.create({ input: prompt }); }\n",
    );
    assert_finding(
        risky.path(),
        "HLT-026-COST-BUDGET-GAP",
        "docs/testing.md",
        "budget",
        "release",
    );

    let safe = tempdir().unwrap();
    write(
        &safe.path().join("src/ai.ts"),
        "export async function call(prompt: string) { return openai.responses.create({ input: prompt }); }\n",
    );
    write(
        &safe.path().join("docs/cost.md"),
        "Budget policy: every paid API has quota checks, a spend cap, a kill switch, and a stop condition before retries.\n",
    );
    assert!(findings_for(safe.path(), "HLT-026-COST-BUDGET-GAP").is_empty());
}

#[test]
fn hlt027_human_review_evidence_rejects_claims_without_receipts() {
    let risky = tempdir().unwrap();
    write(
        &risky.path().join("docs/review.md"),
        "LGTM, tests passed (not run), ship it.\n",
    );
    assert_finding(
        risky.path(),
        "HLT-027-HUMAN-REVIEW-EVIDENCE-GAP",
        "docs/review.md",
        "lgtm",
        "proof",
    );

    let safe = tempdir().unwrap();
    write(
        &safe.path().join("docs/review.md"),
        "Review receipt: raw CI logs are attached under target/jankurai/receipts with command, exit code, and reviewer notes.\n",
    );
    assert!(findings_for(safe.path(), "HLT-027-HUMAN-REVIEW-EVIDENCE-GAP").is_empty());
}
