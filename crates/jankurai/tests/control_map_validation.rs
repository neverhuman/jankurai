use std::fs;
use std::path::PathBuf;

use jankurai::validation;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn canonical_owner_map_validates_with_strict_json_parsing() {
    let repo = repo_root();
    let text = fs::read_to_string(repo.join("agent/owner-map.json")).expect("read owner map");

    validation::validate_owner_map_json_text(&repo, &text)
        .expect("canonical owner map should validate");
}

#[test]
fn canonical_test_map_validates_with_strict_json_parsing() {
    let repo = repo_root();
    let text = fs::read_to_string(repo.join("agent/test-map.json")).expect("read test map");

    validation::validate_test_map_json_text(&repo, &text)
        .expect("canonical test map should validate");
}

#[test]
fn local_jeryu_shadow_config_is_present_and_routed() {
    let repo = repo_root();
    let text = fs::read_to_string(repo.join(".jeryu/local/repos/jankurai.toml"))
        .expect("read local Jeryu shadow config");

    assert!(text.contains("repo = \"root/jankurai\""));
    assert!(text.contains("remote_url = \"git@github.com:neverhuman/jankurai.git\""));
    assert!(text.contains("refs = [\"refs/heads/main\"]"));
}

#[test]
fn strict_json_parser_rejects_duplicate_owner_keys() {
    let text = r#"
    {
      "workspace": "jankurai",
      "owners": {
        "ops/": "ops",
        "ops/": "security"
      }
    }
    "#;

    let err = validation::parse_json_value_strict(text).expect_err("duplicate keys must fail");
    let message = err.to_string();
    assert!(
        message.contains("duplicate JSON key `ops/`"),
        "unexpected error message: {message}"
    );
}

#[test]
fn strict_json_parser_rejects_duplicate_test_keys() {
    let text = r#"
    {
      "workspace": "jankurai",
      "tests": {
        "ops/": {
          "command": "bash scripts/ci-local.sh quick"
        },
        "ops/": {
          "command": "just security"
        }
      }
    }
    "#;

    let err = validation::parse_json_value_strict(text).expect_err("duplicate keys must fail");
    let message = err.to_string();
    assert!(
        message.contains("duplicate JSON key `ops/`"),
        "unexpected error message: {message}"
    );
}
