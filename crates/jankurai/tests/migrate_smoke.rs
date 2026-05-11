use jankurai::commands::migrate;

#[test]
fn analyze_produces_valid_migration_report() {
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let report =
        migrate::build_migration_report(&repo, "rust-ts-postgres").expect("build_migration_report");
    assert_eq!(report.schema_version, "1.0.0");
    assert!(!report.source_stack.is_empty());
    assert!(
        report.source_stack.contains("rust"),
        "expected rust in source_stack, got {}",
        report.source_stack
    );
    assert!(report.liability_score <= 100);
    assert!(!report.module_inventory.is_empty());
    assert!(!report.recommended_slice_order.is_empty());
    assert!(!report.required_proof_lanes.is_empty());
    assert!(!report.rollback_cutover_notes.is_empty());
}

#[test]
fn plan_produces_valid_migration_plan() {
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let plan =
        migrate::build_migration_plan(&repo, "rust-ts-postgres").expect("build_migration_plan");
    assert_eq!(plan.schema_version, "1.0.0");
    assert_eq!(plan.plan_mode, "dry-run");
    assert!(!plan.slices.is_empty(), "expected at least one slice");
    assert!(!plan.human_approval_requirements.is_empty());

    // Validate all slices have required fields
    for slice in &plan.slices {
        assert!(!slice.slice_id.is_empty());
        assert!(!slice.owner.is_empty());
        assert!(
            ["candidate", "ready", "blocked", "do-not-migrate-yet"]
                .contains(&slice.status.as_str()),
            "unexpected status: {}",
            slice.status
        );
        assert!(!slice.proof_lanes.is_empty());
        assert!(!slice.rollback_notes.is_empty());
    }
}

#[test]
fn liability_score_is_bounded() {
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let report =
        migrate::build_migration_report(&repo, "rust-ts-postgres").expect("build_migration_report");
    assert!(
        report.liability_score <= 100,
        "score should be <= 100, got {}",
        report.liability_score
    );
}

#[test]
fn analyze_schema_validates() {
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let report =
        migrate::build_migration_report(&repo, "rust-ts-postgres").expect("build_migration_report");
    let value = serde_json::to_value(&report).expect("serialize report");
    jankurai::validation::validate_value(
        &repo,
        jankurai::validation::ArtifactSchema::MigrationReport,
        &value,
    )
    .expect("report should validate against migration-report.schema.json");
}

#[test]
fn plan_schema_validates() {
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let plan =
        migrate::build_migration_plan(&repo, "rust-ts-postgres").expect("build_migration_plan");
    let value = serde_json::to_value(&plan).expect("serialize plan");
    jankurai::validation::validate_value(
        &repo,
        jankurai::validation::ArtifactSchema::MigrationPlan,
        &value,
    )
    .expect("plan should validate against migration-plan.schema.json");
}
