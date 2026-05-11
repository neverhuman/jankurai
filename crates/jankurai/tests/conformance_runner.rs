use std::path::{Path, PathBuf};

use jankurai::commands::conformance::{build_report, render_tex_table, ConformanceRunArgs};
use jankurai::validation::{self, ArtifactSchema};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn conformance_runner_observes_expected_fixture_decisions() {
    let root = workspace_root();
    let args = ConformanceRunArgs {
        workspace: root.clone(),
        fixtures: PathBuf::from("conformance/fixtures"),
        expected: PathBuf::from("conformance/expected"),
        out: "target/jankurai/conformance-results.json".into(),
        md: "target/jankurai/conformance-results.md".into(),
        tex: "paper/tex/generated/conformance_results_table.tex".into(),
    };

    let report = build_report(&args).expect("build conformance report");
    validation::validate_serializable(&root, ArtifactSchema::ConformanceResults, &report)
        .expect("conformance report validates");
    assert_eq!(report.fixture_count, 10);
    assert_eq!(report.pass_count, 10);
    assert_eq!(report.fail_count, 0);
    assert!(report.results.iter().any(|result| {
        result.fixture_id == "hl3-pass-minimal"
            && result.observed_audit_decision == "pass"
            && result.observed_witness_decision == "pass"
    }));
    assert!(report.results.iter().any(|result| {
        result.fixture_id == "generated-zone-mutation-fail"
            && result
                .observed_rules
                .iter()
                .any(|rule| rule == "HLT-002-GENERATED-MUTATION")
    }));

    let tex = render_tex_table(&report, &args);
    assert!(tex.contains("\\label{tab:conformance-results}"));
    assert!(tex.contains("hl3-pass-minimal"));
}
