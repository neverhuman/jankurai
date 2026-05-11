use anyhow::{Context, Result};
use cargo_metadata::{MetadataCommand, Package};
use chrono::Utc;
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

use crate::render::{write_json, write_markdown};

#[derive(Debug, Clone)]
pub struct RustMapArgs {
    pub repo: PathBuf,
    pub out_dir: String,
}

#[derive(Debug, Clone)]
pub struct RustWitnessBuildArgs {
    pub repo: PathBuf,
    pub out: String,
}

#[derive(Debug, Clone)]
pub struct RustWitnessDiffArgs {
    pub repo: PathBuf,
    pub old: PathBuf,
    pub new: PathBuf,
    pub out: String,
    pub md: String,
}

#[derive(Debug, Clone)]
pub struct RustDiagnoseArgs {
    pub repo: PathBuf,
    pub out: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CiProfile {
    pub name: String,
    #[serde(default)]
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceAgentMetadata {
    #[serde(default)]
    pub validation_order: Vec<String>,
    #[serde(default)]
    pub shared_contracts: Vec<String>,
    #[serde(default)]
    pub ci_profiles: Vec<CiProfile>,
    #[serde(default)]
    pub instruction_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageAgentMetadata {
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub owned_paths: Vec<String>,
    #[serde(default)]
    pub entrypoints: Vec<String>,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(default)]
    pub local_validate: Vec<String>,
    #[serde(default)]
    pub boundary_validate: Vec<String>,
    #[serde(default)]
    pub public_api: bool,
    #[serde(default)]
    pub risk: String,
    #[serde(default)]
    pub consumers: Vec<String>,
    #[serde(default)]
    pub exceptions: Vec<String>,
}

#[derive(Debug, Clone)]
struct PackageSnapshot {
    name: String,
    manifest_path: PathBuf,
    package_root: PathBuf,
    agent: PackageAgentMetadata,
    direct_dependencies: Vec<String>,
    reverse_dependencies: Vec<String>,
    target_names: Vec<String>,
    target_tests: Vec<String>,
    features: Vec<String>,
    has_lib_target: bool,
}

#[derive(Debug, Clone)]
struct WorkspaceSnapshot {
    workspace_root: PathBuf,
    workspace_agent: WorkspaceAgentMetadata,
    packages: Vec<PackageSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMap {
    pub generated_at: String,
    pub workspace_root: String,
    pub validation_order: Vec<String>,
    pub shared_contracts: Vec<String>,
    pub ci_profiles: Vec<CiProfile>,
    pub instruction_roots: Vec<String>,
    pub members: Vec<AgentMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMember {
    pub name: String,
    pub manifest_path: String,
    pub package_root: String,
    pub direct_dependencies: Vec<String>,
    pub reverse_dependencies: Vec<String>,
    pub public_surfaces: Vec<String>,
    pub risk_tags: Vec<String>,
    pub instruction_locations: Vec<String>,
    pub validation_commands: ValidationCommands,
    pub api_surface_hash: String,
    pub proof_density: f64,
    pub context_roots: Vec<String>,
    pub exception_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCommands {
    pub local: Vec<String>,
    pub boundary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMap {
    pub generated_at: String,
    pub workspace_root: String,
    pub entries: Vec<TestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEntry {
    pub arc: String,
    pub source_roots: Vec<String>,
    pub unit_tests: Vec<String>,
    pub doctests: Vec<String>,
    pub integration_harnesses: Vec<String>,
    pub reverse_dependency_tests: Vec<String>,
    pub smoke_tests: Vec<String>,
    pub e2e_gates: Vec<String>,
    pub selection_reason: String,
    pub estimated_cost: String,
    pub required_for_change_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessGraph {
    pub generated_at: String,
    pub workspace_root: String,
    pub crates: Vec<CrateWitness>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateWitness {
    pub name: String,
    pub interface_hash: String,
    pub implementation_hash: String,
    #[serde(default)]
    pub pub_items: Vec<PubItem>,
    #[serde(default)]
    pub direct_deps: Vec<String>,
    #[serde(default)]
    pub reverse_deps: Vec<String>,
    pub file_count: usize,
    pub total_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubItem {
    pub kind: String,
    pub name: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessDiffReport {
    pub generated_at: String,
    pub workspace_root: String,
    pub old_graph_path: String,
    pub new_graph_path: String,
    pub changes: Vec<CrateChange>,
    pub total_crates_changed: usize,
    pub interface_changed_crates: usize,
    pub implementation_only_crates: usize,
    pub added_crates: usize,
    pub removed_crates: usize,
    pub escalation_required: bool,
    pub estimated_test_commands: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateChange {
    pub name: String,
    pub classification: ChangeClassification,
    pub interface_changed: bool,
    pub implementation_changed: bool,
    pub local_commands: Vec<String>,
    pub escalation_commands: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeClassification {
    InterfaceChanged,
    ImplementationOnly,
    Unchanged,
    Added,
    Removed,
}

impl std::fmt::Display for ChangeClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InterfaceChanged => write!(f, "interface-changed"),
            Self::ImplementationOnly => write!(f, "implementation-only"),
            Self::Unchanged => write!(f, "unchanged"),
            Self::Added => write!(f, "added"),
            Self::Removed => write!(f, "removed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilePackets {
    pub generated_at: String,
    pub repo_root: String,
    pub command: String,
    pub packets: Vec<CompilePacket>,
    pub summary: CompileSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilePacket {
    pub level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub owning_crate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crate_purpose: Option<String>,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(default)]
    pub local_commands: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler_suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileSummary {
    pub total_errors: usize,
    pub total_warnings: usize,
    pub crates_affected: usize,
}

pub fn run_map(args: RustMapArgs) -> Result<()> {
    let snapshot = load_workspace(&args.repo)?;
    let agent_map = build_agent_map(&snapshot);
    let test_map = build_test_map(&snapshot);
    let out_dir = resolve_output_dir(&snapshot.workspace_root, &args.out_dir);
    write_json(
        &out_dir.join("agent-map.json").display().to_string(),
        &serde_json::to_string_pretty(&agent_map)?,
    )?;
    write_json(
        &out_dir.join("test-map.json").display().to_string(),
        &serde_json::to_string_pretty(&test_map)?,
    )?;
    println!(
        "rust maps written: {} packages, {} test entries",
        agent_map.members.len(),
        test_map.entries.len()
    );
    Ok(())
}

pub fn run_witness_build(args: RustWitnessBuildArgs) -> Result<()> {
    let snapshot = load_workspace(&args.repo)?;
    let graph = build_witness_graph(&snapshot)?;
    let out_path = resolve_output_path(&snapshot.workspace_root, &args.out);
    write_json(
        &out_path.display().to_string(),
        &serde_json::to_string_pretty(&graph)?,
    )?;
    let crate_count = graph.crates.len();
    let pub_items: usize = graph.crates.iter().map(|item| item.pub_items.len()).sum();
    println!("witness graph written: {crate_count} crates, {pub_items} pub items indexed");
    Ok(())
}

pub fn run_witness_diff(args: RustWitnessDiffArgs) -> Result<()> {
    let snapshot = load_workspace(&args.repo)?;
    let old_path = resolve_input_path(&snapshot.workspace_root, &args.old);
    let new_path = resolve_input_path(&snapshot.workspace_root, &args.new);
    let old_graph = read_witness_graph(&old_path)?;
    let new_graph = read_witness_graph(&new_path)?;
    let report = diff_witness_graphs(
        &snapshot.workspace_root,
        &old_path,
        &new_path,
        &old_graph,
        &new_graph,
    );
    let out_path = resolve_output_path(&snapshot.workspace_root, &args.out);
    write_json(
        &out_path.display().to_string(),
        &serde_json::to_string_pretty(&report)?,
    )?;
    let md_path = resolve_output_path(&snapshot.workspace_root, &args.md);
    write_markdown(
        &md_path.display().to_string(),
        &render_witness_diff_md(&report),
    )?;
    println!(
        "witness diff written: {} changed crates, escalation required: {}",
        report.total_crates_changed, report.escalation_required
    );
    Ok(())
}

pub fn run_diagnose(args: RustDiagnoseArgs) -> Result<()> {
    let snapshot = load_workspace(&args.repo)?;
    let packets = diagnose_workspace(&snapshot)?;
    let out_path = resolve_output_path(&snapshot.workspace_root, &args.out);
    write_json(
        &out_path.display().to_string(),
        &serde_json::to_string_pretty(&packets)?,
    )?;
    if packets.summary.total_errors > 0 {
        println!(
            "compile-packets.json written: {} errors, {} warnings across {} crates",
            packets.summary.total_errors,
            packets.summary.total_warnings,
            packets.summary.crates_affected
        );
    } else {
        println!("workspace compiles cleanly — no diagnostics to route");
    }
    Ok(())
}

fn load_workspace(repo: &Path) -> Result<WorkspaceSnapshot> {
    let mut command = MetadataCommand::new();
    command.current_dir(repo);
    let metadata = command
        .exec()
        .with_context(|| format!("expected a Cargo workspace at {}", repo.display()))?;
    let workspace_root = metadata.workspace_root.as_std_path().to_path_buf();
    let workspace_agent = parse_workspace_agent(&metadata.workspace_metadata)?;
    let member_ids: BTreeSet<_> = metadata.workspace_members.iter().cloned().collect();
    let package_by_id: HashMap<_, _> = metadata
        .packages
        .iter()
        .map(|package| (package.id.clone(), package))
        .collect();

    let mut direct: HashMap<String, Vec<String>> = HashMap::new();
    let mut reverse: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(resolve) = &metadata.resolve {
        for node in &resolve.nodes {
            if !member_ids.contains(&node.id) {
                continue;
            }
            let Some(package) = package_by_id.get(&node.id) else {
                continue;
            };
            for dep in &node.deps {
                if !member_ids.contains(&dep.pkg) {
                    continue;
                }
                let Some(dep_package) = package_by_id.get(&dep.pkg) else {
                    continue;
                };
                direct
                    .entry(package.name.to_string())
                    .or_default()
                    .push(dep_package.name.to_string());
                reverse
                    .entry(dep_package.name.to_string())
                    .or_default()
                    .push(package.name.to_string());
            }
        }
    }

    let packages = metadata
        .packages
        .iter()
        .filter(|package| member_ids.contains(&package.id))
        .map(|package| package_snapshot(package, &direct, &reverse))
        .collect::<Result<Vec<_>>>()?;

    Ok(WorkspaceSnapshot {
        workspace_root,
        workspace_agent,
        packages,
    })
}

fn parse_workspace_agent(value: &serde_json::Value) -> Result<WorkspaceAgentMetadata> {
    let agent = value
        .get("agent")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    if agent.is_null() {
        return Ok(WorkspaceAgentMetadata::default());
    }
    serde_json::from_value(agent).context("failed to parse workspace.metadata.agent")
}

fn parse_package_agent(package: &Package) -> Result<PackageAgentMetadata> {
    let agent = package
        .metadata
        .get("agent")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    if agent.is_null() {
        return Ok(PackageAgentMetadata::default());
    }
    serde_json::from_value(agent).with_context(|| {
        format!(
            "failed to parse package.metadata.agent for {}",
            package.name
        )
    })
}

fn package_snapshot(
    package: &Package,
    direct: &HashMap<String, Vec<String>>,
    reverse: &HashMap<String, Vec<String>>,
) -> Result<PackageSnapshot> {
    let manifest_path = package.manifest_path.as_std_path().to_path_buf();
    let package_root = manifest_path
        .parent()
        .context("package manifest unexpectedly missing parent directory")?
        .to_path_buf();
    let agent = parse_package_agent(package)?;
    let mut target_names = package
        .targets
        .iter()
        .map(|target| target.name.clone())
        .collect::<Vec<_>>();
    target_names.sort();
    let mut target_tests = package
        .targets
        .iter()
        .filter(|target| {
            target
                .kind
                .iter()
                .any(|kind| matches!(kind, cargo_metadata::TargetKind::Test))
        })
        .map(|target| target.src_path.as_std_path().display().to_string())
        .collect::<Vec<_>>();
    target_tests.sort();
    let mut features = package.features.keys().cloned().collect::<Vec<_>>();
    features.sort();
    Ok(PackageSnapshot {
        name: package.name.to_string(),
        manifest_path,
        package_root,
        agent,
        direct_dependencies: sorted_lookup(direct, &package.name),
        reverse_dependencies: sorted_lookup(reverse, &package.name),
        target_names,
        target_tests,
        features,
        has_lib_target: package.targets.iter().any(|target| {
            target
                .kind
                .iter()
                .any(|kind| matches!(kind, cargo_metadata::TargetKind::Lib))
        }),
    })
}

fn sorted_lookup(map: &HashMap<String, Vec<String>>, key: &str) -> Vec<String> {
    let mut values = map.get(key).cloned().unwrap_or_default();
    values.sort();
    values.dedup();
    values
}

fn build_agent_map(snapshot: &WorkspaceSnapshot) -> AgentMap {
    let members = snapshot
        .packages
        .iter()
        .map(|package| AgentMember {
            name: package.name.clone(),
            manifest_path: display_relative(&snapshot.workspace_root, &package.manifest_path),
            package_root: display_relative(&snapshot.workspace_root, &package.package_root),
            direct_dependencies: package.direct_dependencies.clone(),
            reverse_dependencies: package.reverse_dependencies.clone(),
            public_surfaces: public_surfaces(package),
            risk_tags: risk_tags(package),
            instruction_locations: instruction_locations(&snapshot.workspace_root, package),
            validation_commands: ValidationCommands {
                local: local_validation_commands(package),
                boundary: boundary_validation_commands(package),
            },
            api_surface_hash: api_surface_hash(package),
            proof_density: proof_density(package),
            context_roots: context_roots(&snapshot.workspace_root, package),
            exception_refs: package.agent.exceptions.clone(),
        })
        .collect();

    AgentMap {
        generated_at: generated_at(),
        workspace_root: snapshot.workspace_root.display().to_string(),
        validation_order: snapshot.workspace_agent.validation_order.clone(),
        shared_contracts: snapshot.workspace_agent.shared_contracts.clone(),
        ci_profiles: snapshot.workspace_agent.ci_profiles.clone(),
        instruction_roots: snapshot.workspace_agent.instruction_roots.clone(),
        members,
    }
}

fn build_test_map(snapshot: &WorkspaceSnapshot) -> TestMap {
    let smoke_tests = collect_profile_commands(snapshot, "pull-request", "smoke")
        .into_iter()
        .chain(default_smoke_commands(snapshot))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let e2e_gates = collect_profile_commands(snapshot, "scheduled-hardening", "e2e")
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let entries = snapshot
        .packages
        .iter()
        .map(|package| TestEntry {
            arc: package.name.clone(),
            source_roots: owned_path_display(package),
            unit_tests: package
                .agent
                .local_validate
                .iter()
                .filter(|command| !command.contains("--doc"))
                .cloned()
                .chain(default_unit_commands(package))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            doctests: package
                .agent
                .local_validate
                .iter()
                .filter(|command| command.contains("--doc"))
                .cloned()
                .chain(default_doctest_commands(package))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            integration_harnesses: package.target_tests.clone(),
            reverse_dependency_tests: package
                .agent
                .boundary_validate
                .iter()
                .cloned()
                .chain(reverse_dependency_commands(package))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            smoke_tests: smoke_tests.clone(),
            e2e_gates: e2e_gates.clone(),
            selection_reason: if effective_public_api(package) {
                "public surface changes require reverse dependency and doctest awareness"
                    .to_string()
            } else {
                "leaf crate changes usually stop at local compile and test coverage".to_string()
            },
            estimated_cost: estimated_cost(package),
            required_for_change_types: required_for_change_types(package),
        })
        .collect();

    TestMap {
        generated_at: generated_at(),
        workspace_root: snapshot.workspace_root.display().to_string(),
        entries,
    }
}

fn default_smoke_commands(snapshot: &WorkspaceSnapshot) -> Vec<String> {
    if snapshot.packages.is_empty() {
        Vec::new()
    } else {
        vec!["cargo test --workspace".to_string()]
    }
}

fn default_unit_commands(package: &PackageSnapshot) -> Vec<String> {
    if package.agent.local_validate.is_empty() {
        vec![format!("cargo test -p {}", package.name)]
    } else {
        Vec::new()
    }
}

fn default_doctest_commands(package: &PackageSnapshot) -> Vec<String> {
    if effective_public_api(package)
        && !package
            .agent
            .local_validate
            .iter()
            .any(|cmd| cmd.contains("--doc"))
    {
        vec![format!("cargo test -p {} --doc", package.name)]
    } else {
        Vec::new()
    }
}

fn reverse_dependency_commands(package: &PackageSnapshot) -> Vec<String> {
    if !package.agent.boundary_validate.is_empty() {
        return Vec::new();
    }
    package
        .reverse_dependencies
        .iter()
        .map(|dep| format!("cargo test -p {dep}"))
        .collect()
}

fn local_validation_commands(package: &PackageSnapshot) -> Vec<String> {
    if package.agent.local_validate.is_empty() {
        vec![format!("cargo test -p {}", package.name)]
    } else {
        package.agent.local_validate.clone()
    }
}

fn boundary_validation_commands(package: &PackageSnapshot) -> Vec<String> {
    if !package.agent.boundary_validate.is_empty() {
        return package.agent.boundary_validate.clone();
    }
    if effective_public_api(package) {
        let mut commands = package
            .reverse_dependencies
            .iter()
            .map(|dep| format!("cargo test -p {dep}"))
            .collect::<Vec<_>>();
        if commands.is_empty() {
            commands.push(format!("cargo test -p {} --doc", package.name));
        }
        commands
    } else {
        Vec::new()
    }
}

fn public_surfaces(package: &PackageSnapshot) -> Vec<String> {
    if !package.agent.entrypoints.is_empty() {
        return package.agent.entrypoints.clone();
    }
    package.target_names.clone()
}

fn risk_tags(package: &PackageSnapshot) -> Vec<String> {
    let mut tags = vec![if package.agent.risk.is_empty() {
        "risk:unspecified".to_string()
    } else {
        format!("risk:{}", package.agent.risk)
    }];
    if effective_public_api(package) {
        tags.push("public-api".to_string());
    }
    if !package.agent.exceptions.is_empty() {
        tags.push("has-exception".to_string());
    }
    tags
}

fn instruction_locations(workspace_root: &Path, package: &PackageSnapshot) -> Vec<String> {
    let mut locations = Vec::new();
    for path in [
        workspace_root.join("AGENTS.md"),
        workspace_root.join("CLAUDE.md"),
        workspace_root.join(".github/copilot-instructions.md"),
        package.package_root.join("AGENTS.md"),
    ] {
        if path.exists() {
            locations.push(display_relative(workspace_root, &path));
        }
    }
    locations
}

fn context_roots(workspace_root: &Path, package: &PackageSnapshot) -> Vec<String> {
    let mut roots = vec![display_relative(workspace_root, &package.package_root)];
    for suffix in ["src", "tests", "examples"] {
        let candidate = package.package_root.join(suffix);
        if candidate.exists() {
            roots.push(display_relative(workspace_root, &candidate));
        }
    }
    for location in instruction_locations(workspace_root, package) {
        roots.push(location);
    }
    roots.sort();
    roots.dedup();
    roots
}

fn owned_path_display(package: &PackageSnapshot) -> Vec<String> {
    if package.agent.owned_paths.is_empty() {
        return vec![package.package_root.display().to_string()];
    }
    package.agent.owned_paths.clone()
}

fn proof_density(package: &PackageSnapshot) -> f64 {
    let proof_points = package.agent.invariants.len()
        + package.agent.local_validate.len()
        + package.agent.boundary_validate.len()
        + package.agent.exceptions.len();
    let entrypoints = package.agent.entrypoints.len().max(1);
    let density = proof_points as f64 / entrypoints as f64;
    (density * 100.0).round() / 100.0
}

fn api_surface_hash(package: &PackageSnapshot) -> String {
    let mut hasher = Sha256::new();
    hasher.update(package.name.as_bytes());
    hasher.update(if effective_public_api(package) {
        &b"public"[..]
    } else {
        &b"private"[..]
    });
    for item in public_surfaces(package) {
        hasher.update(item.as_bytes());
    }
    for feature in &package.features {
        hasher.update(feature.as_bytes());
    }
    hex_digest(hasher)
}

fn collect_profile_commands(
    snapshot: &WorkspaceSnapshot,
    profile_name: &str,
    needle: &str,
) -> Vec<String> {
    snapshot
        .workspace_agent
        .ci_profiles
        .iter()
        .find(|profile| profile.name == profile_name)
        .map(|profile| {
            profile
                .commands
                .iter()
                .filter(|command| command.contains(needle))
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn estimated_cost(package: &PackageSnapshot) -> String {
    let local = package.agent.local_validate.len();
    let boundary = package.agent.boundary_validate.len();
    let harnesses = package.target_tests.len();
    match local + boundary + harnesses {
        0..=2 => "low".to_string(),
        3..=5 => "medium".to_string(),
        _ => "high".to_string(),
    }
}

fn required_for_change_types(package: &PackageSnapshot) -> Vec<String> {
    let mut change_types = vec!["leaf-bugfix".to_string(), "invariant-change".to_string()];
    if effective_public_api(package) {
        change_types.push("public-api-change".to_string());
    }
    if !package.features.is_empty() {
        change_types.push("feature-change".to_string());
    }
    change_types.push("manifest-change".to_string());
    change_types
}

fn effective_public_api(package: &PackageSnapshot) -> bool {
    package.agent.public_api || package.has_lib_target
}

fn build_witness_graph(snapshot: &WorkspaceSnapshot) -> Result<WitnessGraph> {
    let mut crates = Vec::new();
    for package in &snapshot.packages {
        crates.push(build_crate_witness(&snapshot.workspace_root, package)?);
    }
    Ok(WitnessGraph {
        generated_at: generated_at(),
        workspace_root: snapshot.workspace_root.display().to_string(),
        crates,
    })
}

fn build_crate_witness(workspace_root: &Path, package: &PackageSnapshot) -> Result<CrateWitness> {
    let src_dir = package.package_root.join("src");
    let mut pub_items = Vec::new();
    let mut interface_hasher = Sha256::new();
    let mut impl_hasher = Sha256::new();
    let mut file_count = 0usize;
    let mut total_lines = 0usize;

    if src_dir.exists() {
        for rs_file in rust_source_files(&src_dir) {
            let scan = scan_source_file(
                workspace_root,
                &rs_file,
                &mut interface_hasher,
                &mut impl_hasher,
            )?;
            file_count += scan.file_count;
            total_lines += scan.total_lines;
            pub_items.extend(scan.pub_items);
        }
    }

    pub_items.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(a.name.cmp(&b.name))
            .then(a.signature.cmp(&b.signature))
    });
    pub_items.dedup_by(|a, b| a.kind == b.kind && a.name == b.name && a.signature == b.signature);

    Ok(CrateWitness {
        name: package.name.clone(),
        interface_hash: hex_digest(interface_hasher),
        implementation_hash: hex_digest(impl_hasher),
        pub_items,
        direct_deps: package.direct_dependencies.clone(),
        reverse_deps: package.reverse_dependencies.clone(),
        file_count,
        total_lines,
    })
}

struct SourceFileScan {
    pub_items: Vec<PubItem>,
    file_count: usize,
    total_lines: usize,
}

fn rust_source_files(src_dir: &Path) -> Vec<PathBuf> {
    let mut rs_files: Vec<PathBuf> = WalkDir::new(src_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().is_file()
                && entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs")
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();
    rs_files.sort();
    rs_files
}

fn scan_source_file(
    workspace_root: &Path,
    rs_file: &Path,
    interface_hasher: &mut Sha256,
    impl_hasher: &mut Sha256,
) -> Result<SourceFileScan> {
    let content = fs::read_to_string(rs_file)
        .with_context(|| format!("failed to read {}", rs_file.display()))?;
    let relative = display_relative(workspace_root, rs_file);
    let pub_items = extract_pub_items(&relative, &content);
    update_interface_hash(interface_hasher, &pub_items);
    update_implementation_hash(impl_hasher, &relative, &content, &pub_items);
    Ok(SourceFileScan {
        pub_items,
        file_count: 1,
        total_lines: content.lines().count(),
    })
}

fn push_pub_item(items: &mut Vec<PubItem>, kind: &str, name: String, signature: String) {
    items.push(PubItem {
        kind: kind.to_string(),
        name,
        signature,
    });
}

fn extract_pub_items(relative_path: &str, content: &str) -> Vec<PubItem> {
    let Ok(file) = syn::parse_file(content) else {
        let _ = relative_path;
        return Vec::new();
    };

    let mut items = Vec::new();
    for item in &file.items {
        extract_pub_item(item, &mut items);
    }

    let _ = relative_path;
    items
}

fn extract_pub_item(item: &syn::Item, items: &mut Vec<PubItem>) {
    match item {
        syn::Item::Fn(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
            push_pub_item(
                items,
                "fn",
                item.sig.ident.to_string(),
                format_fn_signature(&item.sig),
            );
        }
        syn::Item::Struct(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
            push_pub_item(
                items,
                "struct",
                item.ident.to_string(),
                struct_signature(item),
            );
        }
        syn::Item::Enum(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
            let name = item.ident.to_string();
            push_pub_item(
                items,
                "enum",
                name.clone(),
                format!("pub enum {name}({} variants)", item.variants.len()),
            );
        }
        syn::Item::Trait(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
            let name = item.ident.to_string();
            let method_count = item
                .items
                .iter()
                .filter(|trait_item| matches!(trait_item, syn::TraitItem::Fn(_)))
                .count();
            push_pub_item(
                items,
                "trait",
                name.clone(),
                format!("pub trait {name}({method_count} methods)"),
            );
        }
        syn::Item::Type(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
            let name = item.ident.to_string();
            push_pub_item(
                items,
                "type",
                name.clone(),
                format!("pub type {name} = ..."),
            );
        }
        syn::Item::Const(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
            let name = item.ident.to_string();
            push_pub_item(
                items,
                "const",
                name.clone(),
                format!("pub const {name}: ..."),
            );
        }
        syn::Item::Static(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
            let name = item.ident.to_string();
            push_pub_item(
                items,
                "static",
                name.clone(),
                format!("pub static {name}: ..."),
            );
        }
        syn::Item::Mod(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
            let name = item.ident.to_string();
            push_pub_item(items, "mod", name.clone(), format!("pub mod {name}"));
        }
        syn::Item::Impl(item) => extract_pub_impl_methods(item, items),
        _ => {}
    }
}

fn struct_signature(item: &syn::ItemStruct) -> String {
    let name = item.ident.to_string();
    let generics = item.generics.params.iter().count();
    let fields = match &item.fields {
        syn::Fields::Named(named) => named.named.len(),
        syn::Fields::Unnamed(unnamed) => unnamed.unnamed.len(),
        syn::Fields::Unit => 0,
    };
    format!(
        "pub struct {name}{}({fields} fields)",
        if generics > 0 {
            format!("<{generics} generics>")
        } else {
            String::new()
        }
    )
}

fn extract_pub_impl_methods(item: &syn::ItemImpl, items: &mut Vec<PubItem>) {
    let self_ty = quote_type(&item.self_ty);
    for impl_item in &item.items {
        if let syn::ImplItem::Fn(method) = impl_item {
            if matches!(method.vis, syn::Visibility::Public(_)) {
                push_pub_item(
                    items,
                    "fn",
                    method.sig.ident.to_string(),
                    format!("impl {self_ty} :: {}", format_fn_signature(&method.sig)),
                );
            }
        }
    }
}

fn generic_param_name(param: &syn::GenericParam) -> String {
    match param {
        syn::GenericParam::Type(t) => t.ident.to_string(),
        syn::GenericParam::Lifetime(l) => format!("'{}", l.lifetime.ident),
        syn::GenericParam::Const(c) => format!("const {}", c.ident),
    }
}

fn signature_generics(sig: &syn::Signature) -> String {
    if sig.generics.params.is_empty() {
        return String::new();
    }
    let params: Vec<String> = sig.generics.params.iter().map(generic_param_name).collect();
    format!("<{}>", params.join(", "))
}

fn receiver_signature(receiver: &syn::Receiver) -> String {
    if receiver.reference.is_some() {
        if receiver.mutability.is_some() {
            "&mut self".to_string()
        } else {
            "&self".to_string()
        }
    } else {
        "self".to_string()
    }
}

fn input_signature(arg: &syn::FnArg) -> String {
    match arg {
        syn::FnArg::Receiver(receiver) => receiver_signature(receiver),
        syn::FnArg::Typed(pat) => quote_type(&pat.ty),
    }
}

fn format_fn_signature(sig: &syn::Signature) -> String {
    let asyncness = if sig.asyncness.is_some() {
        "async "
    } else {
        ""
    };
    let unsafety = if sig.unsafety.is_some() {
        "unsafe "
    } else {
        ""
    };
    let name = &sig.ident;
    let generics = signature_generics(sig);
    let inputs: Vec<String> = sig.inputs.iter().map(input_signature).collect();

    let output = match &sig.output {
        syn::ReturnType::Default => String::new(),
        syn::ReturnType::Type(_, ty) => format!(" -> {}", quote_type(ty)),
    };

    format!(
        "{unsafety}{asyncness}fn {name}{generics}({}){output}",
        inputs.join(", ")
    )
}

fn quote_type(ty: &syn::Type) -> String {
    ty.to_token_stream().to_string()
}

fn update_interface_hash(interface_hasher: &mut Sha256, pub_items: &[PubItem]) {
    let pub_signatures: BTreeSet<String> = pub_items
        .iter()
        .map(|item| item.signature.clone())
        .collect();
    for sig in &pub_signatures {
        interface_hasher.update(sig.as_bytes());
        interface_hasher.update(b"\n");
    }
}

fn update_implementation_hash(
    impl_hasher: &mut Sha256,
    relative: &str,
    content: &str,
    pub_items: &[PubItem],
) {
    impl_hasher.update(relative.as_bytes());
    impl_hasher.update(content.as_bytes());
    for sig in pub_items.iter().map(|item| &item.signature) {
        impl_hasher.update(b"PUB:");
        impl_hasher.update(sig.as_bytes());
    }
}

fn read_witness_graph(path: &Path) -> Result<WitnessGraph> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))
}

fn diff_witness_graphs(
    workspace_root: &Path,
    old_path: &Path,
    new_path: &Path,
    old: &WitnessGraph,
    new: &WitnessGraph,
) -> WitnessDiffReport {
    let old_map = crate_lookup(old);
    let new_map = crate_lookup(new);
    let names = crate_names(&old_map, &new_map);
    let changes = names
        .into_iter()
        .filter_map(|name| classify_crate_change(name, &old_map, &new_map))
        .collect::<Vec<_>>();
    let total_crates_changed = changes.len();
    let escalation_required = changes
        .iter()
        .any(|change| change.classification == ChangeClassification::InterfaceChanged);
    let estimated_test_commands = changes
        .iter()
        .map(|change| change.local_commands.len() + change.escalation_commands.len())
        .sum();
    let interface_changed_crates = changes
        .iter()
        .filter(|change| change.classification == ChangeClassification::InterfaceChanged)
        .count();
    let implementation_only_crates = changes
        .iter()
        .filter(|change| change.classification == ChangeClassification::ImplementationOnly)
        .count();
    let added_crates = changes
        .iter()
        .filter(|change| change.classification == ChangeClassification::Added)
        .count();
    let removed_crates = changes
        .iter()
        .filter(|change| change.classification == ChangeClassification::Removed)
        .count();

    WitnessDiffReport {
        generated_at: generated_at(),
        workspace_root: workspace_root.display().to_string(),
        old_graph_path: display_relative(workspace_root, old_path),
        new_graph_path: display_relative(workspace_root, new_path),
        changes,
        total_crates_changed,
        interface_changed_crates,
        implementation_only_crates,
        added_crates,
        removed_crates,
        escalation_required,
        estimated_test_commands,
    }
}

fn crate_lookup(graph: &WitnessGraph) -> HashMap<&str, &CrateWitness> {
    graph
        .crates
        .iter()
        .map(|item| (item.name.as_str(), item))
        .collect()
}

fn crate_names<'a>(
    old_map: &'a HashMap<&'a str, &'a CrateWitness>,
    new_map: &'a HashMap<&'a str, &'a CrateWitness>,
) -> Vec<&'a str> {
    let mut names: Vec<&str> = old_map
        .keys()
        .chain(new_map.keys())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    names.sort();
    names
}

fn local_commands(name: &str) -> Vec<String> {
    vec![
        format!("cargo check -p {name}"),
        format!("cargo test -p {name}"),
    ]
}

fn escalation_commands(
    name: &str,
    interface_changed: bool,
    reverse_deps: &[String],
) -> Vec<String> {
    if interface_changed {
        if reverse_deps.is_empty() {
            vec![format!("cargo test -p {name} --doc")]
        } else {
            reverse_deps
                .iter()
                .map(|dep| format!("cargo test -p {dep}"))
                .collect()
        }
    } else {
        Vec::new()
    }
}

fn short_hash(hash: &str) -> &str {
    if hash.len() <= 12 {
        hash
    } else {
        &hash[..12]
    }
}

fn added_crate_change(name: &str) -> CrateChange {
    CrateChange {
        name: name.to_string(),
        classification: ChangeClassification::Added,
        interface_changed: true,
        implementation_changed: true,
        local_commands: local_commands(name),
        escalation_commands: vec![format!("cargo test -p {name}")],
        reason: "new crate added to workspace".to_string(),
    }
}

fn removed_crate_change(name: &str) -> CrateChange {
    CrateChange {
        name: name.to_string(),
        classification: ChangeClassification::Removed,
        interface_changed: true,
        implementation_changed: true,
        local_commands: vec![],
        escalation_commands: vec!["cargo check --workspace".to_string()],
        reason: "crate removed from workspace — validate all former consumers".to_string(),
    }
}

fn shared_crate_change(
    name: &str,
    old_crate: &CrateWitness,
    new_crate: &CrateWitness,
) -> Option<CrateChange> {
    let interface_changed = old_crate.interface_hash != new_crate.interface_hash;
    let implementation_changed = old_crate.implementation_hash != new_crate.implementation_hash;
    if !interface_changed && !implementation_changed {
        return None;
    }

    let classification = if interface_changed {
        ChangeClassification::InterfaceChanged
    } else {
        ChangeClassification::ImplementationOnly
    };
    let reason = if interface_changed {
        format!(
            "interface hash changed ({} → {}); public API shift requires reverse dependency validation",
            short_hash(&old_crate.interface_hash),
            short_hash(&new_crate.interface_hash)
        )
    } else {
        format!(
            "implementation hash changed ({} → {}); interface stable — local-only validation",
            short_hash(&old_crate.implementation_hash),
            short_hash(&new_crate.implementation_hash)
        )
    };

    Some(CrateChange {
        name: name.to_string(),
        classification,
        interface_changed,
        implementation_changed,
        local_commands: local_commands(name),
        escalation_commands: escalation_commands(name, interface_changed, &new_crate.reverse_deps),
        reason,
    })
}

fn classify_crate_change(
    name: &str,
    old_map: &HashMap<&str, &CrateWitness>,
    new_map: &HashMap<&str, &CrateWitness>,
) -> Option<CrateChange> {
    match (old_map.get(name), new_map.get(name)) {
        (None, Some(_)) => Some(added_crate_change(name)),
        (Some(_), None) => Some(removed_crate_change(name)),
        (Some(old_crate), Some(new_crate)) => shared_crate_change(name, old_crate, new_crate),
        (None, None) => None,
    }
}

fn render_witness_diff_md(report: &WitnessDiffReport) -> String {
    let mut out = String::new();
    use std::fmt::Write;
    let _ = writeln!(out, "# jankurai Rust Witness Diff");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Workspace: `{}`", report.workspace_root);
    let _ = writeln!(out, "- Old graph: `{}`", report.old_graph_path);
    let _ = writeln!(out, "- New graph: `{}`", report.new_graph_path);
    let _ = writeln!(out, "- Changed crates: `{}`", report.total_crates_changed);
    let _ = writeln!(
        out,
        "- Interface changes: `{}`",
        report.interface_changed_crates
    );
    let _ = writeln!(
        out,
        "- Implementation-only changes: `{}`",
        report.implementation_only_crates
    );
    let _ = writeln!(out, "- Added crates: `{}`", report.added_crates);
    let _ = writeln!(out, "- Removed crates: `{}`", report.removed_crates);
    let _ = writeln!(
        out,
        "- Escalation required: `{}`",
        report.escalation_required
    );
    let _ = writeln!(
        out,
        "- Estimated commands: `{}`",
        report.estimated_test_commands
    );
    if report.changes.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "No crate-level changes detected.");
        return out;
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "| Crate | Class | Local | Escalation | Reason |");
    let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
    for change in &report.changes {
        let local = if change.local_commands.is_empty() {
            "none".to_string()
        } else {
            change.local_commands.join(" | ")
        };
        let escalation = if change.escalation_commands.is_empty() {
            "none".to_string()
        } else {
            change.escalation_commands.join(" | ")
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            change.name, change.classification, local, escalation, change.reason
        );
    }
    out
}

fn diagnose_workspace(snapshot: &WorkspaceSnapshot) -> Result<CompilePackets> {
    let mut command = Command::new("cargo");
    command
        .arg("check")
        .arg("--workspace")
        .arg("--message-format=json")
        .current_dir(&snapshot.workspace_root);
    let output = command
        .output()
        .context("failed to run cargo check --message-format=json")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(route_diagnostics(
        &snapshot.workspace_root,
        &snapshot.packages,
        &stdout,
    ))
}

fn route_diagnostics(
    workspace_root: &Path,
    packages: &[PackageSnapshot],
    stdout: &str,
) -> CompilePackets {
    let mut packets = Vec::new();
    let mut crates_affected = BTreeSet::new();
    let mut total_errors = 0usize;
    let mut total_warnings = 0usize;

    for line in stdout.lines() {
        let Ok(msg) = serde_json::from_str::<CargoMessage>(line) else {
            continue;
        };
        if msg.reason != "compiler-message" {
            continue;
        }
        let Some(diagnostic) = msg.message else {
            continue;
        };
        let Some(packet) = compile_packet_from_diagnostic(workspace_root, packages, diagnostic)
        else {
            continue;
        };
        match packet.level.as_str() {
            "error" => total_errors += 1,
            "warning" => total_warnings += 1,
            _ => {}
        }
        crates_affected.insert(packet.owning_crate.clone());
        packets.push(packet);
    }

    CompilePackets {
        generated_at: generated_at(),
        repo_root: workspace_root.display().to_string(),
        command: "cargo check --workspace --message-format=json".to_string(),
        packets,
        summary: CompileSummary {
            total_errors,
            total_warnings,
            crates_affected: crates_affected.len(),
        },
    }
}

fn compile_packet_from_diagnostic(
    workspace_root: &Path,
    packages: &[PackageSnapshot],
    diagnostic: Diagnostic,
) -> Option<CompilePacket> {
    if !matches!(diagnostic.level.as_str(), "error" | "warning") {
        return None;
    }
    let location = diagnostic_location(&diagnostic);
    let owning_crate = resolve_owning_crate(workspace_root, packages, &location.file);
    let (crate_purpose, invariants, local_commands) = packet_context(packages, &owning_crate);
    let compiler_suggestion = compiler_suggestion(&diagnostic);
    let code = diagnostic.code.as_ref().map(|c| c.code.clone());
    let level = diagnostic.level;
    let message = diagnostic.message;
    Some(CompilePacket {
        level,
        code,
        message,
        file: location.file,
        line: location.line,
        column: location.column,
        owning_crate,
        crate_purpose,
        invariants,
        local_commands,
        compiler_suggestion,
    })
}

fn diagnostic_location(diagnostic: &Diagnostic) -> DiagnosticLocation {
    let primary_span = diagnostic
        .spans
        .iter()
        .find(|span| span.is_primary)
        .or_else(|| diagnostic.spans.first());
    if let Some(span) = primary_span {
        DiagnosticLocation {
            file: span.file_name.clone(),
            line: span.line_start,
            column: span.column_start,
        }
    } else {
        DiagnosticLocation {
            file: "<unknown>".to_string(),
            line: 0,
            column: 0,
        }
    }
}

fn resolve_owning_crate(workspace_root: &Path, packages: &[PackageSnapshot], file: &str) -> String {
    packages
        .iter()
        .find(|package| {
            let pkg_root = package
                .package_root
                .strip_prefix(workspace_root)
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            file.starts_with(&pkg_root)
        })
        .map(|package| package.name.clone())
        .unwrap_or_else(|| "<unmatched>".to_string())
}

fn packet_context(
    packages: &[PackageSnapshot],
    owning_crate: &str,
) -> (Option<String>, Vec<String>, Vec<String>) {
    let package = packages.iter().find(|p| p.name == owning_crate);
    let purpose = package
        .map(|p| p.agent.purpose.clone())
        .filter(|purpose| !purpose.is_empty());
    let invariants = package
        .map(|p| p.agent.invariants.clone())
        .unwrap_or_default();
    let local_commands = package
        .map(|p| p.agent.local_validate.clone())
        .unwrap_or_default();
    (purpose, invariants, local_commands)
}

fn compiler_suggestion(diagnostic: &Diagnostic) -> Option<String> {
    diagnostic
        .children
        .iter()
        .find(|child| child.level == "help")
        .map(|child| child.message.clone())
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn generated_at() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn resolve_output_dir(workspace_root: &Path, out_dir: &str) -> PathBuf {
    let path = Path::new(out_dir);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

fn resolve_output_path(workspace_root: &Path, out: &str) -> PathBuf {
    let path = Path::new(out);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    }
}

fn resolve_input_path(workspace_root: &Path, input: &Path) -> PathBuf {
    if input.is_absolute() {
        input.to_path_buf()
    } else {
        workspace_root.join(input)
    }
}

fn hex_digest(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug, Deserialize)]
struct CargoMessage {
    reason: String,
    #[serde(default)]
    message: Option<Diagnostic>,
}

#[derive(Debug, Deserialize)]
struct Diagnostic {
    level: String,
    message: String,
    #[serde(default)]
    code: Option<DiagnosticCode>,
    #[serde(default)]
    spans: Vec<DiagnosticSpan>,
    #[serde(default)]
    children: Vec<DiagnosticChild>,
}

#[derive(Debug, Deserialize)]
struct DiagnosticCode {
    code: String,
}

#[derive(Debug, Deserialize)]
struct DiagnosticSpan {
    file_name: String,
    line_start: u32,
    column_start: u32,
    #[serde(default)]
    is_primary: bool,
}

#[derive(Debug, Deserialize)]
struct DiagnosticChild {
    level: String,
    message: String,
}

#[derive(Debug)]
struct DiagnosticLocation {
    file: String,
    line: u32,
    column: u32,
}
