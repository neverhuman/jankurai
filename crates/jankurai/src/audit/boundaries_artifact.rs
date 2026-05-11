//! Load and summarize `agent/boundaries.toml` for audit reports.

use crate::model::BoundariesManifestSummary;
use crate::validation;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

const BOUNDARIES_REL: &str = "agent/boundaries.toml";

pub fn load_manifest_summary(root: &Path) -> Option<BoundariesManifestSummary> {
    let path = root.join(BOUNDARIES_REL);
    if !path.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(&path).ok()?;
    let value = validation::validate_boundaries_toml_text(root, &text).ok()?;
    let content_fingerprint = format!("sha256:{:x}", Sha256::digest(text.as_bytes()));
    Some(summarize(&value, BOUNDARIES_REL, content_fingerprint))
}

fn summarize(
    value: &Value,
    rel_path: &str,
    content_fingerprint: String,
) -> BoundariesManifestSummary {
    let stack_id = value
        .get("stack")
        .and_then(|s| s.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let stack_version = value
        .get("stack")
        .and_then(|s| s.get("version"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let queues = value.get("queues");
    let adapter_path_count = queues
        .and_then(|q| q.get("adapter_paths"))
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    let event_contract_path_count = queues
        .and_then(|q| q.get("event_contract_paths"))
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    let generated_type_path_count = queues
        .and_then(|q| q.get("generated_type_paths"))
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    let client_marker_count = queues
        .and_then(|q| q.get("client_markers"))
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);

    let streaming_exception_count = value
        .get("streaming_exception")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);

    BoundariesManifestSummary {
        path: rel_path.into(),
        content_fingerprint,
        stack_id,
        stack_version,
        adapter_path_count,
        event_contract_path_count,
        generated_type_path_count,
        client_marker_count,
        streaming_exception_count,
    }
}
