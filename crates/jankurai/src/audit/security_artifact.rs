//! Load and summarize `target/jankurai/security/evidence.json` for audit reports.

use crate::model::SecurityEvidenceArtifactSummary;
use crate::validation::{self, ArtifactSchema};
use serde_json::Value;
use std::path::Path;

const SECURITY_EVIDENCE_REL: &str = "target/jankurai/security/evidence.json";

pub fn load_report_summary(root: &Path) -> Option<SecurityEvidenceArtifactSummary> {
    let path = root.join(SECURITY_EVIDENCE_REL);
    if !path.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(&path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    validation::validate_value(root, ArtifactSchema::SecurityEvidence, &value).ok()?;
    summarize(&value)
}

/// Validate and summarize already-parsed JSON (for tests).
pub fn summarize_validated_envelope(
    root: &Path,
    value: &Value,
) -> Option<SecurityEvidenceArtifactSummary> {
    validation::validate_value(root, ArtifactSchema::SecurityEvidence, value).ok()?;
    summarize(value)
}

fn summarize(value: &Value) -> Option<SecurityEvidenceArtifactSummary> {
    let envelope_exit_code = value.get("exit_code")?.as_i64()? as i32;
    let elapsed_ms = value.get("elapsed_ms")?.as_u64()?;
    let wrapper_strict = value
        .get("wrapper")
        .and_then(|w| w.get("strict"))
        .and_then(Value::as_bool)?;
    let profile = value
        .get("policy")
        .and_then(|p| p.get("profile"))
        .and_then(Value::as_str)
        .unwrap_or("local")
        .to_string();

    let mut commands_ran = 0usize;
    let mut commands_skipped = 0usize;
    let mut commands_failed = 0usize;
    let mut required_commands_skipped = 0usize;
    let mut required_commands_failed = 0usize;
    let mut blocking_commands = Vec::new();
    if let Some(arr) = value.get("commands").and_then(Value::as_array) {
        for cmd in arr {
            let status = cmd.get("status").and_then(Value::as_str);
            let required = cmd
                .get("required_by_policy")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            match status {
                Some("ran") => commands_ran += 1,
                Some("skipped") => {
                    commands_skipped += 1;
                    if required {
                        required_commands_skipped += 1;
                    }
                }
                Some("failed") => {
                    commands_failed += 1;
                    if required {
                        required_commands_failed += 1;
                    }
                }
                _ => {}
            }
            if cmd
                .get("blocking")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                let label = cmd
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                blocking_commands.push(label.to_string());
            }
        }
    }

    let generated_at = value
        .get("generated_at")
        .and_then(Value::as_str)
        .map(str::to_string);
    let git_head = value
        .get("git_head")
        .and_then(Value::as_str)
        .map(str::to_string);

    Some(SecurityEvidenceArtifactSummary {
        path: SECURITY_EVIDENCE_REL.into(),
        envelope_exit_code,
        elapsed_ms,
        wrapper_strict,
        profile,
        commands_ran,
        commands_skipped,
        commands_failed,
        required_commands_skipped,
        required_commands_failed,
        blocking_commands,
        generated_at,
        git_head,
    })
}
