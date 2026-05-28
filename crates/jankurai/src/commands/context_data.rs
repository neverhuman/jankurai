use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::validation;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OwnerMapFile {
    #[serde(default)]
    pub owners: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TestMapFile {
    #[serde(default)]
    pub tests: BTreeMap<String, TestSpec>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TestSpec {
    pub command: String,
    #[serde(default)]
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GeneratedZonesFile {
    #[serde(default)]
    pub zone: Vec<GeneratedZone>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GeneratedZone {
    pub path: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub write_policy: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProofLanesFile {
    #[serde(default)]
    pub lane: Vec<ProofLane>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProofLane {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub command_id: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub cost: Option<u64>,
    #[serde(default)]
    pub supersedes: Vec<String>,
    #[serde(default)]
    pub rules_covered: Vec<String>,
    #[serde(default)]
    pub required_artifacts: Vec<String>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub requires_network: bool,
    #[serde(default)]
    pub destructive: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RepoCatalog {
    pub owners: BTreeMap<String, String>,
    pub tests: BTreeMap<String, TestSpec>,
    pub generated_zones: Vec<GeneratedZone>,
    pub proof_lanes: Vec<ProofLane>,
}

impl RepoCatalog {
    pub fn load(repo: &Path) -> Result<Self> {
        Ok(Self {
            owners: read_json_strict::<OwnerMapFile>(&repo.join("agent/owner-map.json"))?
                .map(|file| file.owners)
                .unwrap_or_default(),
            tests: read_json_strict::<TestMapFile>(&repo.join("agent/test-map.json"))?
                .map(|file| file.tests)
                .unwrap_or_default(),
            generated_zones: read_toml::<GeneratedZonesFile>(
                &repo.join("agent/generated-zones.toml"),
            )?
            .map(|file| file.zone)
            .unwrap_or_default(),
            proof_lanes: read_toml::<ProofLanesFile>(&repo.join("agent/proof-lanes.toml"))?
                .map(|file| file.lane)
                .unwrap_or_default(),
        })
    }

    pub fn owner_for_path(&self, path: &str) -> Option<&str> {
        self.owner_prefix_for_path(path)
            .and_then(|prefix| self.owners.get(&prefix))
            .map(|owner| owner.as_str())
    }

    pub fn owner_prefix_for_path(&self, path: &str) -> Option<String> {
        self.owners
            .keys()
            .filter_map(|prefix| route_match(path, prefix).map(|route| (prefix.clone(), route)))
            .max_by(|left, right| left.1.specificity.cmp(&right.1.specificity))
            .map(|(prefix, _)| prefix)
    }

    pub fn owner_route_for_path(&self, path: &str) -> Option<RouteMatch> {
        self.owners
            .keys()
            .filter_map(|prefix| route_match(path, prefix))
            .max_by(|left, right| left.specificity.cmp(&right.specificity))
    }

    pub fn test_route_for_path(&self, path: &str) -> Option<(RouteMatch, TestSpec)> {
        self.tests
            .iter()
            .filter_map(|(prefix, spec)| route_match(path, prefix).map(|route| (route, spec)))
            .max_by(|left, right| left.0.specificity.cmp(&right.0.specificity))
            .map(|(route, spec)| (route, spec.clone()))
    }

    pub fn prefixes_for_owner(&self, owner: &str) -> Vec<String> {
        self.owners
            .iter()
            .filter(|(_, value)| value.as_str() == owner)
            .map(|(path, _)| path.clone())
            .collect()
    }

    pub fn commands_for_paths(&self, paths: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        for path in paths {
            for (key, spec) in &self.tests {
                if path_matches(path, key) {
                    push_unique(&mut out, spec.command.clone());
                }
            }
        }
        out
    }

    pub fn proof_lane_names(&self) -> Vec<String> {
        self.proof_lanes
            .iter()
            .map(|lane| lane.name.clone())
            .collect()
    }

    pub fn proof_lane_for_command(&self, command: &str) -> Option<String> {
        self.proof_lanes
            .iter()
            .find(|lane| lane.command == command || lane.name == command)
            .map(|lane| lane.name.clone())
    }

    pub fn proof_lane_commands(&self, lane_names: &[&str]) -> Vec<String> {
        let mut out = Vec::new();
        for lane_name in lane_names {
            for lane in &self.proof_lanes {
                if lane.name == *lane_name {
                    push_unique(&mut out, lane.command.clone());
                }
            }
        }
        out
    }

    pub fn generated_paths(&self) -> Vec<String> {
        self.generated_zones
            .iter()
            .map(|zone| zone.path.clone())
            .collect()
    }

    pub fn forbidden_generated_paths(&self) -> Vec<String> {
        self.generated_zones
            .iter()
            .map(|zone| zone.path.clone())
            .collect()
    }

    /// Normalize a proof command for allowlist comparison: trim, collapse ASCII whitespace to single spaces.
    pub fn normalize_proof_command(command: &str) -> String {
        command.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Commands that `jankurai prove` may execute: union of `agent/proof-lanes.toml` and `agent/test-map.json`.
    pub fn allowed_proof_commands(&self) -> BTreeSet<String> {
        let mut allow = BTreeSet::new();
        for lane in &self.proof_lanes {
            allow.insert(Self::normalize_proof_command(&lane.command));
        }
        for spec in self.tests.values() {
            allow.insert(Self::normalize_proof_command(&spec.command));
        }
        allow
    }
}

fn read_json_strict<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let value = validation::parse_json_value_strict(&text)
        .map_err(|err| anyhow::anyhow!("parse {}: {err}", path.display()))?;
    Ok(Some(
        serde_json::from_value(value).with_context(|| format!("decode {}", path.display()))?,
    ))
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    Ok(Some(
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?,
    ))
}

fn path_matches(path: &str, prefix: &str) -> bool {
    route_match(path, prefix).is_some()
}

#[derive(Debug, Clone)]
pub struct RouteMatch {
    pub prefix: String,
    pub match_kind: String,
    pub specificity: usize,
}

fn route_match(path: &str, prefix: &str) -> Option<RouteMatch> {
    let path = normalize_path(path);
    let prefix = normalize_path(prefix);
    if path.is_empty() || prefix.is_empty() {
        return None;
    }
    if path == prefix {
        return Some(RouteMatch {
            specificity: prefix.split('/').count(),
            prefix,
            match_kind: "exact".into(),
        });
    }
    let directory_prefix = format!("{prefix}/");
    if path.starts_with(&directory_prefix) {
        return Some(RouteMatch {
            specificity: prefix.split('/').count(),
            prefix,
            match_kind: "directory".into(),
        });
    }
    None
}

fn normalize_path(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("./")
        .trim_matches('/')
        .to_string()
}

pub fn push_unique(values: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if !values.contains(&value) {
        values.push(value);
    }
}
