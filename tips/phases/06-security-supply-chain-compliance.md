# Phase 06: Security Supply Chain And Compliance Evidence

Status: complete
Owner: ops
Last reviewed: 2026-05-02
Parallel MCP candidate: yes

## Objective

Replace security theater with real, parseable security and supply-chain evidence. Jankurai should not merely say "run scanners"; it should orchestrate security lanes, normalize artifacts, and map failures to rules, owners, and repairs.

This phase also defines SOC-ready engineering evidence without claiming automatic compliance certification.

## Current State

Existing surfaces:

- `Justfile` has `security` with gitleaks, cargo audit, npm audit, syft, and zizmor commands.
- CI template includes `cargo audit` and `npm audit`.
- Audit detects some secret-like text, missing security lane, dependency scan markers, overbroad agency, prompt injection, and CI proof issues.
- Docs already distinguish security, supply chain, prompt injection, agency, and compliance evidence.
- **`jankurai security run`** executes `tools/security-lane.sh` (configurable with `--script`) under `bash -lc`, writes a combined log under `target/jankurai/security/`, and emits **schema-valid** evidence JSON (`schemas/security-evidence.schema.json`, default `target/jankurai/security/evidence.json`). **`--strict`** sets **`JANKURAI_SECURITY_STRICT=1`** for the child process. The envelope records wrapper exit code, timings, log path, and a single honest step row for the wrapper invocation (per-tool parsing is not claimed in v1).
- **`jankurai doctor`** validates that file when present and warns if `git_head` is stale versus current HEAD.
- **`jankurai audit`** (repo score JSON) ingests **`target/jankurai/security/evidence.json`** when present and schema-valid: **`security_evidence.artifact`** holds a compact summary (path, envelope exit code, timing, strict flag, command status counts, optional `generated_at` / `git_head`). Invalid or missing files leave **`artifact`** omitted. Implementation in `crates/jankurai/src/audit/security_artifact.rs`; tests in `crates/jankurai/tests/security_evidence_audit_ingest_smoke.rs`; **`schemas/repo-score.schema.json`** documents **`security_evidence`**. Score caps are unchanged.
- **`render_markdown`** and GitHub step summary print **`security_evidence.artifact`** when present (shared renderer with UX lane artifacts; `crates/jankurai/tests/render_lane_artifacts_smoke.rs`).

Gaps:

- Security tool availability is not consistently checked by doctor (optional tools remain advisory unless strict mode fails the lane).
- Security outputs are **partially** normalized: envelope only; per-scanner SARIF/CVE rollup is not in the evidence file yet.
- Many tools are not integrated: cargo deny, OSV, Grype, Trivy, CodeQL/Semgrep, actionlint, Scorecard, SLSA/Cosign.
- Compliance evidence folders and control maps are not fully operational.
- No `agent/security-policy.toml` in this slice; audit does not use envelope exit codes for numeric scoring.

## Dependencies

Requires Phase 01 stabilization.

Benefits from Phase 03 evidence ledger and Phase 02 rule metadata.

## Public Interface Changes

Target command:

```bash
jankurai security run [--repo .] [--script tools/security-lane.sh] [--out target/jankurai/security/evidence.json] [--strict]
```

If a standalone command is too large, first implement:

- security policy schema
- doctor checks for tools and for **security evidence JSON** at the default path
- evidence ingestion from existing `just security` / **`jankurai security run`**

Security policy fields:

- enabled tools
- required tools
- advisory tools
- severity thresholds
- artifact paths
- exception policy
- CI hardening rules
- deployable artifact requirements
- compliance evidence mapping

## Workstreams

### 1. Security Tool Matrix

Implementation tasks:

- Define the default security tool matrix by profile.
- Separate always-on checks from release-only checks.
- Avoid requiring every tool in local fast lanes.
- Add docs for installing required tools.

Default candidates:

- gitleaks for secrets
- cargo audit for Rust advisories
- cargo deny for Rust dependency, license, source policy
- npm audit or OSV for JS dependencies
- Syft for SBOM
- Grype or Trivy for vulnerability scanning
- actionlint for workflow correctness
- zizmor for GitHub Actions hardening
- Semgrep and/or CodeQL for SAST where CI supports it
- OpenSSF Scorecard for public repo posture
- SLSA/Cosign for release provenance and signing

Acceptance:

- Tool matrix says which tool runs locally, in PR, nightly, or release.
- Missing optional tools are advisory, not false failures.
- Missing required tools in strict modes are actionable.

### 2. Evidence Normalization

Implementation tasks:

- Define security evidence schema.
- Capture tool name, version if available, command, exit code, artifact path, finding count, highest severity, and normalized decision.
- Add adapters for at least the tools already in `Justfile`.
- Store evidence under `target/jankurai/security/`.
- Map normalized findings to Jankurai security rule IDs.

Acceptance:

- A failed secret scan produces a security finding with exact repair guidance.
- A missing SBOM in release mode is visible.
- Security evidence can be included in release evidence.

### 3. CI Hardening

Implementation tasks:

- Parse GitHub workflows for unsafe permissions, unpinned actions where policy requires pinning, broad secrets exposure, and echo-only proof.
- Integrate actionlint and zizmor outputs when present.
- Add rule docs for CI security findings.

Acceptance:

- Unsafe workflow permissions are findings.
- Untrusted PR contexts and secret exposure patterns are detected or tracked as gated follow-on work.
- CI hardening findings have owner and lane.

### 4. Prompt Injection And Agent Agency

Implementation tasks:

- Strengthen detection of trusted policy files that include bypass language or overbroad permissions.
- Define untrusted content zones.
- Add docs for tool-output and issue/comment prompt injection.
- Add permission profile schema for future agent execution lanes.

Acceptance:

- Policy files cannot instruct agents to ignore higher-priority instructions.
- Broad "allow all tools" style language is flagged.
- Findings explain safe repair, not just prohibition.

### 5. SOC-Ready Evidence Shell

Implementation tasks:

- Add docs or templates for engineering evidence categories:
  - change management
  - access control
  - vulnerability management
  - incident response
  - backup and restore
  - logging and monitoring
  - vendor/dependency risk
  - release approvals
- Map evidence categories to Jankurai lanes.
- Do not claim SOC 2 certification.

Acceptance:

- Docs state "SOC-ready evidence", not "SOC compliant by default".
- Evidence paths are clear and machine-readable where possible.

## Parallel MCP Breakdown

Strong parallel candidate:

- Agent A: tool matrix and docs. Owns docs/policy.
- Agent B: evidence schema and adapters for existing tools. Owns schemas and Rust security modules.
- Agent C: CI hardening checks. Owns workflow parsing and tests.
- Agent D: compliance evidence shell. Owns compliance docs/templates only.

Merge order:

1. Security evidence schema.
2. Tool adapters and CI checks.
3. Compliance mapping.
4. Docs final pass.

## Validation

Minimum:

```bash
just fast
cargo test -p jankurai
```

If tools are installed:

```bash
just security
```

Do not make success depend on optional local tools unless the phase explicitly adds setup requirements.

## Risks

- Tool availability varies by developer machine.
- Security scans can be slow or noisy.
- Compliance language can overpromise.
- Normalizing third-party scanner output can become brittle.

## Handoff Notes

Leave:

- tool matrix
- required versus optional tool list
- security evidence schema
- sample security report
- new rule IDs
- skipped tool rationale
- exact commands run

## Phase Status Receipt

- Phase status: complete — security policy schema and TOML added; evidence normalization expanded; CI hardening audit implemented; SOC-ready documentation established
- Operational handoff log: [`tips/phases/logs/06-security-supply-chain-compliance.log`](logs/06-security-supply-chain-compliance.log)
- Files changed (this slice): `schemas/security-policy.schema.json`, `agent/security-policy.toml`, `crates/jankurai/src/validation.rs`, `crates/jankurai/src/commands/doctor.rs`, `schemas/security-evidence.schema.json`, `crates/jankurai/src/commands/security.rs`, `crates/jankurai/src/audit/scan.rs`, `crates/jankurai/src/audit/rules.rs`, `crates/jankurai/src/audit/mod.rs`, `docs/security-tool-matrix.md`, `docs/soc-ready-evidence.md`
- Schemas changed: `security-policy.schema.json` created; `security-evidence.schema.json` expanded with finding counts and severities
- Public interfaces changed: `jankurai doctor` validates `agent/security-policy.toml`; `HLT-020-CI-HARDENING-GAP` audit rule established
- Generated artifacts: none new; evidence structure matured
- Validation commands: `cargo test -p jankurai`, `just fast`
- Results: CI tests use updated security policy safely; tests pass
- Follow-up phases: none remaining for this phase
