use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

use jankurai::validation::{self, ArtifactSchema};

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jankurai"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/migration/prompt-verify")
        .join(name)
}

fn run_prompt_verify(
    repo: &PathBuf,
    document: &str,
    strict: bool,
) -> (std::process::Output, tempfile::TempDir, PathBuf, PathBuf) {
    let out_dir = tempdir().unwrap();
    let json_path = out_dir.path().join("prompt.json");
    let md_path = out_dir.path().join("prompt.md");
    let mut cmd = Command::new(binary_path());
    cmd.arg("migrate")
        .arg(repo)
        .arg("verify-prompt")
        .arg(document)
        .arg("--out")
        .arg(&json_path)
        .arg("--md")
        .arg(&md_path);
    if strict {
        cmd.arg("--strict");
    }
    let output = cmd.output().unwrap();
    (output, out_dir, json_path, md_path)
}

#[test]
fn prompt_verifier_accepts_good_claims() {
    let repo = fixture("repo-good");
    let (output, _dir, json_path, md_path) = run_prompt_verify(&repo, "prompt.md", false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    validation::validate_value(
        &fixture("repo-good"),
        ArtifactSchema::MigrationPromptVerification,
        &report,
    )
    .unwrap();
    assert_eq!(report["decision"], "pass");
    assert_eq!(report["claims_total"], 4);
    assert_eq!(report["claims_verified"], 4);
    assert_eq!(report["claims_invalid"], 0);
    assert_eq!(report["claims_review"], 0);
    assert!(fs::read_to_string(&md_path)
        .unwrap()
        .starts_with("# jankurai Migration Prompt Verification"));
}

#[test]
fn prompt_verifier_marks_ambiguous_matches_as_review() {
    let repo = fixture("repo-review");
    let (output, _dir, json_path, _) = run_prompt_verify(&repo, "prompt.md", false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    validation::validate_value(
        &fixture("repo-review"),
        ArtifactSchema::MigrationPromptVerification,
        &report,
    )
    .unwrap();
    assert_eq!(report["decision"], "review");
    assert_eq!(report["claims_total"], 1);
    assert_eq!(report["claims_review"], 1);
    assert_eq!(report["claims_invalid"], 0);
}

#[test]
fn prompt_verifier_rejects_invalid_claims_in_strict_mode() {
    let repo = fixture("repo-bad");
    let (output, _dir, json_path, _) = run_prompt_verify(&repo, "prompt.md", true);
    assert!(
        !output.status.success(),
        "strict mode should fail on invalid claims"
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    validation::validate_value(
        &fixture("repo-bad"),
        ArtifactSchema::MigrationPromptVerification,
        &report,
    )
    .unwrap();
    assert_eq!(report["decision"], "fail");
    assert!(report["claims_invalid"].as_u64().unwrap() >= 3);
    assert!(report["claims_total"].as_u64().unwrap() >= 4);
}

#[cfg(unix)]
#[test]
fn prompt_verifier_skips_refutation_rows_blockquotes_and_dotted_refs() {
    let repo_dir = tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("docs")).unwrap();
    fs::write(repo_dir.path().join("prompt.md"), "- docs/guide.md:1\n> src/ignored.rs:3\n| False | reality | actually | no LLM call |\n- pkg.module:8\n").unwrap();
    fs::write(repo_dir.path().join("docs/guide.md"), "guide claim\n").unwrap();

    let (output, _dir, json_path, _) =
        run_prompt_verify(&repo_dir.path().to_path_buf(), "prompt.md", false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(report["decision"], "pass");
    assert_eq!(report["claims_total"], 1);
    assert_eq!(report["claims_verified"], 1);
    assert_eq!(report["claims_invalid"], 0);
    assert_eq!(report["claims_review"], 0);
}

#[test]
fn prompt_verifier_rejects_repo_local_symlink_escape_as_claim_invalid() {
    use std::os::unix::fs::symlink;

    let repo_dir = tempdir().unwrap();
    let outside_dir = tempdir().unwrap();
    fs::write(outside_dir.path().join("secret.txt"), "outside\n").unwrap();
    symlink(
        outside_dir.path().join("secret.txt"),
        repo_dir.path().join("leak.txt"),
    )
    .unwrap();
    fs::write(repo_dir.path().join("prompt.md"), "- leak.txt:1\n").unwrap();

    let (output, _dir, json_path, _) =
        run_prompt_verify(&repo_dir.path().to_path_buf(), "prompt.md", false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(report["decision"], "fail");
    assert_eq!(report["claims_invalid"], 1);
    assert_eq!(report["claims"][0]["note"], "path escapes repo");
}

#[test]
fn prompt_verifier_does_not_verify_symbols_from_comments_or_strings() {
    let repo_dir = tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("src")).unwrap();
    fs::write(repo_dir.path().join("prompt.md"), "- good::build_client\n").unwrap();
    fs::write(
        repo_dir.path().join("src/good.rs"),
        "// pub fn build_client() {}\nconst TEXT: &str = \"fn build_client\";\n",
    )
    .unwrap();

    let (output, _dir, json_path, _) =
        run_prompt_verify(&repo_dir.path().to_path_buf(), "prompt.md", false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(report["decision"], "fail");
    assert_eq!(report["claims_invalid"], 1);
}

#[test]
fn prompt_verifier_marks_rustish_class_base_claim_as_review() {
    let repo_dir = tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join("src")).unwrap();
    fs::write(
        repo_dir.path().join("prompt.md"),
        "- class Model(BaseRunner)\n",
    )
    .unwrap();
    fs::write(repo_dir.path().join("src/lib.rs"), "pub struct Model;\n").unwrap();

    let (output, _dir, json_path, _) =
        run_prompt_verify(&repo_dir.path().to_path_buf(), "prompt.md", false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(report["decision"], "review");
    assert_eq!(report["claims_review"], 1);
}

#[test]
fn prompt_verifier_accepts_python_and_typescript_provider_import_variants() {
    let cases = [
        (
            "py-openai",
            "runner.py",
            "import openai\n\n\ndef run():\n    return openai.responses.create(model=\"x\", input=\"hi\")\n",
        ),
        (
            "py-anthropic",
            "runner.py",
            "import anthropic\n\n\ndef run():\n    client = anthropic.Anthropic()\n    return client.messages.create(model=\"x\", messages=[])\n",
        ),
        (
            "ts-openai",
            "runner.ts",
            "import OpenAI from \"openai\";\n\nexport function run() {\n  const client = new OpenAI();\n  return client.responses.create({ model: \"x\", input: \"hi\" });\n}\n",
        ),
        (
            "ts-langchain",
            "runner.ts",
            "import { ChatOpenAI } from \"@langchain/openai\";\n\nexport function run() {\n  const model = new ChatOpenAI();\n  return model.invoke(\"hi\");\n}\n",
        ),
    ];

    for (name, file, source) in cases {
        let repo_dir = tempdir().unwrap();
        fs::write(repo_dir.path().join("prompt.md"), "- LLM call\n").unwrap();
        fs::write(repo_dir.path().join(file), source).unwrap();

        let (output, _dir, json_path, _) =
            run_prompt_verify(&repo_dir.path().to_path_buf(), "prompt.md", false);
        assert!(
            output.status.success(),
            "{name} stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
        assert_eq!(report["decision"], "pass", "{name}");
        assert_eq!(report["claims_verified"], 1, "{name}");
    }
}

#[test]
fn prompt_verifier_marks_multiple_llm_call_sites_as_review() {
    let repo_dir = tempdir().unwrap();
    fs::write(repo_dir.path().join("prompt.md"), "- LLM call\n").unwrap();
    fs::write(
        repo_dir.path().join("one.py"),
        "import openai\n\n\ndef run():\n    return openai.responses.create(model=\"x\", input=\"hi\")\n",
    )
    .unwrap();
    fs::write(
        repo_dir.path().join("two.py"),
        "import anthropic\n\n\ndef run():\n    client = anthropic.Anthropic()\n    return client.messages.create(model=\"x\", messages=[])\n",
    )
    .unwrap();

    let (output, _dir, json_path, _) =
        run_prompt_verify(&repo_dir.path().to_path_buf(), "prompt.md", false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).unwrap();
    assert_eq!(report["decision"], "review");
    assert_eq!(report["claims_review"], 1);
}
