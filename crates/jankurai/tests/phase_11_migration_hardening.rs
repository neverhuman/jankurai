use jankurai::commands::migrate;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/migration")
        .join(name)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

// ---------------------------------------------------------------------------
// Stack Detection per fixture
// ---------------------------------------------------------------------------

#[test]
fn detect_node_express() {
    let inv = migrate::detect_stack(&fixture("node-express"));
    assert!(inv.languages.iter().any(|l| l.name == "typescript"));
    assert!(inv.frameworks.iter().any(|f| f.name == "express"));
    assert!(inv.db_clients.iter().any(|d| d.name == "prisma"));
    assert!(inv.test_frameworks.iter().any(|t| t.name == "jest"));
    assert!(inv.api_surfaces.iter().any(|a| a.framework == "express"));
}

#[test]
fn detect_java_spring() {
    let inv = migrate::detect_stack(&fixture("java-spring"));
    assert!(inv.languages.iter().any(|l| l.name == "java"));
    assert!(inv.frameworks.iter().any(|f| f.name == "spring"));
    assert!(inv.api_surfaces.iter().any(|a| a.framework == "spring"));
}

#[test]
fn detect_ruby_rails() {
    let inv = migrate::detect_stack(&fixture("ruby-rails"));
    assert!(inv.languages.iter().any(|l| l.name == "ruby"));
    assert!(inv.frameworks.iter().any(|f| f.name == "rails"));
    assert!(inv.api_surfaces.iter().any(|a| a.framework == "rails"));
}

#[test]
fn detect_go_api() {
    let inv = migrate::detect_stack(&fixture("go-api"));
    assert!(inv.languages.iter().any(|l| l.name == "go"));
    assert!(inv.package_managers.iter().any(|p| p.name == "go-modules"));
    assert!(inv.test_frameworks.iter().any(|t| t.name == "go-test"));
}

#[test]
fn detect_unknown_stack() {
    let inv = migrate::detect_stack(&fixture("unknown-stack"));
    assert!(
        inv.languages.is_empty(),
        "unknown stack should have no languages"
    );
    assert!(inv.frameworks.is_empty());
}

// ---------------------------------------------------------------------------
// Dimensional Liability Scoring
// ---------------------------------------------------------------------------

#[test]
fn liability_has_eight_dimensions() {
    let inv = migrate::detect_stack(&fixture("node-express"));
    let liability = migrate::compute_liability(&fixture("node-express"), &inv);
    assert_eq!(liability.dimensions.len(), 8, "expected 8 dimensions");
    let names: Vec<&str> = liability
        .dimensions
        .iter()
        .map(|d| d.name.as_str())
        .collect();
    for expected in [
        "agent-operability",
        "contract-drift",
        "product-truth-sprawl",
        "security-risk",
        "db-data-risk",
        "test-proof-gaps",
        "runtime-cost-risk",
        "migration-complexity",
    ] {
        assert!(names.contains(&expected), "missing dimension: {expected}");
    }
}

#[test]
fn liability_total_is_bounded() {
    for name in [
        "node-express",
        "java-spring",
        "ruby-rails",
        "go-api",
        "unknown-stack",
    ] {
        let path = fixture(name);
        let inv = migrate::detect_stack(&path);
        let liability = migrate::compute_liability(&path, &inv);
        assert!(
            liability.total <= 100,
            "{name}: total {} > 100",
            liability.total
        );
    }
}

#[test]
fn liability_weights_sum_to_one() {
    let inv = migrate::detect_stack(&fixture("node-express"));
    let liability = migrate::compute_liability(&fixture("node-express"), &inv);
    let total_weight: f64 = liability.dimensions.iter().map(|d| d.weight).sum();
    assert!(
        (total_weight - 1.0).abs() < 0.01,
        "weights should sum to ~1.0, got {total_weight}"
    );
}

#[test]
fn rust_repo_has_lower_runtime_cost() {
    let repo = repo_root();
    let inv = migrate::detect_stack(&repo);
    let liability = migrate::compute_liability(&repo, &inv);
    let runtime = liability
        .dimensions
        .iter()
        .find(|d| d.name == "runtime-cost-risk")
        .unwrap();
    assert!(
        runtime.score <= 20,
        "rust repo should have low runtime-cost-risk, got {}",
        runtime.score
    );
}

// ---------------------------------------------------------------------------
// Schema Validation
// ---------------------------------------------------------------------------

#[test]
fn report_validates_against_schema_for_each_fixture() {
    let repo = repo_root();
    for name in [
        "node-express",
        "java-spring",
        "ruby-rails",
        "go-api",
        "unknown-stack",
    ] {
        let report = migrate::build_migration_report(&fixture(name), "rust-ts-postgres")
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let value = serde_json::to_value(&report).unwrap();
        jankurai::validation::validate_value(
            &repo,
            jankurai::validation::ArtifactSchema::MigrationReport,
            &value,
        )
        .unwrap_or_else(|e| panic!("{name} report schema validation failed: {e}"));
    }
}

#[test]
fn plan_validates_against_schema_for_each_fixture() {
    let repo = repo_root();
    for name in [
        "node-express",
        "java-spring",
        "ruby-rails",
        "go-api",
        "unknown-stack",
    ] {
        let plan = migrate::build_migration_plan(&fixture(name), "rust-ts-postgres")
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let value = serde_json::to_value(&plan).unwrap();
        jankurai::validation::validate_value(
            &repo,
            jankurai::validation::ArtifactSchema::MigrationPlan,
            &value,
        )
        .unwrap_or_else(|e| panic!("{name} plan schema validation failed: {e}"));
    }
}

// ---------------------------------------------------------------------------
// Slice Generation
// ---------------------------------------------------------------------------

#[test]
fn node_express_plan_has_db_and_api_slices() {
    let plan = migrate::build_migration_plan(&fixture("node-express"), "rust-ts-postgres").unwrap();
    assert!(
        plan.slices
            .iter()
            .any(|s| s.slice_id.starts_with("db-isolation")),
        "expected db-isolation slice"
    );
    assert!(
        plan.slices
            .iter()
            .any(|s| s.slice_id.starts_with("api-contract")),
        "expected api-contract slice"
    );
    assert!(
        plan.slices
            .iter()
            .any(|s| s.slice_id == "equivalence-proof"),
        "expected equivalence-proof slice"
    );
}

#[test]
fn unknown_stack_plan_has_blocked_equivalence() {
    let plan =
        migrate::build_migration_plan(&fixture("unknown-stack"), "rust-ts-postgres").unwrap();
    let eq = plan
        .slices
        .iter()
        .find(|s| s.slice_id == "equivalence-proof")
        .unwrap();
    assert_eq!(eq.status, "blocked");
}

#[test]
fn slices_have_monotonic_dependency_order() {
    let plan = migrate::build_migration_plan(&fixture("node-express"), "rust-ts-postgres").unwrap();
    let orders: Vec<u32> = plan.slices.iter().map(|s| s.dependency_order).collect();
    for i in 1..orders.len() {
        assert!(
            orders[i] > orders[i - 1],
            "dependency_order not monotonic: {:?}",
            orders
        );
    }
}

#[test]
fn equivalence_proof_always_requires_human_approval() {
    for name in ["node-express", "java-spring", "ruby-rails", "go-api"] {
        let plan = migrate::build_migration_plan(&fixture(name), "rust-ts-postgres").unwrap();
        let eq = plan
            .slices
            .iter()
            .find(|s| s.slice_id == "equivalence-proof")
            .unwrap();
        assert!(
            eq.human_approval_required,
            "{name}: equivalence-proof should require human approval"
        );
        assert_eq!(eq.risk_level, "high");
    }
}

// ---------------------------------------------------------------------------
// Target Flag
// ---------------------------------------------------------------------------

#[test]
fn target_stack_propagates_to_report() {
    let report = migrate::build_migration_report(&fixture("node-express"), "go-postgres").unwrap();
    assert_eq!(report.target_stack, "go-postgres");
}

#[test]
fn target_stack_propagates_to_plan() {
    let plan = migrate::build_migration_plan(&fixture("node-express"), "go-postgres").unwrap();
    assert_eq!(plan.target_stack, "go-postgres");
}

// ---------------------------------------------------------------------------
// Contract Evidence Detection
// ---------------------------------------------------------------------------

#[test]
fn jankurai_repo_detects_schemas_directory() {
    let inv = migrate::detect_stack(&repo_root());
    assert!(
        inv.contract_evidence
            .iter()
            .any(|c| c.kind == "directory" && c.path.contains("schemas")),
        "jankurai repo should detect schemas/ directory as contract evidence"
    );
}

// ---------------------------------------------------------------------------
// Backward Compatibility
// ---------------------------------------------------------------------------

#[test]
fn report_still_has_flat_module_inventory() {
    let report =
        migrate::build_migration_report(&fixture("node-express"), "rust-ts-postgres").unwrap();
    assert!(
        !report.module_inventory.is_empty(),
        "flat module_inventory should still be present"
    );
    assert!(report
        .module_inventory
        .iter()
        .any(|m| m.starts_with("language:")));
}

// ---------------------------------------------------------------------------
// Structured Inventory
// ---------------------------------------------------------------------------

#[test]
fn inventory_items_have_evidence_and_confidence() {
    let inv = migrate::detect_stack(&fixture("node-express"));
    for lang in &inv.languages {
        assert!(
            !lang.evidence.is_empty(),
            "language {} missing evidence",
            lang.name
        );
        assert!(
            ["high", "medium", "low"].contains(&lang.confidence.as_str()),
            "bad confidence: {}",
            lang.confidence
        );
    }
    for fw in &inv.frameworks {
        assert!(
            !fw.evidence.is_empty(),
            "framework {} missing evidence",
            fw.name
        );
    }
}

// ---------------------------------------------------------------------------
// CLI help surface
// ---------------------------------------------------------------------------

#[test]
fn migrate_help_shows_target_flag() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_jankurai"))
        .args(["migrate", "--help"])
        .output()
        .expect("failed to run jankurai migrate --help");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--target"),
        "migrate help should show --target flag"
    );
}
