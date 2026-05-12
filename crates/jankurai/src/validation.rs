use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum ArtifactSchema {
    AdoptionPlan,
    RepoScore,
    ProofPlan,
    ProofReceipt,
    ProofBindWitness,
    ProofBindObligations,
    ProofMarkReceipt,
    EvidenceIndex,
    ProofVerification,
    DoctorReceipt,
    InitReceipt,
    InitProfile,
    SecurityPolicy,
    ToolAdoption,
    SecurityEvidence,
    ContextPack,
    Kickoff,
    RepairPlan,
    RepairRun,
    RepairPrDraft,
    OptimizationReport,
    ExceptionExpiryReport,
    Boundaries,
    BoundaryEvidence,
    UxQaPolicy,
    UxQaReport,
    MigrationReport,
    MigrationPlan,
    MigrationPromptVerification,
    MigrationSliceRisk,
    Postmortem,
    BenchmarkSuite,
    BenchmarkReport,
    Certification,
    CertificationBadge,
    ConformanceResults,
    GovernancePolicy,
    PublicEvidenceBundle,
    CellManifest,
    CellRegistry,
    AuditPolicy,
    OwnerMap,
    TestMap,
    GeneratedZones,
    ProofLanes,
    StandardVersion,
    UpdatePlan,
    UpdateReceipt,
    Finding,
    RepairQueueItem,
    RuleRegistry,
    RuleVerify,
    MergeWitness,
    ScoreDiff,
    ScoreHistoryEntry,
    ScoreHistoryExport,
    ScoreTrend,
    VibeCoverageSource,
    VibeCoverageReport,
    CoverageSources,
    CoverageAudit,
    ReadmeBadge,
    CopyCode,
}

impl ArtifactSchema {
    pub fn rel_path(self) -> &'static str {
        match self {
            Self::RepoScore => "repo-score.schema.json",
            Self::AdoptionPlan => "adoption-plan.schema.json",
            Self::ProofPlan => "proof-plan.schema.json",
            Self::ProofReceipt => "proof-receipt.schema.json",
            Self::ProofBindWitness => "proofbind-witness.schema.json",
            Self::ProofBindObligations => "proofbind-obligations.schema.json",
            Self::ProofMarkReceipt => "proofmark-receipt.schema.json",
            Self::EvidenceIndex => "evidence-index.schema.json",
            Self::ProofVerification => "proof-verification.schema.json",
            Self::DoctorReceipt => "doctor-receipt.schema.json",
            Self::InitReceipt => "init-receipt.schema.json",
            Self::InitProfile => "init-profile.schema.json",
            Self::SecurityPolicy => "security-policy.schema.json",
            Self::ToolAdoption => "tool-adoption.schema.json",
            Self::SecurityEvidence => "security-evidence.schema.json",
            Self::ContextPack => "context-pack.schema.json",
            Self::Kickoff => "kickoff.schema.json",
            Self::RepairPlan => "repair-plan.schema.json",
            Self::RepairRun => "repair-run.schema.json",
            Self::RepairPrDraft => "repair-pr-draft.schema.json",
            Self::OptimizationReport => "optimization-report.schema.json",
            Self::ExceptionExpiryReport => "exception-expiry-report.schema.json",
            Self::Boundaries => "boundaries.schema.json",
            Self::BoundaryEvidence => "boundary-evidence.schema.json",
            Self::UxQaPolicy => "ux-qa-policy.schema.json",
            Self::UxQaReport => "ux-qa.schema.json",
            Self::MigrationReport => "migration-report.schema.json",
            Self::MigrationPlan => "migration-plan.schema.json",
            Self::MigrationPromptVerification => "migration-prompt-verification.schema.json",
            Self::MigrationSliceRisk => "migration-slice-risk.schema.json",
            Self::Postmortem => "postmortem.schema.json",
            Self::BenchmarkSuite => "benchmark-suite.schema.json",
            Self::BenchmarkReport => "benchmark-report.schema.json",
            Self::Certification => "certification.schema.json",
            Self::CertificationBadge => "certification-badge.schema.json",
            Self::ConformanceResults => "conformance-results.schema.json",
            Self::GovernancePolicy => "governance-policy.schema.json",
            Self::PublicEvidenceBundle => "public-evidence-bundle.schema.json",
            Self::CellManifest => "cell-manifest.schema.json",
            Self::CellRegistry => "cell-registry.schema.json",
            Self::AuditPolicy => "audit-policy.schema.json",
            Self::OwnerMap => "owner-map.schema.json",
            Self::TestMap => "test-map.schema.json",
            Self::GeneratedZones => "generated-zones.schema.json",
            Self::ProofLanes => "proof-lanes.schema.json",
            Self::StandardVersion => "standard-version.schema.json",
            Self::UpdatePlan => "update-plan.schema.json",
            Self::UpdateReceipt => "update-receipt.schema.json",
            Self::Finding => "finding.schema.json",
            Self::RepairQueueItem => "repair-queue.schema.json",
            Self::RuleRegistry => "rule-registry.schema.json",
            Self::RuleVerify => "rule-verify.schema.json",
            Self::MergeWitness => "merge-witness.schema.json",
            Self::ScoreDiff => "score-diff.schema.json",
            Self::ScoreHistoryEntry => "score-history-entry.schema.json",
            Self::ScoreHistoryExport => "score-history-export.schema.json",
            Self::ScoreTrend => "score-trend.schema.json",
            Self::VibeCoverageSource => "vibe-coverage-source.schema.json",
            Self::VibeCoverageReport => "vibe-coverage-report.schema.json",
            Self::CoverageSources => "coverage-sources.schema.json",
            Self::CoverageAudit => "coverage-audit.schema.json",
            Self::ReadmeBadge => "readme-badge.schema.json",
            Self::CopyCode => "copy-code.schema.json",
        }
    }
}

pub fn validate_value(repo: &Path, schema: ArtifactSchema, value: &Value) -> Result<()> {
    let mut ctx = Validator::new(schema_root(repo));
    let schema_rel = schema.rel_path();
    let schema_value = ctx.load_schema(schema_rel)?;
    ctx.validate_schema(schema_rel, &schema_value, value, "$")?;
    Ok(())
}

/// Parse `agent/boundaries.toml` as TOML and validate the JSON-shaped value against `boundaries.schema.json`.
pub fn validate_boundaries_toml_text(repo: &Path, text: &str) -> Result<Value> {
    let toml_value: toml::Value = toml::from_str(text).context("parse agent/boundaries.toml")?;
    let json_value = serde_json::to_value(&toml_value)
        .context("convert boundaries TOML to JSON for schema validation")?;
    validate_value(repo, ArtifactSchema::Boundaries, &json_value)?;
    Ok(json_value)
}

/// Parse `agent/ux-qa.toml` as TOML and validate the JSON-shaped value against `ux-qa-policy.schema.json`.
pub fn validate_ux_qa_policy_toml_text(repo: &Path, text: &str) -> Result<Value> {
    let toml_value: toml::Value = toml::from_str(text).context("parse agent/ux-qa.toml")?;
    let json_value = serde_json::to_value(&toml_value)
        .context("convert UX QA policy TOML to JSON for schema validation")?;
    validate_value(repo, ArtifactSchema::UxQaPolicy, &json_value)?;
    Ok(json_value)
}

/// Parse `agent/security-policy.toml` as TOML and validate the JSON-shaped value against `security-policy.schema.json`.
pub fn validate_security_policy_toml_text(repo: &Path, text: &str) -> Result<Value> {
    let toml_value: toml::Value =
        toml::from_str(text).context("parse agent/security-policy.toml")?;
    let json_value = serde_json::to_value(&toml_value)
        .context("convert security policy TOML to JSON for schema validation")?;
    validate_value(repo, ArtifactSchema::SecurityPolicy, &json_value)?;
    Ok(json_value)
}

/// Parse `agent/tool-adoption.toml` as TOML and validate the JSON-shaped value against `tool-adoption.schema.json`.
pub fn validate_tool_adoption_toml_text(repo: &Path, text: &str) -> Result<Value> {
    let toml_value: toml::Value = toml::from_str(text).context("parse agent/tool-adoption.toml")?;
    let json_value = serde_json::to_value(&toml_value)
        .context("convert tool adoption TOML to JSON for schema validation")?;
    validate_value(repo, ArtifactSchema::ToolAdoption, &json_value)?;
    Ok(json_value)
}

/// Parse `agent/audit-policy.toml` and validate against `audit-policy.schema.json`.
pub fn validate_audit_policy_toml_text(repo: &Path, text: &str) -> Result<Value> {
    let toml_value: toml::Value = toml::from_str(text).context("parse agent/audit-policy.toml")?;
    let json_value = serde_json::to_value(&toml_value)
        .context("convert audit policy TOML to JSON for schema validation")?;
    validate_value(repo, ArtifactSchema::AuditPolicy, &json_value)?;
    Ok(json_value)
}

pub fn validate_owner_map_json_text(repo: &Path, text: &str) -> Result<Value> {
    let v: Value = serde_json::from_str(text).context("parse agent/owner-map.json")?;
    validate_value(repo, ArtifactSchema::OwnerMap, &v)?;
    Ok(v)
}

pub fn validate_test_map_json_text(repo: &Path, text: &str) -> Result<Value> {
    let v: Value = serde_json::from_str(text).context("parse agent/test-map.json")?;
    validate_value(repo, ArtifactSchema::TestMap, &v)?;
    Ok(v)
}

pub fn validate_generated_zones_toml_text(repo: &Path, text: &str) -> Result<Value> {
    let toml_value: toml::Value =
        toml::from_str(text).context("parse agent/generated-zones.toml")?;
    let json_value = serde_json::to_value(&toml_value)
        .context("convert generated-zones TOML to JSON for schema validation")?;
    validate_value(repo, ArtifactSchema::GeneratedZones, &json_value)?;
    Ok(json_value)
}

pub fn validate_proof_lanes_toml_text(repo: &Path, text: &str) -> Result<Value> {
    let toml_value: toml::Value = toml::from_str(text).context("parse agent/proof-lanes.toml")?;
    let json_value = serde_json::to_value(&toml_value)
        .context("convert proof-lanes TOML to JSON for schema validation")?;
    validate_value(repo, ArtifactSchema::ProofLanes, &json_value)?;
    Ok(json_value)
}

pub fn validate_standard_version_toml_text(repo: &Path, text: &str) -> Result<Value> {
    let toml_value: toml::Value =
        toml::from_str(text).context("parse agent/standard-version.toml")?;
    let json_value = serde_json::to_value(&toml_value)
        .context("convert standard-version TOML to JSON for schema validation")?;
    validate_value(repo, ArtifactSchema::StandardVersion, &json_value)?;
    Ok(json_value)
}

pub fn validate_vibe_coverage_source_toml_text(repo: &Path, text: &str) -> Result<Value> {
    let toml_value: toml::Value = toml::from_str(text).context("parse agent/vibe-coverage.toml")?;
    let json_value = serde_json::to_value(&toml_value)
        .context("convert vibe coverage TOML to JSON for schema validation")?;
    validate_value(repo, ArtifactSchema::VibeCoverageSource, &json_value)?;
    Ok(json_value)
}

pub fn validate_coverage_sources_toml_text(repo: &Path, text: &str) -> Result<Value> {
    let toml_value: toml::Value =
        toml::from_str(text).context("parse agent/coverage-sources.toml")?;
    let json_value = serde_json::to_value(&toml_value)
        .context("convert coverage sources TOML to JSON for schema validation")?;
    validate_value(repo, ArtifactSchema::CoverageSources, &json_value)?;
    Ok(json_value)
}

pub fn validate_serializable<T: Serialize>(
    repo: &Path,
    schema: ArtifactSchema,
    value: &T,
) -> Result<()> {
    let json = serde_json::to_value(value)?;
    validate_value(repo, schema, &json)
}

pub fn write_json<T: Serialize>(
    repo: &Path,
    schema: ArtifactSchema,
    path: &str,
    value: &T,
) -> Result<()> {
    validate_serializable(repo, schema, value)?;
    let text = serde_json::to_string_pretty(value)?;
    crate::render::write_json(path, &text)
}

struct Validator {
    schema_root: PathBuf,
    cache: HashMap<String, Value>,
}

impl Validator {
    fn new(schema_root: PathBuf) -> Self {
        Self {
            schema_root,
            cache: HashMap::new(),
        }
    }

    fn load_schema(&mut self, schema_rel: &str) -> Result<Value> {
        if let Some(schema) = self.cache.get(schema_rel) {
            return Ok(schema.clone());
        }
        let path = self.schema_root.join(schema_rel);
        let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let value: Value =
            serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        validate_supported_schema_keywords(schema_rel, &value)?;
        self.cache.insert(schema_rel.to_string(), value.clone());
        Ok(value)
    }

    fn validate_schema(
        &mut self,
        schema_rel: &str,
        schema: &Value,
        instance: &Value,
        path: &str,
    ) -> Result<()> {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            let (target_rel, target_schema) = self.resolve_ref(schema_rel, schema, reference)?;
            return self.validate_schema(&target_rel, &target_schema, instance, path);
        }

        if let Some(constant) = schema.get("const") {
            if instance != constant {
                bail!("{path}: expected constant {constant}, found {instance}");
            }
        }

        if let Some(enum_values) = schema.get("enum").and_then(Value::as_array) {
            if !enum_values.iter().any(|candidate| candidate == instance) {
                bail!("{path}: value {instance} is not in enum {enum_values:?}");
            }
        }

        if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
            let text = instance
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("{path}: expected string matching {pattern}"))?;
            let regex = Regex::new(pattern)
                .with_context(|| format!("compile schema pattern `{pattern}`"))?;
            if !regex.is_match(text) {
                bail!("{path}: `{text}` does not match pattern `{pattern}`");
            }
        }

        if let Some(ty) = schema.get("type") {
            self.validate_type(schema_rel, schema, ty, instance, path)?;
        }

        if let Some(all_of) = schema.get("allOf").and_then(Value::as_array) {
            for (idx, branch) in all_of.iter().enumerate() {
                self.validate_schema(
                    schema_rel,
                    branch,
                    instance,
                    &format!("{path}/allOf[{idx}]"),
                )?;
            }
        }

        if let Some(any_of) = schema.get("anyOf").and_then(Value::as_array) {
            let mut errors = Vec::new();
            for (idx, branch) in any_of.iter().enumerate() {
                match self.validate_schema(
                    schema_rel,
                    branch,
                    instance,
                    &format!("{path}/anyOf[{idx}]"),
                ) {
                    Ok(()) => return Ok(()),
                    Err(error) => errors.push(error.to_string()),
                }
            }
            bail!(
                "{path}: none of anyOf branches matched ({})",
                errors.join("; ")
            );
        }

        if let Some(one_of) = schema.get("oneOf").and_then(Value::as_array) {
            let mut matches = 0usize;
            for (idx, branch) in one_of.iter().enumerate() {
                if self
                    .validate_schema(
                        schema_rel,
                        branch,
                        instance,
                        &format!("{path}/oneOf[{idx}]"),
                    )
                    .is_ok()
                {
                    matches += 1;
                }
            }
            if matches != 1 {
                bail!("{path}: expected exactly one matching oneOf branch, found {matches}");
            }
        }

        Ok(())
    }

    fn validate_type(
        &mut self,
        schema_rel: &str,
        schema: &Value,
        ty: &Value,
        instance: &Value,
        path: &str,
    ) -> Result<()> {
        if let Some(ty) = ty.as_str() {
            self.validate_single_type(schema_rel, schema, ty, instance, path)
        } else if let Some(types) = ty.as_array() {
            let mut errors = Vec::new();
            for candidate in types {
                if let Some(candidate) = candidate.as_str() {
                    if self
                        .validate_single_type(schema_rel, schema, candidate, instance, path)
                        .is_ok()
                    {
                        return Ok(());
                    }
                    errors.push(candidate.to_string());
                }
            }
            bail!("{path}: value {instance} does not satisfy any allowed type {errors:?}");
        } else {
            Ok(())
        }
    }

    fn validate_single_type(
        &mut self,
        schema_rel: &str,
        schema: &Value,
        ty: &str,
        instance: &Value,
        path: &str,
    ) -> Result<()> {
        match ty {
            "object" => self.validate_object(schema_rel, schema, instance, path),
            "array" => self.validate_array(schema_rel, schema, instance, path),
            "string" => {
                if instance.is_string() {
                    self.validate_string_constraints(schema, instance, path)?;
                    Ok(())
                } else {
                    bail!("{path}: expected string, found {instance}")
                }
            }
            "integer" => {
                if is_integer(instance) {
                    self.validate_numeric_constraints(schema, instance, path)?;
                    Ok(())
                } else {
                    bail!("{path}: expected integer, found {instance}")
                }
            }
            "number" => {
                if instance.is_number() {
                    self.validate_numeric_constraints(schema, instance, path)?;
                    Ok(())
                } else {
                    bail!("{path}: expected number, found {instance}")
                }
            }
            "boolean" => {
                if instance.is_boolean() {
                    Ok(())
                } else {
                    bail!("{path}: expected boolean, found {instance}")
                }
            }
            "null" => {
                if instance.is_null() {
                    Ok(())
                } else {
                    bail!("{path}: expected null, found {instance}")
                }
            }
            _ => Ok(()),
        }
    }

    fn validate_object(
        &mut self,
        schema_rel: &str,
        schema: &Value,
        instance: &Value,
        path: &str,
    ) -> Result<()> {
        let Some(object) = instance.as_object() else {
            bail!("{path}: expected object, found {instance}");
        };

        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for key in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(key) {
                    bail!("{path}: missing required property `{key}`");
                }
            }
        }

        let mut defined = Vec::new();
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for (key, prop_schema) in properties {
                defined.push(key.clone());
                if let Some(value) = object.get(key) {
                    self.validate_schema(schema_rel, prop_schema, value, &format!("{path}/{key}"))?;
                }
            }
        }

        match schema.get("additionalProperties") {
            Some(Value::Bool(false)) => {
                for key in object.keys() {
                    if !defined.iter().any(|candidate| candidate == key) {
                        bail!("{path}: additional property `{key}` is not allowed");
                    }
                }
            }
            Some(Value::Object(additional_schema)) => {
                for (key, value) in object {
                    if !defined.iter().any(|candidate| candidate == key) {
                        self.validate_schema(
                            schema_rel,
                            &Value::Object(additional_schema.clone()),
                            value,
                            &format!("{path}/{key}"),
                        )?;
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn validate_array(
        &mut self,
        schema_rel: &str,
        schema: &Value,
        instance: &Value,
        path: &str,
    ) -> Result<()> {
        let Some(items) = instance.as_array() else {
            bail!("{path}: expected array, found {instance}");
        };

        if let Some(min_items) = schema.get("minItems").and_then(Value::as_u64) {
            if items.len() < min_items as usize {
                bail!(
                    "{path}: expected at least {min_items} items, found {}",
                    items.len()
                );
            }
        }

        if let Some(max_items) = schema.get("maxItems").and_then(Value::as_u64) {
            if items.len() > max_items as usize {
                bail!(
                    "{path}: expected at most {max_items} items, found {}",
                    items.len()
                );
            }
        }

        if let Some(item_schema) = schema.get("items") {
            match item_schema {
                Value::Object(_) => {
                    for (idx, item) in items.iter().enumerate() {
                        self.validate_schema(
                            schema_rel,
                            item_schema,
                            item,
                            &format!("{path}[{idx}]"),
                        )?;
                    }
                }
                Value::Array(schemas) => {
                    for (idx, item) in items.iter().enumerate() {
                        if let Some(element_schema) = schemas.get(idx) {
                            self.validate_schema(
                                schema_rel,
                                element_schema,
                                item,
                                &format!("{path}[{idx}]"),
                            )?;
                        }
                    }
                }
                _ => {}
            }
        }

        if schema
            .get("uniqueItems")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let mut seen = Vec::new();
            for (idx, item) in items.iter().enumerate() {
                let encoded = serde_json::to_string(item)
                    .context("serialize array item for uniqueItems validation")?;
                if seen.iter().any(|candidate| candidate == &encoded) {
                    bail!("{path}[{idx}]: duplicate array item violates uniqueItems");
                }
                seen.push(encoded);
            }
        }

        Ok(())
    }

    fn validate_string_constraints(
        &self,
        schema: &Value,
        instance: &Value,
        path: &str,
    ) -> Result<()> {
        let Some(text) = instance.as_str() else {
            return Ok(());
        };

        if let Some(min_len) = schema.get("minLength").and_then(Value::as_u64) {
            let len = text.chars().count() as u64;
            if len < min_len {
                bail!("{path}: string length {len} below minimum {min_len}");
            }
        }

        if let Some(max_len) = schema.get("maxLength").and_then(Value::as_u64) {
            let len = text.chars().count() as u64;
            if len > max_len {
                bail!("{path}: string length {len} above maximum {max_len}");
            }
        }

        Ok(())
    }

    fn validate_numeric_constraints(
        &self,
        schema: &Value,
        instance: &Value,
        path: &str,
    ) -> Result<()> {
        let Some(value) = instance.as_f64() else {
            return Ok(());
        };

        if let Some(minimum) = schema.get("minimum").and_then(Value::as_f64) {
            if value < minimum {
                bail!("{path}: value {value} below minimum {minimum}");
            }
        }

        if let Some(maximum) = schema.get("maximum").and_then(Value::as_f64) {
            if value > maximum {
                bail!("{path}: value {value} above maximum {maximum}");
            }
        }

        Ok(())
    }

    fn resolve_ref(
        &mut self,
        schema_rel: &str,
        _schema: &Value,
        reference: &str,
    ) -> Result<(String, Value)> {
        if let Some(fragment) = reference.strip_prefix('#') {
            let root_schema = self.load_schema(schema_rel)?;
            let target = resolve_fragment(&root_schema, fragment)
                .with_context(|| format!("resolve internal ref `{reference}` in `{schema_rel}`"))?;
            return Ok((schema_rel.to_string(), target.clone()));
        }

        let (target_rel, fragment) = split_reference(reference);
        let target_schema = self.load_schema(&target_rel)?;
        if let Some(fragment) = fragment {
            let target = resolve_fragment(&target_schema, &fragment).with_context(|| {
                format!("resolve ref `{reference}` from `{schema_rel}` to `{target_rel}`")
            })?;
            Ok((target_rel, target.clone()))
        } else {
            Ok((target_rel, target_schema))
        }
    }
}

fn split_reference(reference: &str) -> (String, Option<String>) {
    let mut parts = reference.splitn(2, '#');
    let file = parts.next().unwrap_or_default().to_string();
    let fragment = parts
        .next()
        .map(|fragment| format!("#{fragment}"))
        .filter(|fragment| fragment != "#");
    (file, fragment)
}

fn resolve_fragment<'a>(schema: &'a Value, fragment: &str) -> Result<&'a Value> {
    let mut current = schema;
    let mut pointer = fragment.trim_start_matches('#');
    if pointer.is_empty() {
        return Ok(current);
    }
    if pointer.starts_with('/') {
        pointer = &pointer[1..];
    }
    for part in pointer.split('/') {
        let part = part.replace("~1", "/").replace("~0", "~");
        current = current
            .get(&part)
            .ok_or_else(|| anyhow::anyhow!("missing schema fragment `{fragment}`"))?;
    }
    Ok(current)
}

fn validate_supported_schema_keywords(schema_rel: &str, schema: &Value) -> Result<()> {
    validate_supported_schema_keywords_at(schema_rel, schema, "$")
}

fn validate_supported_schema_keywords_at(
    schema_rel: &str,
    schema: &Value,
    path: &str,
) -> Result<()> {
    let Some(object) = schema.as_object() else {
        return Ok(());
    };

    for (key, value) in object {
        match key.as_str() {
            "$schema"
            | "$id"
            | "$ref"
            | "$defs"
            | "title"
            | "description"
            | "default"
            | "examples"
            | "type"
            | "const"
            | "enum"
            | "required"
            | "properties"
            | "additionalProperties"
            | "items"
            | "minItems"
            | "maxItems"
            | "uniqueItems"
            | "minimum"
            | "maximum"
            | "minLength"
            | "maxLength"
            | "pattern"
            | "allOf"
            | "anyOf"
            | "oneOf" => {}
            unsupported => {
                bail!("{schema_rel}:{path}: unsupported schema keyword `{unsupported}`");
            }
        }

        match key.as_str() {
            "properties" | "$defs" => {
                if let Some(children) = value.as_object() {
                    for (child_key, child_schema) in children {
                        validate_supported_schema_keywords_at(
                            schema_rel,
                            child_schema,
                            &format!("{path}/{key}/{child_key}"),
                        )?;
                    }
                }
            }
            "items" | "additionalProperties" => {
                validate_nested_schema_value(schema_rel, value, &format!("{path}/{key}"))?;
            }
            "allOf" | "anyOf" | "oneOf" | "required" | "enum" => {
                if let Some(children) = value.as_array() {
                    for (idx, child) in children.iter().enumerate() {
                        validate_nested_schema_value(
                            schema_rel,
                            child,
                            &format!("{path}/{key}[{idx}]"),
                        )?;
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn validate_nested_schema_value(schema_rel: &str, value: &Value, path: &str) -> Result<()> {
    if value.is_object() {
        validate_supported_schema_keywords_at(schema_rel, value, path)?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::validate_supported_schema_keywords;
    use serde_json::json;

    #[test]
    fn schema_keyword_coverage_rejects_unsupported_keywords() {
        let schema = json!({
            "type": "object",
            "unsupportedKeyword": true
        });
        let err = validate_supported_schema_keywords("schemas/example.schema.json", &schema)
            .expect_err("unsupported keywords must be rejected");
        assert!(err.to_string().contains("unsupported schema keyword"));
    }

    #[test]
    fn schema_keyword_coverage_allows_supported_keywords() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 4,
                    "pattern": "^[a-z]+$"
                },
                "count": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 3
                }
            },
            "required": ["name", "count"],
            "additionalProperties": false
        });
        validate_supported_schema_keywords("schemas/example.schema.json", &schema)
            .expect("supported keywords should pass");
    }
}

fn is_integer(value: &Value) -> bool {
    let Some(number) = value.as_f64() else {
        return false;
    };
    number.fract() == 0.0
}

fn schema_root(_repo: &Path) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
}
