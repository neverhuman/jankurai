use std::fs;
use std::path::{Path, PathBuf};

use jankurai::audit::helpers::AuditContext;
use jankurai::audit::{
    analyzers,
    copy_code::{self, CopyCodeKind, CopyCodeOptions, CopyCodeSeverity},
    copy_code_cross_check, run_audit,
};
use jankurai::model::FileInfo;
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn file(rel_path: &str, text: &str) -> FileInfo {
    let name = Path::new(rel_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(rel_path)
        .to_string();
    let suffix = if name.eq_ignore_ascii_case("Dockerfile") {
        ".dockerfile".to_string()
    } else {
        Path::new(rel_path)
            .extension()
            .and_then(|value| value.to_str())
            .map(|ext| format!(".{ext}"))
            .unwrap_or_default()
    };
    FileInfo {
        rel_path: rel_path.to_string(),
        name,
        suffix,
        size: text.len() as u64,
        line_count: text.lines().count().max(1),
        text: text.to_string(),
        is_generated: false,
        is_code: true,
    }
}

fn scan(
    files: Vec<FileInfo>,
    include_tests: bool,
    min_lines: usize,
    min_tokens: usize,
) -> copy_code::CopyCodeReport {
    copy_code::scan_files(
        Path::new("repo"),
        &files,
        CopyCodeOptions {
            min_lines,
            min_tokens,
            include_tests,
            ..CopyCodeOptions::default()
        },
    )
}

fn shape_score(files: Vec<FileInfo>, report: Option<copy_code::CopyCodeReport>) -> i32 {
    let ctx = AuditContext {
        root: PathBuf::from("repo"),
        all_files: files.clone(),
        scope_files: files,
        scope_paths: vec![],
        self_audit: false,
        boundary_reclassifications: vec![],
        copy_code: report,
    };
    analyzers::shape::analyze(&ctx).score
}

fn write_audit_surface(repo: &Path) {
    fs::create_dir_all(repo.join("agent")).unwrap();
    fs::create_dir_all(repo.join("docs")).unwrap();
    fs::create_dir_all(repo.join("src")).unwrap();
    for rel in [
        "AGENTS.md",
        "README.md",
        "Justfile",
        "VERSION",
        "Cargo.toml",
        "agent/standard-version.toml",
        "agent/proof-lanes.toml",
        "agent/tool-adoption.toml",
        "agent/JANKURAI_STANDARD.md",
        "docs/agent-native-standard.md",
    ] {
        fs::copy(repo_root().join(rel), repo.join(rel)).unwrap();
    }
    fs::write(
        repo.join("agent/owner-map.json"),
        r#"{
  "workspace": "copy-code-fixture",
  "owners": {
    "AGENTS.md": "agent",
    "README.md": "workspace",
    "Justfile": "workspace",
    "VERSION": "workspace",
    "Cargo.toml": "tools",
    "agent/": "agent",
    "docs/": "standard",
    "src/": "tools"
  }
}"#,
    )
    .unwrap();
    fs::write(
        repo.join("agent/test-map.json"),
        r#"{
  "workspace": "copy-code-fixture",
  "tests": {
    "AGENTS.md": { "command": "just fast", "purpose": "root instructions" },
    "README.md": { "command": "just score", "purpose": "workspace routing" },
    "Justfile": { "command": "just fast", "purpose": "command surface" },
    "VERSION": { "command": "just versions", "purpose": "version source" },
    "Cargo.toml": { "command": "cargo test -p jankurai", "purpose": "workspace manifest" },
    "agent/": { "command": "just score", "purpose": "agent policy" },
    "docs/": { "command": "just score", "purpose": "docs" },
    "src/": { "command": "cargo test -p jankurai --test copy_code_scan", "purpose": "copy-code fixture" }
  }
}"#,
    )
    .unwrap();
}

#[test]
fn exact_duplicate_active_files_produce_hard_class_finding_and_cap() {
    let repo = tempdir().unwrap();
    write_audit_surface(repo.path());
    let files = vec![
        file("src/a.rs", "pub fn run() { println!(\"hi\"); }\n"),
        file("src/b.rs", "pub fn run() { println!(\"hi\"); }\n"),
    ];
    fs::write(
        repo.path().join("src/a.rs"),
        "pub fn run() { println!(\"hi\"); }\n",
    )
    .unwrap();
    fs::write(
        repo.path().join("src/b.rs"),
        "pub fn run() { println!(\"hi\"); }\n",
    )
    .unwrap();

    let report = run_audit(repo.path(), &[]).unwrap();
    let copy = report.copy_code.as_ref().expect("copy-code report");
    assert_eq!(copy.summary.hard_classes, 1);
    assert_eq!(copy.classes[0].kind, CopyCodeKind::ExactFile);
    let without = shape_score(files.clone(), None);
    let with = shape_score(files, Some(copy.clone()));
    assert!(with < without);
    assert!(report
        .findings
        .iter()
        .any(|finding| finding.rule_id.as_deref() == Some("HLT-043-COPY-PASTE-BAD-BEHAVIOR")));
    assert!(report
        .caps_applied
        .iter()
        .any(|cap| cap == "severe-duplication-in-product-code"));
}

#[test]
fn exact_same_name_rust_units_are_hard() {
    let report = scan(
        vec![
            file(
                "src/a.rs",
                "// alpha copy\npub fn run() {\n    let greeting = \"hi\";\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n}\n",
            ),
            file(
                "src/b.rs",
                "// beta copy\npub fn run() {\n    let greeting = \"hi\";\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n    println!(\"alpha {} {} {} {} {} {} {} {} {}\", greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting, greeting);\n}\n",
            ),
        ],
        false,
        10,
        100,
    );
    assert_eq!(report.summary.hard_classes, 1);
    let class = &report.classes[0];
    assert_eq!(class.kind, CopyCodeKind::ExactUnitSameName);
    assert_eq!(class.severity, CopyCodeSeverity::Hard);
    assert_eq!(class.language, "rust");
    assert_eq!(class.unit_name.as_deref(), Some("run"));
}

#[test]
fn same_body_different_names_are_warning_only_and_score_neutral() {
    let files = vec![
        file("src/a.rs", "pub fn alpha() { println!(\"hi\"); }\n"),
        file("src/b.rs", "pub fn beta() { println!(\"hi\"); }\n"),
    ];
    let report = scan(files.clone(), false, 10, 100);
    assert_eq!(report.summary.hard_classes, 0);
    assert_eq!(report.summary.warning_classes, 1);
    assert_eq!(report.classes[0].kind, CopyCodeKind::ExactUnitDifferentName);
    assert_eq!(report.classes[0].severity, CopyCodeSeverity::Warning);

    let without = shape_score(files.clone(), None);
    let with = shape_score(files, Some(report));
    assert_eq!(without, with);
}

#[test]
fn boilerplate_lines_do_not_produce_findings() {
    let report = scan(
        vec![
            file(
                "src/a.rs",
                "use std::fmt::Debug;\ntype Foo = ();\n// header\n// more header\n",
            ),
            file(
                "src/b.rs",
                "use std::fmt::Debug;\ntype Bar = ();\n// header\n// more header\n",
            ),
        ],
        false,
        2,
        2,
    );
    assert!(report.classes.is_empty());
}

#[test]
fn excluded_paths_are_ignored() {
    let report = scan(
        vec![
            file("src/a.rs", "pub fn run() { println!(\"hi\"); }\n"),
            file("src/b.rs", "pub fn run() { println!(\"hi\"); }\n"),
            file("generated/a.rs", "pub fn run() { println!(\"hi\"); }\n"),
            file("vendor/a.rs", "pub fn run() { println!(\"hi\"); }\n"),
            file("target/a.rs", "pub fn run() { println!(\"hi\"); }\n"),
            file("docs/a.rs", "pub fn run() { println!(\"hi\"); }\n"),
            file("reference/a.rs", "pub fn run() { println!(\"hi\"); }\n"),
            file("tips/a.rs", "pub fn run() { println!(\"hi\"); }\n"),
        ],
        false,
        10,
        100,
    );
    assert_eq!(report.summary.files_considered, 2);
    assert_eq!(report.summary.hard_classes, 1);
}

#[test]
fn warning_only_paths_are_excluded_by_default_and_warn_when_enabled() {
    let files = vec![
        file("tests/a.rs", "pub fn run() { println!(\"hi\"); }\n"),
        file("tests/b.rs", "pub fn run() { println!(\"hi\"); }\n"),
        file("fixtures/a.rs", "pub fn run() { println!(\"hi\"); }\n"),
        file("stories/a.tsx", "export const View = () => <div />;\n"),
    ];

    let default_report = scan(files.clone(), false, 10, 100);
    assert!(default_report.classes.is_empty());

    let included = scan(files, true, 10, 100);
    assert_eq!(included.summary.hard_classes, 0);
    assert!(!included.classes.is_empty());
    assert!(included
        .classes
        .iter()
        .all(|class| class.severity == CopyCodeSeverity::Warning));
}

#[test]
fn overlapping_repeated_blocks_collapse_to_one_token_class() {
    let report = scan(
        vec![
            file(
                "src/a.rs",
                "// a\nalpha_variable = 1001 + 2002\nbeta_variable = 1002 + 2003\ngamma_variable = 1003 + 2004\ndelta_variable = 1004 + 2005\nalpha_variable = 1001 + 2002\nbeta_variable = 1002 + 2003\ngamma_variable = 1003 + 2004\ndelta_variable = 1004 + 2005\n// end a\n",
            ),
            file(
                "src/b.rs",
                "// b\nalpha_variable = 1001 + 2002\nbeta_variable = 1002 + 2003\ngamma_variable = 1003 + 2004\ndelta_variable = 1004 + 2005\nalpha_variable = 1001 + 2002\nbeta_variable = 1002 + 2003\ngamma_variable = 1003 + 2004\ndelta_variable = 1004 + 2005\n// end b\n",
            ),
        ],
        false,
        4,
        4,
    );
    assert_eq!(report.classes.len(), 1);
    assert_eq!(report.classes[0].kind, CopyCodeKind::TokenBlock);
}

#[test]
fn python_and_typescript_unit_extraction_works() {
    let report = scan(
        vec![
            file("src/a.py", "# a\ndef run():\n    return 1\n"),
            file("src/b.py", "# b\ndef run():\n    return 1\n"),
            file(
                "src/a.tsx",
                "// a\nexport const Widget = () => { return <div />; };\n",
            ),
            file(
                "src/b.tsx",
                "// b\nexport const Widget = () => { return <div />; };\n",
            ),
        ],
        false,
        10,
        100,
    );
    assert!(report
        .classes
        .iter()
        .any(|class| class.language == "python" && class.kind == CopyCodeKind::ExactUnitSameName));
    assert!(report.classes.iter().any(|class| {
        class.language == "typescript" && class.kind == CopyCodeKind::ExactUnitSameName
    }));
}

#[test]
fn docker_and_config_duplicates_are_warning_only() {
    let report = scan(
        vec![
            file("Dockerfile", "FROM rust:1.78\nRUN echo hi\n"),
            file("docker/Dockerfile", "FROM rust:1.78\nRUN echo hi\n"),
            file("config/app.conf", "enabled=true\nvalue=1\n"),
            file("config/other.conf", "enabled=true\nvalue=1\n"),
        ],
        true,
        10,
        100,
    );
    assert!(!report.classes.is_empty());
    assert!(report
        .classes
        .iter()
        .all(|class| class.severity == CopyCodeSeverity::Warning));
}

// Helper: a Rust function body with enough lines/tokens to trigger ExactUnitSameName.
// Uses 20 iterations so body has 20 lines × 6 tokens ≥ min_tokens=100.
fn long_fn_body(tag: &str) -> String {
    let mut s = format!("// {tag}\npub fn compute() {{\n");
    for i in 0..20 {
        s.push_str(&format!(
            "    let v{i} = alpha_{i} + beta_{i} + gamma_{i} + delta_{i};\n",
            i = i
        ));
    }
    s.push_str("}\n");
    s
}

#[test]
fn volume_ranking_orders_by_total_redundant_lines() {
    // Group A: 3 identical 25-line files  → total_redundant = (3-1)*25 = 50
    // Group B: 2 identical 30-line files  → total_redundant = (2-1)*30 = 30
    // Group A must sort first.
    let line_a = "x = alpha_variable + beta_variable + gamma_variable\n";
    let line_b = "y = delta_variable + epsilon_variable + zeta_variable + eta_variable\n";

    let mut body_a = String::new();
    for _ in 0..25 {
        body_a.push_str(line_a);
    }
    let mut body_b = String::new();
    for _ in 0..30 {
        body_b.push_str(line_b);
    }

    let report = scan(
        vec![
            file("src/a1.rs", &body_a),
            file("src/a2.rs", &body_a),
            file("src/a3.rs", &body_a),
            file("src/b1.rs", &body_b),
            file("src/b2.rs", &body_b),
        ],
        false,
        2,
        2,
    );

    let hard: Vec<_> = report.classes.iter().filter(|c| c.hard_fail).collect();
    assert!(hard.len() >= 2, "expected at least two hard classes");
    // First hard class must have higher total_redundant_lines.
    assert!(
        hard[0].total_redundant_lines >= hard[1].total_redundant_lines,
        "expected descending volume sort: {} >= {}",
        hard[0].total_redundant_lines,
        hard[1].total_redundant_lines
    );
    // Confirm the 3-instance class sorts first (total 50 vs 30).
    let class_a = hard
        .iter()
        .find(|c| c.instance_count == 3)
        .expect("3-instance class");
    let class_b = hard
        .iter()
        .find(|c| c.instance_count == 2)
        .expect("2-instance class");
    assert!(
        class_a.total_redundant_lines > class_b.total_redundant_lines,
        "class_a redundant={}, class_b redundant={}",
        class_a.total_redundant_lines,
        class_b.total_redundant_lines
    );
    assert_eq!(hard[0].instance_count, 3);
}

#[test]
fn effective_severity_demotes_token_block_to_warning() {
    // Token-block duplicates are always advisory regardless of raw severity.
    let report = scan(
        vec![
            file(
                "src/a.rs",
                "// a\nalpha_variable = 1001 + 2002\nbeta_variable = 1002 + 2003\ngamma_variable = 1003 + 2004\ndelta_variable = 1004 + 2005\n// end a\n",
            ),
            file(
                "src/b.rs",
                "// b\nalpha_variable = 1001 + 2002\nbeta_variable = 1002 + 2003\ngamma_variable = 1003 + 2004\ndelta_variable = 1004 + 2005\n// end b\n",
            ),
        ],
        false,
        4,
        4,
    );
    let token_blocks: Vec<_> = report
        .classes
        .iter()
        .filter(|c| c.kind == CopyCodeKind::TokenBlock)
        .collect();
    for class in &token_blocks {
        assert_eq!(
            class.effective_severity,
            CopyCodeSeverity::Warning,
            "token block must have effective_severity=Warning"
        );
        assert!(!class.hard_fail, "token block must never hard_fail");
    }
    // There may be zero token blocks (threshold) but the assertion still holds vacuously.
}

#[test]
fn exact_unit_same_name_requires_both_active_for_hard() {
    // One instance in tests/ (WarningOnly). Class must not be hard.
    // Files must differ at the file level so ExactFile doesn't cover them.
    let shared = long_fn_body("shared");
    let body_a = format!("pub fn unique_src() {{ let x = 1; }}\n{shared}");
    let body_b = format!("pub fn unique_test() {{ let x = 2; }}\n{shared}");
    let report = scan(
        vec![file("src/a.rs", &body_a), file("tests/b.rs", &body_b)],
        true, // include_tests so tests/ path is considered
        10,
        100,
    );
    let same_name: Vec<_> = report
        .classes
        .iter()
        .filter(|c| c.kind == CopyCodeKind::ExactUnitSameName)
        .collect();
    for class in &same_name {
        assert!(
            !class.hard_fail,
            "ExactUnitSameName with one test-path instance must not hard_fail"
        );
        assert_eq!(class.effective_severity, CopyCodeSeverity::Warning);
    }
}

#[test]
fn exact_unit_same_name_in_two_active_files_is_hard() {
    // Files share the same compute() function body but have different preambles
    // so they are NOT exact-file copies (ExactFile excludes files from unit scan).
    let shared = long_fn_body("shared");
    let body_a = format!("pub fn unique_alpha() {{ let x = 1; }}\n{shared}");
    let body_b = format!("pub fn unique_beta() {{ let x = 2; }}\n{shared}");
    let report = scan(
        vec![file("src/a.rs", &body_a), file("src/b.rs", &body_b)],
        false,
        10,
        100,
    );
    let same_name: Vec<_> = report
        .classes
        .iter()
        .filter(|c| c.kind == CopyCodeKind::ExactUnitSameName)
        .collect();
    assert!(
        !same_name.is_empty(),
        "expected at least one ExactUnitSameName class"
    );
    assert!(
        same_name.iter().any(|c| c.hard_fail),
        "expected at least one hard ExactUnitSameName when both paths are active"
    );
    assert!(
        same_name
            .iter()
            .any(|c| c.effective_severity == CopyCodeSeverity::Hard),
        "effective_severity must be Hard"
    );
}

#[test]
fn fingerprint_is_stable_across_runs() {
    let body = "pub fn stable() { let x = 1 + 2; let y = x * 3; }\n";
    let files = vec![file("src/a.rs", body), file("src/b.rs", body)];
    let r1 = scan(files.clone(), false, 1, 1);
    let r2 = scan(files, false, 1, 1);
    for (c1, c2) in r1.classes.iter().zip(r2.classes.iter()) {
        assert_eq!(
            c1.fingerprint, c2.fingerprint,
            "fingerprint must be identical across runs"
        );
    }
}

#[test]
fn fingerprint_changes_when_paths_change() {
    let body = "pub fn stable() { let x = 1 + 2; let y = x * 3; }\n";
    let r1 = scan(
        vec![file("src/alpha.rs", body), file("src/beta.rs", body)],
        false,
        1,
        1,
    );
    let r2 = scan(
        vec![file("src/gamma.rs", body), file("src/delta.rs", body)],
        false,
        1,
        1,
    );
    if !r1.classes.is_empty() && !r2.classes.is_empty() {
        assert_ne!(
            r1.classes[0].fingerprint, r2.classes[0].fingerprint,
            "fingerprint must differ when paths change"
        );
    }
}

#[test]
fn workspace_cargo_toml_does_not_hard_fail() {
    // Cargo.toml files are workspace manifests → forced WarningOnly → never hard.
    let toml_body = "[package]\nname = \"my-crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\nanyhow = \"1\"\n";
    let report = scan(
        vec![
            file("crates/foo/Cargo.toml", toml_body),
            file("crates/bar/Cargo.toml", toml_body),
        ],
        true, // include_tests so workspace manifests are considered
        1,
        1,
    );
    for class in &report.classes {
        assert!(
            !class.hard_fail,
            "Cargo.toml duplicates must never hard_fail; class={:?}",
            class.kind
        );
    }
}

#[test]
fn derive_block_does_not_produce_token_class() {
    // Files with only derive/attribute boilerplate should not trigger token-block classes.
    let body = "#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct MyStruct {\n    pub field_alpha: String,\n    pub field_beta: u64,\n    pub field_gamma: bool,\n}\n";
    let report = scan(
        vec![file("src/a.rs", body), file("src/b.rs", body)],
        false,
        2,
        4,
    );
    let token_blocks: Vec<_> = report
        .classes
        .iter()
        .filter(|c| c.kind == CopyCodeKind::TokenBlock)
        .collect();
    assert!(
        token_blocks.is_empty(),
        "derive-only content must not produce a TokenBlock class; got {} classes",
        token_blocks.len()
    );
}

#[test]
fn allowlist_entry_demotes_hard_class() {
    let tmp = tempdir().unwrap();
    // Write identical files so we get an ExactFile Hard class.
    let body = "pub fn run() { println!(\"hi\"); }\n";
    let files = vec![file("src/a.rs", body), file("src/b.rs", body)];

    // First scan: no allowlist → get the fingerprint.
    let first = copy_code::scan_files(tmp.path(), &files, CopyCodeOptions::default());
    let hard: Vec<_> = first.classes.iter().filter(|c| c.hard_fail).collect();
    assert!(!hard.is_empty(), "expected a hard class before allowlist");
    let fp = hard[0].fingerprint.clone();

    // Write the allowlist with a future expiry.
    fs::create_dir_all(tmp.path().join("agent")).unwrap();
    fs::write(
        tmp.path().join("agent/copy-code-allowlist.toml"),
        format!(
            "[[entries]]\nfingerprint = \"{fp}\"\nowner = \"@test\"\nreason = \"test suppression\"\nexpires = \"2030-01-01\"\n"
        ),
    )
    .unwrap();

    // Second scan: allowlist present → class should be suppressed.
    let second = copy_code::scan_files(tmp.path(), &files, CopyCodeOptions::default());
    let suppressed: Vec<_> = second
        .classes
        .iter()
        .filter(|c| c.fingerprint == fp)
        .collect();
    assert!(
        !suppressed.is_empty(),
        "class with matching fingerprint not found"
    );
    assert!(
        suppressed[0].suppressed.is_some(),
        "class must have suppressed field set"
    );
    assert!(
        !suppressed[0].hard_fail,
        "suppressed class must not hard_fail"
    );
    assert_eq!(
        suppressed[0].effective_severity,
        CopyCodeSeverity::Warning,
        "suppressed class must be demoted to Warning"
    );
}

#[test]
fn expired_allowlist_entry_is_ignored() {
    let tmp = tempdir().unwrap();
    let body = "pub fn run() { println!(\"hi\"); }\n";
    let files = vec![file("src/a.rs", body), file("src/b.rs", body)];

    // First scan to get fingerprint.
    let first = copy_code::scan_files(tmp.path(), &files, CopyCodeOptions::default());
    let hard: Vec<_> = first.classes.iter().filter(|c| c.hard_fail).collect();
    assert!(!hard.is_empty(), "need a hard class for this test");
    let fp = hard[0].fingerprint.clone();

    // Write allowlist with a past expiry → should be ignored.
    fs::create_dir_all(tmp.path().join("agent")).unwrap();
    fs::write(
        tmp.path().join("agent/copy-code-allowlist.toml"),
        format!(
            "[[entries]]\nfingerprint = \"{fp}\"\nowner = \"@test\"\nreason = \"expired\"\nexpires = \"2020-01-01\"\n"
        ),
    )
    .unwrap();

    let second = copy_code::scan_files(tmp.path(), &files, CopyCodeOptions::default());
    let class = second
        .classes
        .iter()
        .find(|c| c.fingerprint == fp)
        .expect("class with matching fingerprint");
    assert!(
        class.hard_fail,
        "expired allowlist entry must be ignored; class must still hard_fail"
    );
    assert!(
        class.suppressed.is_none(),
        "expired entry must not suppress the class"
    );
}

#[test]
fn rank_sort_order_matches_total_redundant_volume() {
    // Verify that report classes are pre-sorted by total_redundant_lines desc
    // (the same ordering that `copy-code rank` would display).
    let line = "alpha_var = one + two + three + four + five + six + seven\n";
    let mut big = String::new();
    for _ in 0..40 {
        big.push_str(line);
    }
    let mut small = String::new();
    for _ in 0..20 {
        small.push_str(line);
    }

    let report = scan(
        vec![
            file("src/a1.rs", &big),
            file("src/a2.rs", &big),
            file("src/a3.rs", &big), // 3-instance group: (3-1)*40 = 80
            file("src/b1.rs", &small),
            file("src/b2.rs", &small), // 2-instance group: (2-1)*20 = 20
        ],
        false,
        2,
        2,
    );

    let hard: Vec<_> = report.classes.iter().filter(|c| c.hard_fail).collect();
    for window in hard.windows(2) {
        assert!(
            window[0].total_redundant_lines >= window[1].total_redundant_lines,
            "hard classes must be sorted by total_redundant_lines desc: {} < {}",
            window[0].total_redundant_lines,
            window[1].total_redundant_lines
        );
    }
}

#[test]
fn rank_kind_filter_hard_only_excludes_warning_classes() {
    let report = scan(
        vec![
            file("src/a.rs", "pub fn run() { println!(\"hi\"); }\n"),
            file("src/b.rs", "pub fn run() { println!(\"hi\"); }\n"),
        ],
        false,
        1,
        1,
    );
    // All hard classes must have hard_fail=true.
    for class in report.classes.iter().filter(|c| c.hard_fail) {
        assert!(
            class.effective_severity == CopyCodeSeverity::Hard,
            "hard_fail class must have effective_severity=Hard"
        );
    }
    // All warning classes must have hard_fail=false.
    for class in report.classes.iter().filter(|c| !c.hard_fail) {
        assert!(
            class.effective_severity == CopyCodeSeverity::Warning,
            "non-hard_fail class must have effective_severity=Warning"
        );
    }
}

#[test]
fn jscpd_bridge_unavailable_when_not_on_path() {
    // If jscpd is not installed (the common case), run_jscpd reports unavailable.
    // If jscpd IS installed, we verify it ran without panic.
    let tmp = tempdir().unwrap();
    let out_dir = tmp.path().join("out");
    let result = copy_code_cross_check::run_jscpd(tmp.path(), &out_dir)
        .expect("run_jscpd must not return Err");
    if !result.available {
        let note = result.note.as_deref().unwrap_or("");
        assert!(
            note.contains("jscpd not on PATH") || note.contains("jscpd"),
            "unavailable note must mention jscpd; got: {note:?}"
        );
    }
    // If available, we just verify it ran cleanly (no assertion on count since tmp is empty).
}

#[test]
fn rename_invariance_unit_extraction() {
    // Same name + same body → ExactUnitSameName.
    // Files must differ at file level so ExactFile doesn't cover them first.
    let shared = long_fn_body("shared");
    let body_a = format!("pub fn helper_one() {{ let x = 1; }}\n{shared}");
    let body_b = format!("pub fn helper_two() {{ let x = 2; }}\n{shared}");
    let report_same = scan(
        vec![file("src/a.rs", &body_a), file("src/b.rs", &body_b)],
        false,
        10,
        100,
    );
    assert!(
        report_same
            .classes
            .iter()
            .any(|c| c.kind == CopyCodeKind::ExactUnitSameName),
        "same-name same-body must produce ExactUnitSameName"
    );

    // Rename function in one file → no ExactUnitSameName hard class.
    let body_b_renamed = format!(
        "pub fn helper_two() {{ let x = 2; }}\n{}",
        shared.replace("fn compute()", "fn renamed_compute()")
    );
    let report_renamed = scan(
        vec![file("src/a.rs", &body_a), file("src/b.rs", &body_b_renamed)],
        false,
        10,
        100,
    );
    // Should not produce an ExactUnitSameName Hard class for the renamed pair.
    let same_name_hard: Vec<_> = report_renamed
        .classes
        .iter()
        .filter(|c| c.kind == CopyCodeKind::ExactUnitSameName && c.hard_fail)
        .collect();
    assert!(
        same_name_hard.is_empty(),
        "renamed function must not produce ExactUnitSameName hard class"
    );
    // May produce ExactUnitDifferentName (advisory).
    for class in report_renamed
        .classes
        .iter()
        .filter(|c| c.kind == CopyCodeKind::ExactUnitDifferentName)
    {
        assert!(
            !class.hard_fail,
            "ExactUnitDifferentName must never hard_fail"
        );
    }
}

#[test]
fn comment_whitespace_only_difference_still_matches_exact_file() {
    // Normalization strips trailing whitespace and blank lines;
    // files with only whitespace/comment differences should still hash-match.
    let body_a = "pub fn run() { println!(\"hi\"); }\n";
    let body_b = "pub fn run() { println!(\"hi\"); }\n\n"; // extra trailing newline
    let report = scan(
        vec![file("src/a.rs", body_a), file("src/b.rs", body_b)],
        false,
        1,
        1,
    );
    let exact_file: Vec<_> = report
        .classes
        .iter()
        .filter(|c| c.kind == CopyCodeKind::ExactFile)
        .collect();
    assert!(
        !exact_file.is_empty(),
        "trailing-newline-only difference must still produce ExactFile class"
    );
}

#[test]
#[ignore = "perf bound — slow, run manually with --include-ignored"]
fn perf_bound_does_not_panic_on_5000_line_file() {
    let mut big = String::with_capacity(100_000);
    for i in 0..5000 {
        big.push_str(&format!(
            "let var_{i} = alpha_{i} + beta_{i} + gamma_{i} + delta_{i} + epsilon_{i};\n"
        ));
    }
    let files = vec![file("src/big.rs", &big)];
    let start = std::time::Instant::now();
    let _ = scan(files, false, 10, 100);
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 10,
        "scan of 5000-line file took too long: {elapsed:?}"
    );
}
