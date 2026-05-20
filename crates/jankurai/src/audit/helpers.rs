use crate::audit::prose;
use crate::model::*;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

pub const WEIGHTS: &[(&str, u32)] = &[
    ("Ownership and navigation surface", 13),
    ("Contract and boundary integrity", 13),
    ("Proof lanes and test routing", 12),
    ("Security and supply-chain posture", 12),
    ("Code shape and semantic surface", 12),
    ("Data truth and workflow safety", 8),
    ("Observability and repair evidence", 8),
    ("Context economy and agent instructions", 7),
    ("Jankurai tool adoption and CI replacement", 7),
    ("Python containment and polyglot hygiene", 4),
    ("Build speed signals", 4),
];

pub const ALLOWED_PYTHON_ROOTS: &[&str] = &[
    ".github",
    "benchmarks",
    "docs",
    "examples",
    "paper",
    "python/ai-service",
    "reference",
    "tests",
];

pub const OWNER_MAP_PREFIXES: &[(&str, &str)] = &[
    ("AGENTS.md", "agent"),
    ("README.md", "workspace"),
    ("Justfile", "workspace"),
    ("package.json", "workspace"),
    ("package-lock.json", "workspace"),
    (".github/", "ops"),
    ("agent/", "agent"),
    ("assets/", "paper"),
    ("docs/", "standard"),
    ("crates/", "tools"),
    ("packages/ux-qa/", "tools"),
    ("paper/", "paper"),
    ("reference/", "read-only"),
    ("tips/", "paper"),
    ("tools/", "tools"),
];

#[derive(Clone)]
pub struct AuditContext {
    pub root: std::path::PathBuf,
    pub all_files: Vec<FileInfo>,
    pub scope_files: Vec<FileInfo>,
    pub scope_paths: Vec<String>,
    pub self_audit: bool,
    pub boundary_reclassifications: Vec<BoundaryReclassification>,
    pub copy_code: Option<crate::audit::copy_code::CopyCodeReport>,
}

pub fn weight_for(name: &str) -> u32 {
    WEIGHTS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, w)| *w)
        .unwrap_or(0)
}

pub fn make_dim(
    name: &str,
    score: i32,
    evidence: Vec<String>,
    notes: Vec<String>,
) -> DimensionResult {
    let w = weight_for(name);
    DimensionResult {
        name: name.into(),
        weight: w,
        score: score.clamp(0, 100),
        weighted_points: (w as f64) * (score.clamp(0, 100) as f64) / 100.0,
        evidence,
        notes,
    }
}

// --- Surface predicates ---

pub fn has_root_agents(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|f| f.rel_path == "AGENTS.md")
}

pub fn has_one_command(ctx: &AuditContext) -> bool {
    let t = command_surface_text(ctx);
    t.contains("\ncheck:")
        || t.contains("\nfast:")
        || t.contains("\nsetup:")
        || t.contains("\nverify:")
        || t.contains("\ninstall:")
        || t.contains("\nbootstrap:")
}

pub fn has_fast_lane(ctx: &AuditContext) -> bool {
    real_command_surface_contains(
        ctx,
        &[
            "cargo check",
            "cargo nextest",
            "vitest",
            "pytest",
            "go test",
            "jankurai",
        ],
    )
}

pub fn has_security_lane(ctx: &AuditContext) -> bool {
    let t = security_lane_text(ctx);
    [
        "tools/security-lane.sh",
        "gitleaks detect",
        "dependency-review-action",
        "syft ",
        "grype ",
        "zizmor ",
        "cargo deny",
        "cargo audit",
        "npm audit",
    ]
    .iter()
    .any(|needle| t.contains(needle))
}

pub fn has_jankurai_audit_ci_lane(ctx: &AuditContext) -> bool {
    real_command_surface_contains(ctx, &["jankurai audit", "repo-score"])
        || real_command_surface_contains(ctx, &["cargo run -p jankurai", "repo-score"])
}

#[derive(Debug, Clone, Copy)]
pub struct ToolAdoptionCatalogEntry {
    pub id: &'static str,
    pub category: &'static str,
    pub replaced_tools: &'static [&'static str],
    pub local_command: &'static str,
    pub ci_command: &'static str,
    pub artifact_paths: &'static [&'static str],
    pub applicability: fn(&AuditContext) -> bool,
}

pub const TOOL_ADOPTION_CATALOG: &[ToolAdoptionCatalogEntry] = &[
    ToolAdoptionCatalogEntry {
        id: "audit-ci",
        category: "audit",
        replaced_tools: &["manual repo scoring", "ad hoc score gates"],
        local_command: "jankurai audit . --mode advisory --json .jankurai/repo-score.json --md .jankurai/repo-score.md",
        ci_command: "jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md",
        artifact_paths: &[".jankurai/repo-score.json", ".jankurai/repo-score.md"],
        applicability: tool_audit_ci_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "proof-routing",
        category: "proof",
        replaced_tools: &["ad hoc proof lane selection", "manual proof receipts"],
        local_command: "jankurai proof . --changed-from origin/main --out target/jankurai/proof-plan.json --md target/jankurai/proof-plan.md",
        ci_command: "jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md",
        artifact_paths: &[".jankurai/repo-score.json", ".jankurai/repo-score.md", "target/jankurai/repair-queue.jsonl"],
        applicability: tool_proof_routing_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "proofbind",
        category: "proof",
        replaced_tools: &["manual changed-surface routing", "ad hoc proof obligation lists"],
        local_command: "jankurai proofbind verify . --changed-from origin/main",
        ci_command: "jankurai proofbind verify . --changed-from origin/main",
        artifact_paths: &["target/jankurai/proofbind/surface-witness.json", "target/jankurai/proofbind/obligations.json"],
        applicability: tool_proof_routing_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "proofmark-rust",
        category: "proof",
        replaced_tools: &["line-only coverage review", "manual in-diff mutation review"],
        local_command: "jankurai proofmark rust . --obligations target/jankurai/proofbind/obligations.json",
        ci_command: "jankurai proofmark rust . --obligations target/jankurai/proofbind/obligations.json",
        artifact_paths: &["target/jankurai/proofmark/proofmark-receipt.json", "target/jankurai/proofmark/proof-receipt.json"],
        applicability: tool_rust_witness_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "copy-code",
        category: "audit",
        replaced_tools: &["ad hoc copy-code review", "manual duplication triage"],
        local_command: "cargo run -p jankurai -- copy-code . --json target/jankurai/copy-code.json --md target/jankurai/copy-code.md",
        ci_command: "cargo run -p jankurai -- copy-code . --json target/jankurai/copy-code.json --md target/jankurai/copy-code.md",
        artifact_paths: &["target/jankurai/copy-code.json", "target/jankurai/copy-code.md"],
        applicability: tool_copy_code_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "security",
        category: "security",
        replaced_tools: &["gitleaks", "dependency review", "SBOM/provenance"],
        local_command: "jankurai security run . --out target/jankurai/security/evidence.json",
        ci_command: "jankurai security run . --out target/jankurai/security/evidence.json",
        artifact_paths: &["target/jankurai/security/evidence.json"],
        applicability: tool_security_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "ci-bad-behavior",
        category: "security",
        replaced_tools: &[
            "mutable workflow refs",
            "secret echo/debug workflow checks",
            "non-blocking security scans",
        ],
        local_command: "cargo test -p jankurai --test language_bad_behavior",
        ci_command: "cargo test -p jankurai --test language_bad_behavior",
        artifact_paths: &["target/jankurai/language-bad-behavior.log"],
        applicability: tool_security_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "git-bad-behavior",
        category: "audit",
        replaced_tools: &[
            "destructive git automation",
            "force-push release scripts",
            "hidden stash-based state",
        ],
        local_command: "cargo test -p jankurai --test language_bad_behavior",
        ci_command: "cargo test -p jankurai --test language_bad_behavior",
        artifact_paths: &["target/jankurai/language-bad-behavior.log"],
        applicability: tool_security_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "release-bad-behavior",
        category: "release",
        replaced_tools: &[
            "manual release checklist",
            "ad hoc tag and artifact review",
            "manual provenance review",
        ],
        local_command: "cargo test -p jankurai --test language_bad_behavior",
        ci_command: "cargo test -p jankurai --test language_bad_behavior",
        artifact_paths: &["target/jankurai/language-bad-behavior.log"],
        applicability: tool_release_readiness_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "ux-qa",
        category: "ux",
        replaced_tools: &["playwright", "axe-core", "visual baselines"],
        local_command: "jankurai ux audit --config agent/ux-qa.toml --out target/jankurai/ux-qa.json",
        ci_command: "jankurai ux audit --config agent/ux-qa.toml --out target/jankurai/ux-qa.json",
        artifact_paths: &["target/jankurai/ux-qa.json"],
        applicability: tool_ux_qa_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "db-migration-analyze",
        category: "db",
        replaced_tools: &["manual migration review"],
        local_command: "jankurai migrate . --analyze --json target/jankurai/migration-report.json",
        ci_command: "jankurai migrate . --analyze --json target/jankurai/migration-report.json",
        artifact_paths: &["target/jankurai/migration-report.json"],
        applicability: tool_db_migration_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "contract-drift",
        category: "contract",
        replaced_tools: &["handwritten contract drift checks", "openapi diff"],
        local_command: "jankurai audit . --mode advisory --json .jankurai/repo-score.json --md .jankurai/repo-score.md",
        ci_command: "jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md",
        artifact_paths: &[".jankurai/repo-score.json", ".jankurai/repo-score.md"],
        applicability: tool_contract_drift_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "rust-witness",
        category: "rust",
        replaced_tools: &["manual witness graphing"],
        local_command: "jankurai rust witness build .",
        ci_command: "jankurai rust witness build .",
        artifact_paths: &["target/jankurai/rust/witness-graph.json"],
        applicability: tool_rust_witness_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "vibe-coverage",
        category: "audit",
        replaced_tools: &["manual vibe-coding coverage spreadsheet"],
        local_command: "jankurai vibe coverage --source agent/vibe-coverage.toml --tips tips/vibe_coding --json target/jankurai/vibe-coverage.json --md target/jankurai/vibe-coverage.md",
        ci_command: "jankurai vibe coverage --source agent/vibe-coverage.toml --tips tips/vibe_coding --json target/jankurai/vibe-coverage.json --md target/jankurai/vibe-coverage.md",
        artifact_paths: &["target/jankurai/vibe-coverage.json", "target/jankurai/vibe-coverage.md"],
        applicability: tool_vibe_coverage_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "coverage-evidence",
        category: "proof",
        replaced_tools: &["manual coverage report review", "ad hoc mutation survivor review"],
        local_command: "jankurai coverage audit . --config agent/coverage-sources.toml --json target/jankurai/coverage/coverage-audit.json --md target/jankurai/coverage/coverage-audit.md",
        ci_command: "jankurai coverage audit . --config agent/coverage-sources.toml --json target/jankurai/coverage/coverage-audit.json --md target/jankurai/coverage/coverage-audit.md",
        artifact_paths: &["target/jankurai/coverage/coverage-audit.json", "target/jankurai/coverage/coverage-audit.md"],
        applicability: tool_coverage_evidence_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "authz-matrix",
        category: "security",
        replaced_tools: &["manual authz matrix review"],
        local_command: "jankurai audit . --mode advisory --json .jankurai/repo-score.json --md .jankurai/repo-score.md",
        ci_command: "jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md",
        artifact_paths: &[".jankurai/repo-score.json", ".jankurai/repo-score.md"],
        applicability: tool_authz_matrix_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "input-boundary",
        category: "security",
        replaced_tools: &["manual unsafe sink review"],
        local_command: "jankurai audit . --mode advisory --json .jankurai/repo-score.json --md .jankurai/repo-score.md",
        ci_command: "jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md",
        artifact_paths: &[".jankurai/repo-score.json", ".jankurai/repo-score.md"],
        applicability: tool_input_boundary_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "agent-tool-supply",
        category: "security",
        replaced_tools: &["manual MCP/tool trust review"],
        local_command: "jankurai audit . --mode advisory --json .jankurai/repo-score.json --md .jankurai/repo-score.md",
        ci_command: "jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md",
        artifact_paths: &[".jankurai/repo-score.json", ".jankurai/repo-score.md"],
        applicability: tool_agent_tool_supply_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "release-readiness",
        category: "release",
        replaced_tools: &["manual launch checklist"],
        local_command: "jankurai audit . --mode advisory --json .jankurai/repo-score.json --md .jankurai/repo-score.md",
        ci_command: "jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md",
        artifact_paths: &[".jankurai/repo-score.json", ".jankurai/repo-score.md"],
        applicability: tool_release_readiness_applicable,
    },
    ToolAdoptionCatalogEntry {
        id: "cost-budget",
        category: "release",
        replaced_tools: &["manual spend review"],
        local_command: "jankurai audit . --mode advisory --json .jankurai/repo-score.json --md .jankurai/repo-score.md",
        ci_command: "jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md",
        artifact_paths: &[".jankurai/repo-score.json", ".jankurai/repo-score.md"],
        applicability: tool_cost_budget_applicable,
    },
];

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ToolAdoptionMode {
    Auto,
    Required,
    Advisory,
    Disabled,
}

impl ToolAdoptionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Required => "required",
            Self::Advisory => "advisory",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolAdoptionConfigItem {
    pub id: String,
    pub mode: ToolAdoptionMode,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolAdoptionConfigFile {
    #[serde(default)]
    pub schema_version: String,
    #[serde(default)]
    pub tools: Vec<ToolAdoptionConfigItem>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolAdoptionConfig {
    pub present: bool,
    pub modes: HashMap<String, ToolAdoptionMode>,
}

impl ToolAdoptionConfig {
    pub fn mode_for(&self, id: &str) -> ToolAdoptionMode {
        self.modes
            .get(id)
            .copied()
            .unwrap_or(ToolAdoptionMode::Auto)
    }

    pub fn has_entry(&self, id: &str) -> bool {
        self.modes.contains_key(id)
    }
}

pub fn load_tool_adoption_config(root: &std::path::Path) -> ToolAdoptionConfig {
    let path = root.join("agent/tool-adoption.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return ToolAdoptionConfig::default();
    };
    let Ok(parsed) = toml::from_str::<ToolAdoptionConfigFile>(&text) else {
        return ToolAdoptionConfig::default();
    };
    ToolAdoptionConfig {
        present: true,
        modes: parsed
            .tools
            .into_iter()
            .map(|item| (item.id, item.mode))
            .collect(),
    }
}

pub fn tool_adoption_control_plane_present(ctx: &AuditContext) -> bool {
    has_root_agents(ctx)
        || ctx
            .all_files
            .iter()
            .any(|f| f.rel_path == "agent/test-map.json" || f.rel_path == "agent/proof-lanes.toml")
}

pub fn github_workflow_text(ctx: &AuditContext) -> String {
    let mut text = String::new();
    for file in ctx
        .all_files
        .iter()
        .filter(|f| f.rel_path.starts_with(".github/workflows/"))
    {
        text.push('\n');
        text.push_str(&file.text.to_ascii_lowercase());
    }
    text
}

pub fn tool_adoption_upload_text(ctx: &AuditContext) -> String {
    github_workflow_text(ctx)
}

fn tool_audit_ci_applicable(ctx: &AuditContext) -> bool {
    is_high_risk_repo(ctx)
}

fn tool_proof_routing_applicable(ctx: &AuditContext) -> bool {
    tool_adoption_control_plane_present(ctx)
}

fn tool_security_applicable(ctx: &AuditContext) -> bool {
    is_high_risk_repo(ctx)
        || ctx.all_files.iter().any(|f| {
            matches!(
                f.name.as_str(),
                "Cargo.toml" | "package.json" | "Cargo.lock" | "package-lock.json"
            )
        })
}

fn tool_ux_qa_applicable(ctx: &AuditContext) -> bool {
    has_web_surface(ctx)
}

fn tool_db_migration_applicable(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|f| {
        (f.rel_path.starts_with("db/migrations/") || f.rel_path.starts_with("migrations/"))
            && f.suffix == ".sql"
    })
}

fn tool_contract_drift_applicable(ctx: &AuditContext) -> bool {
    has_contract_surface(ctx)
        || has_generated_contracts(ctx)
        || ctx
            .all_files
            .iter()
            .any(|f| f.rel_path == "agent/generated-zones.toml")
}

fn tool_rust_witness_applicable(ctx: &AuditContext) -> bool {
    has_rust_surface(ctx)
}

fn tool_copy_code_applicable(ctx: &AuditContext) -> bool {
    product_code_files(ctx).iter().any(|f| f.is_code)
}

fn tool_vibe_coverage_applicable(ctx: &AuditContext) -> bool {
    ctx.all_files
        .iter()
        .any(|f| f.rel_path == "agent/vibe-coverage.toml")
}

fn tool_coverage_evidence_applicable(ctx: &AuditContext) -> bool {
    ctx.all_files
        .iter()
        .any(|f| f.rel_path == "agent/coverage-sources.toml")
}

fn tool_authz_matrix_applicable(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|f| {
        prose::allows_word_scan(f) && {
            let lower = f.text.to_ascii_lowercase();
            lower.contains("auth")
                || lower.contains("owner_id")
                || lower.contains("tenant_id")
                || lower.contains("rls")
        }
    })
}

fn tool_input_boundary_applicable(ctx: &AuditContext) -> bool {
    product_code_files(ctx).iter().any(|f| {
        let lower = f.text.to_ascii_lowercase();
        lower.contains("eval(")
            || lower.contains("exec(")
            || lower.contains("fetch(")
            || lower.contains("innerhtml")
            || lower.contains("select * from")
    })
}

fn tool_agent_tool_supply_applicable(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|f| {
        f.rel_path.starts_with("agent/")
            || f.rel_path.starts_with(".agents/")
            || f.rel_path.starts_with(".cursor/")
    })
}

fn tool_release_readiness_applicable(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|f| {
        prose::allows_word_scan(f) && {
            let lower = f.text.to_ascii_lowercase();
            lower.contains("release") || lower.contains("launch") || lower.contains("rollback")
        }
    })
}

fn tool_cost_budget_applicable(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|f| {
        prose::allows_word_scan(f) && {
            let lower = f.text.to_ascii_lowercase();
            lower.contains("budget") || lower.contains("quota") || lower.contains("spend")
        }
    })
}

pub fn is_high_risk_repo(ctx: &AuditContext) -> bool {
    if product_code_files(ctx).iter().any(|f| f.is_code) {
        return true;
    }
    let manifests: Vec<&FileInfo> = ctx
        .all_files
        .iter()
        .filter(|f| ["package.json", "Cargo.toml", "go.mod"].contains(&f.name.as_str()))
        .collect();
    if manifests.is_empty() {
        return false;
    }
    // If every manifest in the inventory is gitignored (e.g. a runtime install
    // directory like `.jekko/package.json`), don't treat the repo as high-risk
    // for missing supply-chain tooling on its behalf.
    let ignored = build_repo_gitignore(&ctx.root);
    manifests
        .iter()
        .any(|file| !path_is_gitignored(&ignored, &file.rel_path))
}

fn build_repo_gitignore(root: &std::path::Path) -> Option<ignore::gitignore::Gitignore> {
    let candidate = root.join(".gitignore");
    if !candidate.exists() {
        return None;
    }
    let mut builder = ignore::gitignore::GitignoreBuilder::new(root);
    if builder.add(&candidate).is_some() {
        return None;
    }
    builder.build().ok()
}

fn path_is_gitignored(ignored: &Option<ignore::gitignore::Gitignore>, rel_path: &str) -> bool {
    let Some(matcher) = ignored.as_ref() else {
        return false;
    };
    let stripped = rel_path.trim_start_matches('/');
    matcher.matched(stripped, false).is_ignore() || matcher.matched(stripped, true).is_ignore()
}

pub fn has_contract_surface(ctx: &AuditContext) -> bool {
    has_prefix(ctx, "contracts")
        || ctx.all_files.iter().any(|f| {
            prose::allows_word_scan(f)
                && (f.text.contains("openapi")
                    || f.text.contains("protobuf")
                    || f.suffix == ".proto")
        })
}

pub fn has_polyglot_boundary(ctx: &AuditContext) -> bool {
    has_prefix(ctx, "apps/web")
        || has_prefix(ctx, "apps/api")
        || has_prefix(ctx, "crates/domain")
        || has_prefix(ctx, "crates/application")
        || has_prefix(ctx, "crates/adapters")
}

pub fn has_generated_contracts(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|f| {
        prose::allows_word_scan(f)
            && (f.rel_path == "agent/generated-zones.toml"
                || f.rel_path.contains("/generated/")
                || f.text.contains("Generated by:"))
    })
}

pub fn has_api_drift_checks(ctx: &AuditContext) -> bool {
    let t = command_surface_text(ctx);
    t.contains("cargo public-api")
        || t.contains("cargo semver")
        || t.contains("api-extractor")
        || t.contains("tsd")
        || t.contains("openapi-diff")
}

pub fn has_secret_or_dependency_scans(ctx: &AuditContext) -> bool {
    let t = security_lane_text(ctx);
    [
        "tools/security-lane.sh",
        "gitleaks",
        "dependency-review",
        "syft",
        "grype",
        "cargo audit",
        "npm audit",
    ]
    .iter()
    .any(|needle| t.contains(needle))
}

pub fn has_playwright_e2e(ctx: &AuditContext) -> bool {
    if !has_web_surface(ctx) {
        return true;
    }
    real_command_surface_contains(ctx, &["playwright"])
        || ctx.all_files.iter().any(|f| {
            f.rel_path.contains("e2e")
                || (prose::allows_word_scan(f) && f.text.contains("@playwright/test"))
        })
}

pub fn has_rust_surface(ctx: &AuditContext) -> bool {
    ctx.all_files
        .iter()
        .any(|f| f.suffix == ".rs" && is_runtime_stack_surface(f, ctx.self_audit))
}

pub fn has_rust_property_tests(ctx: &AuditContext) -> bool {
    !has_rust_surface(ctx)
        || ctx.all_files.iter().any(|f| {
            f.text.contains("proptest")
                || f.text.contains("quickcheck")
                || f.text.contains("rstest")
        })
}

pub fn has_rust_integration_tests(ctx: &AuditContext) -> bool {
    !has_rust_surface(ctx)
        || ctx.all_files.iter().any(|f| {
            f.suffix == ".rs"
                && (f.rel_path.contains("/tests/")
                    || f.rel_path.starts_with("tests/")
                    || f.text.contains("#[test]")
                    || f.text.contains("#[tokio::test]"))
        })
}

pub fn has_web_surface(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|f| {
        let manifest_web_hint = (f.rel_path == "package.json"
            || f.rel_path.ends_with("/package.json"))
            && (f.text.contains("\"react\"")
                || f.text.contains("\"vite\"")
                || f.text.contains("\"storybook\""));
        !f.rel_path.starts_with("docs/")
            && !f.rel_path.starts_with("paper/")
            && !f.rel_path.starts_with("reference/")
            && !f.rel_path.starts_with("tips/")
            && !f.rel_path.starts_with("agent/")
            && (f.rel_path.starts_with("apps/web")
                || f.rel_path.starts_with("frontend")
                || f.rel_path.starts_with("ui")
                || f.rel_path.starts_with("packages/web")
                || f.rel_path.starts_with("packages/ui")
                || manifest_web_hint)
    })
}

pub fn has_agent_friendly_exceptions(ctx: &AuditContext) -> bool {
    product_files(ctx).iter().any(|f| {
        let lower = f.text.to_ascii_lowercase();
        let shape = lower.contains("thiserror")
            || lower.contains("enum error")
            || lower.contains("extends error")
            || lower.contains("exception");
        let markers = [
            "purpose",
            "reason",
            "common fixes",
            "docs_url",
            "repair_hint",
        ]
        .iter()
        .filter(|m| lower.contains(**m))
        .count();
        shape && markers >= 3
    }) || has_agent_friendly_exception_docs(&observability_docs_text(ctx))
}

pub fn has_agent_friendly_exception_docs(text: &str) -> bool {
    text.contains("purpose")
        && text.contains("reason")
        && text.contains("common fixes")
        && (text.contains("docs_url") || text.contains("repair_hint"))
}

pub fn has_prefix(ctx: &AuditContext, prefix: &str) -> bool {
    ctx.all_files.iter().any(|f| {
        f.rel_path == prefix
            || f.rel_path
                .starts_with(&format!("{}/", prefix.trim_end_matches('/')))
    })
}

// --- Text aggregation ---

pub fn command_surface_text(ctx: &AuditContext) -> String {
    super::evidence::operational_command_text(&ctx.all_files)
}

pub fn real_command_surface_contains(ctx: &AuditContext, needles: &[&str]) -> bool {
    super::evidence::operational_command_lines(&ctx.all_files)
        .into_iter()
        .any(|line| {
            needles
                .iter()
                .any(|needle| line.contains(&needle.to_ascii_lowercase()))
        })
}

pub fn security_lane_text(ctx: &AuditContext) -> String {
    let mut text = command_surface_text(ctx);
    if let Some(script) = ctx
        .all_files
        .iter()
        .find(|f| f.rel_path == "tools/security-lane.sh")
    {
        text.push('\n');
        text.push_str(&script.text.to_ascii_lowercase());
    }
    text
}

pub fn observability_docs_text(ctx: &AuditContext) -> String {
    let mut text = String::new();
    for path in [
        "docs/testing.md",
        "docs/release-plan.md",
        "docs/agent-native-standard.md",
        "agent/JANKURAI_STANDARD.md",
    ] {
        if let Some(file) = ctx.all_files.iter().find(|f| f.rel_path == path) {
            if prose::allows_word_scan(file) {
                text.push('\n');
                text.push_str(&file.text.to_ascii_lowercase());
            }
        }
    }
    text
}

pub fn db_policy_text(ctx: &AuditContext) -> String {
    let mut text = String::new();
    for path in [
        "db/README.md",
        "db/migrations/README.md",
        "db/constraints/README.md",
    ] {
        if let Some(file) = ctx.all_files.iter().find(|f| f.rel_path == path) {
            if prose::allows_word_scan(file) {
                text.push('\n');
                text.push_str(&file.text.to_ascii_lowercase());
            }
        }
    }
    text
}

// --- File classification ---

pub fn is_runtime_stack_surface(file: &FileInfo, self_audit: bool) -> bool {
    !file.is_generated
        && !matches!(
            file.name.as_str(),
            "AGENTS.md"
                | "CLAUDE.md"
                | "Cargo.lock"
                | "Cargo.toml"
                | "GEMINI.md"
                | ".gitignore"
                | ".latexmkrc"
                | "Justfile"
                | "Makefile"
                | "README.md"
                | "VERSION"
                | "makefile"
                | "justfile"
                | "package-lock.json"
                | "package.json"
        )
        && !file.rel_path.starts_with("agent/")
        && !file.rel_path.starts_with(".claude/")
        && !file.rel_path.starts_with(".cursor/")
        && !file.rel_path.starts_with(".agents/")
        && !file.rel_path.starts_with("assets/")
        && (self_audit || !file.rel_path.starts_with("crates/jankurai/"))
        && !file.rel_path.starts_with("schemas/")
        && !["contracts/", "db/", "migrations/", "ops/", "/.github/"]
            .iter()
            .any(|p| file.rel_path.starts_with(p))
        && !file.rel_path.starts_with("docs/")
        && !file.rel_path.starts_with(".github/")
        && !file.rel_path.starts_with("paper/")
        && !file.rel_path.starts_with("reference/")
        && !file.rel_path.starts_with("examples/")
        && !file.rel_path.starts_with("labs/")
        && (self_audit || !file.rel_path.starts_with("packages/ux-qa/"))
        && !file.rel_path.starts_with("scripts/")
        && !file.rel_path.starts_with("tests/")
        && !file.rel_path.starts_with("tips/")
        && !file.rel_path.starts_with("tools/")
        && !crate::audit::scan::is_test_or_example_path(&file.rel_path)
}

pub fn product_files(ctx: &AuditContext) -> Vec<FileInfo> {
    ctx.all_files
        .iter()
        .filter(|f| is_runtime_stack_surface(f, ctx.self_audit))
        .cloned()
        .collect()
}

pub fn product_code_files(ctx: &AuditContext) -> Vec<FileInfo> {
    product_files(ctx)
        .into_iter()
        .filter(|f| f.is_code)
        .collect()
}

pub fn python_ratio(ctx: &AuditContext) -> f64 {
    let files = product_files(ctx)
        .into_iter()
        .filter(|f| {
            !accepted_boundary_file_for_cap(ctx, &f.rel_path, TOO_MUCH_PYTHON_CAP)
                && !python_scoring_exempt(ctx, &f.rel_path)
        })
        .collect::<Vec<_>>();
    let total: usize = files.iter().map(|f| f.line_count).sum();
    let py: usize = files
        .iter()
        .filter(|f| f.suffix == ".py")
        .map(|f| f.line_count)
        .sum();
    if total == 0 {
        0.0
    } else {
        py as f64 / total as f64
    }
}

pub fn is_allowed_python_path(path: &str) -> bool {
    ALLOWED_PYTHON_ROOTS
        .iter()
        .any(|p| path == *p || path.starts_with(&format!("{}/", p.trim_end_matches('/'))))
}

pub fn is_allowed_non_product_python_path(ctx: &AuditContext, path: &str) -> bool {
    if is_allowed_python_path(path) {
        return true;
    }
    let Some(manifest) = boundary_manifest(ctx) else {
        return false;
    };
    let Some(python) = manifest.python else {
        return false;
    };
    python.allowed_non_product_paths.iter().any(|prefix| {
        let prefix = prefix.trim_end_matches('/');
        path == prefix || path.starts_with(&format!("{prefix}/"))
    })
}

pub fn python_scoring_exempt(ctx: &AuditContext, path: &str) -> bool {
    is_allowed_non_product_python_path(ctx, path) || path_in_generated_zone(ctx, path)
}

pub fn bad_python_paths(ctx: &AuditContext) -> bool {
    !bad_python_path_hits(ctx).is_empty()
}

pub fn bad_python_path_hits(ctx: &AuditContext) -> Vec<FileInfo> {
    ctx.all_files
        .iter()
        .filter(|f| {
            f.suffix == ".py"
                && !python_scoring_exempt(ctx, &f.rel_path)
                && !f.rel_path.contains("/tests/fixtures/")
                && !f.rel_path.starts_with("tests/fixtures/")
                && !accepted_boundary_file_for_cap(ctx, &f.rel_path, PYTHON_DIRECT_CAP)
        })
        .cloned()
        .collect()
}

pub fn max_loc(files: &[FileInfo]) -> Option<usize> {
    files.iter().map(|f| f.line_count).max()
}

pub fn largest_file(files: &[FileInfo]) -> Option<FileInfo> {
    files.iter().max_by_key(|f| f.line_count).cloned()
}

pub fn root_readme_routes(ctx: &AuditContext) -> bool {
    ctx.all_files.iter().any(|f| f.rel_path == "README.md")
}

pub fn missing_core_docs(ctx: &AuditContext) -> Vec<String> {
    let present: HashSet<_> = ctx.all_files.iter().map(|f| f.rel_path.as_str()).collect();
    let mut missing = vec![];
    for need in ["AGENTS.md", "README.md"] {
        if !present.contains(need) {
            missing.push(need.into());
        }
    }
    if !(present.contains("docs/architecture.md") || present.contains("docs/boundaries.md")) {
        missing.push("docs/architecture.md or docs/boundaries.md".into());
    }
    if (has_web_surface(ctx) || has_rust_surface(ctx)) && !present.contains("docs/testing.md") {
        missing.push("docs/testing.md".into());
    }
    missing
}

pub fn boundary_manifest(
    ctx: &AuditContext,
) -> Option<crate::boundaries::manifest::BoundaryManifest> {
    crate::boundaries::manifest::load(&ctx.root.join("agent/boundaries.toml")).ok()
}

/// Loads the trimmed `path` field of every `[[zone]]` declared in
/// `agent/generated-zones.toml`. Returns an empty vector when the manifest is
/// absent or unparseable. Both `read_only=true` and `read_only=false` zones are
/// returned because either marks the file as generated/derived rather than
/// authored runtime code.
pub fn generated_zone_paths(ctx: &AuditContext) -> Vec<String> {
    let path = ctx.root.join("agent/generated-zones.toml");
    if !path.exists() {
        return vec![];
    }
    let Ok(text) = std::fs::read_to_string(&path) else {
        return vec![];
    };
    let Ok(file) = toml::from_str::<crate::commands::context_data::GeneratedZonesFile>(&text)
    else {
        return vec![];
    };
    file.zone
        .into_iter()
        .map(|zone| zone.path.trim().to_string())
        .filter(|zone_path| !zone_path.is_empty())
        .collect()
}

/// Returns the generated-zone paths that are safe to use as suppression hints
/// for language scans. Paths that point at protected control-plane or source
/// roots stay visible to the auditor even if they are declared here.
pub fn generated_zone_suppression_paths(ctx: &AuditContext) -> Vec<String> {
    generated_zone_paths(ctx)
        .into_iter()
        .filter(|zone| !crate::audit::fs::is_generated_zone_protected_path(zone))
        .collect()
}

/// Returns generated-zone paths that target protected source or control-plane
/// roots. These declarations are always suspicious because they can hide files
/// that should remain audit-visible.
pub fn generated_zone_protected_paths(ctx: &AuditContext) -> Vec<String> {
    generated_zone_paths(ctx)
        .into_iter()
        .filter(|zone| crate::audit::fs::is_generated_zone_protected_path(zone))
        .collect()
}

/// Returns true when `rel_path` matches any declared `[[zone]] path` from the
/// generated-zones manifest. Matches both exact paths and directory prefixes,
/// honoring trailing-slash semantics in `path_matches_prefix`.
pub fn path_in_generated_zone(ctx: &AuditContext, rel_path: &str) -> bool {
    let zones = generated_zone_suppression_paths(ctx);
    if zones.is_empty() {
        return false;
    }
    zones.iter().any(|zone| path_matches_prefix(rel_path, zone))
}

pub fn path_matches_prefix(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

pub fn paths_with(ctx: &AuditContext, path_markers: &[&str], markers: &[&str]) -> Vec<String> {
    let mut out = vec![];
    for f in &ctx.all_files {
        if f.rel_path.starts_with("docs/")
            || f.rel_path.starts_with("paper/")
            || f.rel_path.starts_with("reference/")
            || f.rel_path.starts_with("tips/")
        {
            continue;
        }
        let rel = f.rel_path.to_ascii_lowercase();
        let text = f.text.to_ascii_lowercase();
        let path_hit = path_markers
            .iter()
            .any(|m| rel.contains(&m.to_ascii_lowercase()));
        let text_hit = prose::allows_word_scan(f)
            && markers
                .iter()
                .any(|m| text.contains(&m.to_ascii_lowercase()));
        if path_hit || text_hit {
            out.push(f.rel_path.clone());
        }
        if out.len() >= 5 {
            break;
        }
    }
    out
}

// --- Repair exception ---

#[derive(Debug, Clone)]
pub struct AuditRepairException {
    pub purpose: &'static str,
    pub reason: &'static str,
    pub common_fixes: &'static [&'static str],
    pub docs_url: &'static str,
    pub repair_hint: &'static str,
}

pub fn audit_repair_exception() -> AuditRepairException {
    AuditRepairException {
        purpose: "route repair work to the next agent",
        reason: "opaque failures slow local debugging and reruns",
        common_fixes: &[
            "add a typed repair hint",
            "name the common fixes",
            "point at the local docs URL",
        ],
        docs_url: "docs/testing.md",
        repair_hint: "define a typed exception surface with purpose, reason, common fixes, docs_url, and repair_hint so the next rerun is local",
    }
}

// --- Non-optimal language detection ---

pub fn non_optimal_language_hits(ctx: &AuditContext) -> Vec<FileInfo> {
    product_code_files(ctx)
        .into_iter()
        .filter(|f| {
            !accepted_boundary_file_for_cap(ctx, &f.rel_path, NON_OPTIMAL_LANGUAGE_CAP)
                && ([
                    ".c", ".cc", ".cpp", ".cs", ".dart", ".ex", ".exs", ".go", ".h", ".hh", ".hpp",
                    ".java", ".js", ".jsx", ".kt", ".kts", ".lua", ".m", ".mm", ".php", ".rb",
                    ".scala", ".swift",
                ]
                .contains(&f.suffix.as_str())
                    || (f.suffix == ".py" && !python_scoring_exempt(ctx, &f.rel_path)))
        })
        .collect()
}

pub const PYTHON_DIRECT_CAP: &str = "python-direct-product-truth-or-db-ownership";
pub const NON_OPTIMAL_LANGUAGE_CAP: &str = "non-optimal-product-language-found";
pub const TOO_MUCH_PYTHON_CAP: &str = "too-much-python-in-product-surface";
pub const BOUNDARY_RECLASSIFICATION_GAP_CAP: &str = "boundary-reclassification-evidence-gap";

pub const PYTHON_STACK_RECLASSIFY_CAPS: &[&str] = &[
    PYTHON_DIRECT_CAP,
    NON_OPTIMAL_LANGUAGE_CAP,
    TOO_MUCH_PYTHON_CAP,
];

pub fn accepted_boundary_file_for_cap(ctx: &AuditContext, rel_path: &str, cap: &str) -> bool {
    ctx.boundary_reclassifications.iter().any(|boundary| {
        boundary.status == "passed"
            && boundary
                .reclassified_caps
                .iter()
                .any(|declared| declared == cap)
            && boundary.covered_files.iter().any(|path| path == rel_path)
    })
}

pub fn suppressing_boundary_file_for_cap(ctx: &AuditContext, rel_path: &str, cap: &str) -> bool {
    ctx.boundary_reclassifications.iter().any(|boundary| {
        boundary.suppresses_python_stack_caps
            && boundary
                .reclassified_caps
                .iter()
                .any(|declared| declared == cap)
            && boundary.covered_files.iter().any(|path| path == rel_path)
    })
}

pub fn all_files_suppressed_for_cap(ctx: &AuditContext, files: &[FileInfo], cap: &str) -> bool {
    !files.is_empty()
        && files
            .iter()
            .all(|file| suppressing_boundary_file_for_cap(ctx, &file.rel_path, cap))
}

pub fn python_ratio_cap_suppressed(ctx: &AuditContext) -> bool {
    let python_files = product_files(ctx)
        .into_iter()
        .filter(|f| {
            f.suffix == ".py"
                && !accepted_boundary_file_for_cap(ctx, &f.rel_path, TOO_MUCH_PYTHON_CAP)
        })
        .collect::<Vec<_>>();
    all_files_suppressed_for_cap(ctx, &python_files, TOO_MUCH_PYTHON_CAP)
}

pub fn has_boundary_reclassification_gap(ctx: &AuditContext) -> bool {
    ctx.boundary_reclassifications
        .iter()
        .any(|boundary| boundary.status != "passed" && !boundary.reclassified_caps.is_empty())
}

pub fn all_scope_python_files_are_accepted_boundaries(ctx: &AuditContext) -> bool {
    let python_files = ctx
        .scope_files
        .iter()
        .filter(|file| file.suffix == ".py" && !python_scoring_exempt(ctx, &file.rel_path))
        .collect::<Vec<_>>();
    !python_files.is_empty()
        && python_files.iter().all(|file| {
            PYTHON_STACK_RECLASSIFY_CAPS
                .iter()
                .any(|cap| accepted_boundary_file_for_cap(ctx, &file.rel_path, cap))
        })
}

// --- Handwritten API detection ---

pub fn handwritten_api_hits(ctx: &AuditContext) -> bool {
    product_code_files(ctx).iter().any(|f| {
        (f.rel_path.starts_with("apps/web/")
            || f.rel_path.starts_with("frontend/")
            || f.rel_path.starts_with("ui/")
            || f.rel_path.starts_with("src/"))
            && (f.text.contains("fetch(")
                || f.text.contains("axios.")
                || f.text.contains("XMLHttpRequest")
                || f.text.contains("Request") && f.text.contains("Response"))
    })
}

// --- Domain IO detection ---

pub fn domain_io_hits(ctx: &AuditContext) -> Vec<String> {
    product_code_files(ctx)
        .into_iter()
        .filter(|f| f.rel_path.contains("/core/") || f.rel_path.contains("/domain/"))
        .filter(|f| {
            [
                "fetch(",
                "open(",
                "read(",
                "requests.",
                "socket",
                "std::fs",
                "subprocess",
                "write(",
                "println!",
            ]
            .iter()
            .any(|m| f.text.contains(m))
        })
        .map(|f| f.rel_path)
        .collect()
}

pub fn weak_name_hits(_ctx: &AuditContext) -> Vec<String> {
    vec![]
}

pub fn missing_owner_paths(ctx: &AuditContext) -> Vec<String> {
    let Some(map) = load_owner_map(ctx) else {
        return vec![];
    };
    auditable_manifest_paths(ctx)
        .into_iter()
        .filter(|path| !matches_prefix(path, map.owners.keys()))
        .collect()
}

pub fn missing_test_paths(ctx: &AuditContext) -> Vec<String> {
    let Some(map) = load_test_map(ctx) else {
        return vec![];
    };
    auditable_manifest_paths(ctx)
        .into_iter()
        .filter(|path| !matches_prefix(path, map.tests.keys()))
        .filter(|path| !path.starts_with("reference/"))
        .collect()
}

fn load_owner_map(ctx: &AuditContext) -> Option<crate::commands::context_data::OwnerMapFile> {
    std::fs::read_to_string(ctx.root.join("agent/owner-map.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}

fn load_test_map(ctx: &AuditContext) -> Option<crate::commands::context_data::TestMapFile> {
    std::fs::read_to_string(ctx.root.join("agent/test-map.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}

fn auditable_manifest_paths(ctx: &AuditContext) -> Vec<String> {
    ctx.scope_files
        .iter()
        .filter(|file| {
            !file.rel_path.starts_with(".git/")
                && !file.rel_path.starts_with("target/")
                && !file.rel_path.starts_with("node_modules/")
                && !file.rel_path.starts_with("paper/jankurai.")
                && !file.rel_path.starts_with("reference/")
        })
        .map(|file| file.rel_path.clone())
        .collect()
}

fn matches_prefix<'a>(path: &str, prefixes: impl Iterator<Item = &'a String>) -> bool {
    prefixes.into_iter().any(|prefix| {
        path == prefix
            || path.starts_with(prefix)
            || path.starts_with(&format!("{}/", prefix.trim_end_matches('/')))
    })
}
