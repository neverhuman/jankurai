use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;

fn binary_path() -> &'static str {
    env!("CARGO_BIN_EXE_jankurai")
}

fn write_cargo_fixture(root: &Path, source: &str) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "fixture-rust"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), source).unwrap();
}

#[test]
fn rust_map_and_witness_build_write_expected_artifacts() {
    let dir = tempdir().unwrap();
    write_cargo_fixture(
        dir.path(),
        r#"
pub fn api() -> u32 {
    helper()
}

fn helper() -> u32 {
    7
}
"#,
    );

    let status = Command::new(binary_path())
        .arg("rust")
        .arg("map")
        .arg(dir.path())
        .status()
        .unwrap();
    assert!(status.success());

    let agent_map_path = dir.path().join("target/jankurai/rust/agent-map.json");
    let test_map_path = dir.path().join("target/jankurai/rust/test-map.json");
    assert!(agent_map_path.exists());
    assert!(test_map_path.exists());

    let agent_map: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&agent_map_path).unwrap()).unwrap();
    let test_map: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&test_map_path).unwrap()).unwrap();
    assert_eq!(agent_map["members"].as_array().unwrap().len(), 1);
    assert_eq!(agent_map["members"][0]["name"], "fixture-rust");
    assert!(agent_map["members"][0]["validation_commands"]["local"]
        .as_array()
        .unwrap()
        .iter()
        .any(|cmd| cmd == "cargo test -p fixture-rust"));
    assert_eq!(test_map["entries"].as_array().unwrap().len(), 1);
    assert_eq!(test_map["entries"][0]["arc"], "fixture-rust");
    assert!(test_map["entries"][0]["doctests"]
        .as_array()
        .unwrap()
        .iter()
        .any(|cmd| cmd == "cargo test -p fixture-rust --doc"));

    let status = Command::new(binary_path())
        .arg("rust")
        .arg("witness")
        .arg("build")
        .arg(dir.path())
        .status()
        .unwrap();
    assert!(status.success());

    let witness_path = dir.path().join("target/jankurai/rust/witness-graph.json");
    assert!(witness_path.exists());
    let witness: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&witness_path).unwrap()).unwrap();
    assert_eq!(witness["crates"].as_array().unwrap().len(), 1);
    assert_eq!(witness["crates"][0]["name"], "fixture-rust");
    assert_eq!(
        witness["crates"][0]["interface_hash"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        witness["crates"][0]["implementation_hash"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert!(!witness["crates"][0]["pub_items"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn rust_witness_diff_distinguishes_interface_and_implementation_changes() {
    let dir = tempdir().unwrap();
    write_cargo_fixture(
        dir.path(),
        r#"
pub fn api() -> u32 {
    helper()
}

fn helper() -> u32 {
    7
}
"#,
    );

    let old_graph = dir.path().join("target/jankurai/rust/old.json");
    let new_graph = dir.path().join("target/jankurai/rust/new.json");
    fs::create_dir_all(old_graph.parent().unwrap()).unwrap();

    assert!(Command::new(binary_path())
        .arg("rust")
        .arg("witness")
        .arg("build")
        .arg(dir.path())
        .arg("--out")
        .arg(&old_graph)
        .status()
        .unwrap()
        .success());

    fs::write(
        dir.path().join("src/lib.rs"),
        r#"
pub fn api() -> u32 {
    helper()
}

fn helper() -> u32 {
    8
}
"#,
    )
    .unwrap();

    assert!(Command::new(binary_path())
        .arg("rust")
        .arg("witness")
        .arg("build")
        .arg(dir.path())
        .arg("--out")
        .arg(&new_graph)
        .status()
        .unwrap()
        .success());

    let diff_json = dir.path().join("target/jankurai/rust/diff.json");
    let diff_md = dir.path().join("target/jankurai/rust/diff.md");
    assert!(Command::new(binary_path())
        .arg("rust")
        .arg("witness")
        .arg("diff")
        .arg(dir.path())
        .arg("--old")
        .arg(&old_graph)
        .arg("--new")
        .arg(&new_graph)
        .arg("--out")
        .arg(&diff_json)
        .arg("--md")
        .arg(&diff_md)
        .status()
        .unwrap()
        .success());

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&diff_json).unwrap()).unwrap();
    assert_eq!(report["implementation_only_crates"], 1);
    assert_eq!(report["interface_changed_crates"], 0);
    assert_eq!(
        report["changes"][0]["classification"],
        "implementation-only"
    );

    fs::write(
        dir.path().join("src/lib.rs"),
        r#"
pub fn api(input: u32) -> u32 {
    input
}

fn helper() -> u32 {
    8
}
"#,
    )
    .unwrap();

    let interface_graph = dir.path().join("target/jankurai/rust/interface.json");
    assert!(Command::new(binary_path())
        .arg("rust")
        .arg("witness")
        .arg("build")
        .arg(dir.path())
        .arg("--out")
        .arg(&interface_graph)
        .status()
        .unwrap()
        .success());

    let interface_diff = dir.path().join("target/jankurai/rust/interface-diff.json");
    let interface_md = dir.path().join("target/jankurai/rust/interface-diff.md");
    assert!(Command::new(binary_path())
        .arg("rust")
        .arg("witness")
        .arg("diff")
        .arg(dir.path())
        .arg("--old")
        .arg(&old_graph)
        .arg("--new")
        .arg(&interface_graph)
        .arg("--out")
        .arg(&interface_diff)
        .arg("--md")
        .arg(&interface_md)
        .status()
        .unwrap()
        .success());
    let interface_report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&interface_diff).unwrap()).unwrap();
    assert_eq!(interface_report["interface_changed_crates"], 1);
    assert_eq!(interface_report["implementation_only_crates"], 0);
    assert!(interface_report["escalation_required"].as_bool().unwrap());
}

#[test]
fn rust_diagnose_writes_compile_packets_for_broken_fixture() {
    let dir = tempdir().unwrap();
    write_cargo_fixture(
        dir.path(),
        r#"
pub fn api() -> u32 {
    "oops"
}
"#,
    );

    let output = Command::new(binary_path())
        .arg("rust")
        .arg("diagnose")
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let packets_path = dir.path().join("target/jankurai/rust/compile-packets.json");
    assert!(packets_path.exists());
    let packets: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&packets_path).unwrap()).unwrap();
    assert!(packets["summary"]["total_errors"].as_u64().unwrap() > 0);
    assert_eq!(packets["packets"][0]["owning_crate"], "fixture-rust");
    assert!(packets["packets"][0]["file"]
        .as_str()
        .unwrap()
        .ends_with("src/lib.rs"));
}
