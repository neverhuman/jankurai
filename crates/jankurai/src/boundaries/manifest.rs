use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BoundaryManifest {
    pub stack: Option<Stack>,
    #[serde(default)]
    pub rust: Option<RustBoundary>,
    #[serde(default)]
    pub typescript: Option<TypeScriptBoundary>,
    #[serde(default)]
    pub python: Option<PythonBoundary>,
    pub queues: Option<QueueBoundary>,
    pub db: Option<DbBoundary>,
    #[serde(default)]
    pub streaming_exception: Vec<StreamingException>,
    #[serde(default)]
    pub audited_runtime_boundary: Vec<AuditedRuntimeBoundary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Stack {
    pub id: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct RustBoundary {
    #[serde(default)]
    pub domain_paths: Vec<String>,
    #[serde(default)]
    pub forbidden_domain_imports: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TypeScriptBoundary {
    #[serde(default)]
    pub web_paths: Vec<String>,
    #[serde(default)]
    pub forbidden_web_imports: Vec<String>,
    #[serde(default)]
    pub generated_contract_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PythonBoundary {
    #[serde(default)]
    pub allowed_truth_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct QueueBoundary {
    #[serde(default)]
    pub adapter_paths: Vec<String>,
    #[serde(default)]
    pub event_contract_paths: Vec<String>,
    #[serde(default)]
    pub generated_type_paths: Vec<String>,
    #[serde(default)]
    pub client_markers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DbBoundary {
    #[serde(default)]
    pub root_paths: Vec<String>,
    #[serde(default)]
    pub migration_paths: Vec<String>,
    #[serde(default)]
    pub constraint_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamingException {
    pub runtime: String,
    pub classification: Option<String>,
    pub reason: Option<String>,
    pub owner: String,
    pub expires: String,
    pub migration_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuditedRuntimeBoundary {
    pub id: String,
    #[serde(default)]
    pub paths: Vec<String>,
    pub classification: String,
    #[serde(default)]
    pub product_surface: bool,
    pub runtime_language: String,
    #[serde(default)]
    pub target_stack_exception: bool,
    #[serde(default)]
    pub reclassifies: Vec<String>,
    pub proof_command: Option<String>,
    pub rerun_command: Option<String>,
    #[serde(default)]
    pub required_evidence: Vec<String>,
    #[serde(default)]
    pub required_checks: Vec<String>,
}

pub fn parse(text: &str) -> Result<BoundaryManifest> {
    Ok(toml::from_str(text)?)
}

pub fn load(path: &Path) -> Result<BoundaryManifest> {
    parse(&fs::read_to_string(path)?)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_queue_streaming_db_and_language_boundaries() {
        let text = r#"
[stack]
id = "rust-ts-vite-react-postgres-bounded-python"
version = "0.5.0"

[rust]
domain_paths = ["crates/domain", "crates/*/src/domain"]
forbidden_domain_imports = ["std::fs", "std::env", "sqlx::"]

[typescript]
web_paths = ["apps/web", "packages/web", "packages/ui"]
forbidden_web_imports = ["pg", "postgres", "better-sqlite3"]
generated_contract_paths = ["contracts/generated"]

[python]
allowed_truth_paths = ["python/ai-service"]

[queues]
adapter_paths = ["crates/adapters/queues", "crates/adapters/src/queues"]
event_contract_paths = ["contracts/events"]
generated_type_paths = ["contracts/generated"]
client_markers = ["rdkafka", "kafkajs"]

[db]
root_paths = ["db"]
migration_paths = ["db/migrations"]
constraint_paths = ["db/constraints"]

[[streaming_exception]]
runtime = "kafka"
classification = "brownfield"
owner = "platform"
expires = "2026-12-31"
migration_path = "Keep Kafka behind queue adapters."

[[audited_runtime_boundary]]
id = "runtime-payload"
paths = ["runtime_payload/python/**/*.py"]
classification = "audited-runtime-payload"
product_surface = true
runtime_language = "python"
target_stack_exception = true
reclassifies = [
  "non-optimal-product-language-found",
  "too-much-python-in-product-surface",
  "python-direct-product-truth-or-db-ownership"
]
proof_command = "python tools/check_runtime_payload_boundary.py"
rerun_command = "python tools/check_runtime_payload_boundary.py"
required_evidence = ["target/jankurai/boundaries/runtime-payload/evidence.json"]
required_checks = ["manifest-coverage", "payload-hash-match", "no-direct-db-access"]
"#;
        let manifest = parse(text).unwrap();
        assert_eq!(
            manifest.stack.as_ref().map(|stack| stack.id.as_str()),
            Some("rust-ts-vite-react-postgres-bounded-python")
        );
        assert_eq!(
            manifest
                .queues
                .as_ref()
                .map(|queues| queues.adapter_paths.len()),
            Some(2)
        );
        assert_eq!(
            manifest
                .rust
                .as_ref()
                .map(|rust| rust.domain_paths.as_slice()),
            Some(
                &[
                    "crates/domain".to_string(),
                    "crates/*/src/domain".to_string()
                ][..]
            )
        );
        assert_eq!(
            manifest
                .typescript
                .as_ref()
                .map(|ts| ts.generated_contract_paths.as_slice()),
            Some(&["contracts/generated".to_string()][..])
        );
        assert_eq!(
            manifest
                .python
                .as_ref()
                .map(|python| python.allowed_truth_paths.as_slice()),
            Some(&["python/ai-service".to_string()][..])
        );
        assert_eq!(
            manifest.db.as_ref().map(|db| db.root_paths.as_slice()),
            Some(&["db".to_string()][..])
        );
        assert_eq!(
            manifest.streaming_exception[0].classification.as_deref(),
            Some("brownfield")
        );
        assert_eq!(manifest.audited_runtime_boundary[0].id, "runtime-payload");
        assert!(manifest.audited_runtime_boundary[0].target_stack_exception);
    }
}
