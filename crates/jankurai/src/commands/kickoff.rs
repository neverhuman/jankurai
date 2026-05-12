use crate::commands::context_data::{push_unique, GeneratedZone, RepoCatalog};
use crate::commands::repair::now_string;
use crate::validation::{self, ArtifactSchema};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct KickoffArgs {
    pub repo: PathBuf,
    pub intent: String,
    pub changed: Vec<PathBuf>,
    pub changed_from: Option<String>,
    pub agent: String,
    pub max_tokens: usize,
    pub out: String,
    pub md: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct KickoffReport {
    pub schema_version: String,
    pub command: String,
    pub generated_at: String,
    pub repo: String,
    pub intent: String,
    pub agent: String,
    pub changed_paths: Vec<String>,
    pub read_first: Vec<String>,
    pub route_decisions: Vec<KickoffRouteDecision>,
    pub generated_zone_touches: Vec<GeneratedZoneTouch>,
    pub ownership_boundaries: Vec<OwnershipBoundary>,
    pub forbidden_paths: Vec<String>,
    pub proof_lanes: Vec<String>,
    pub clarifying_questions: Vec<ClarifyingQuestion>,
    pub implementation_steps: Vec<ImplementationStep>,
    pub parallel_packets: Vec<ParallelPacket>,
    pub stop_conditions: Vec<String>,
    pub expected_receipts: Vec<String>,
    pub next_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KickoffRouteDecision {
    pub path: String,
    pub owner: String,
    pub owner_route: String,
    pub test_command: String,
    pub proof_lane: String,
    pub generated_zone: bool,
    pub generated_source: String,
    pub generated_command: String,
    pub decision: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratedZoneTouch {
    pub path: String,
    pub zone: String,
    pub source: String,
    pub command: String,
    pub read_only: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OwnershipBoundary {
    pub owner: String,
    pub paths: Vec<String>,
    pub owner_routes: Vec<String>,
    pub test_commands: Vec<String>,
    pub proof_lanes: Vec<String>,
    pub write_policy: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParallelPacket {
    pub name: String,
    pub paths: Vec<String>,
    pub owner: String,
    pub proof_lane: String,
    pub commands: Vec<String>,
    pub expected_receipts: Vec<String>,
    pub stop_conditions: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClarifyingQuestion {
    pub id: String,
    pub question: String,
    pub reason: String,
    pub suggested_paths: Vec<String>,
    pub blocking: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImplementationStep {
    pub order: usize,
    pub title: String,
    pub files: Vec<String>,
    pub commands: Vec<String>,
    pub notes: String,
}

pub fn run(args: KickoffArgs) -> Result<()> {
    if !args.changed.is_empty() && args.changed_from.is_some() {
        bail!("use either --changed or --changed-from, not both");
    }

    let report = build_kickoff(
        &args.repo,
        &args.intent,
        &args.changed,
        args.changed_from.as_deref(),
        &args.agent,
        args.max_tokens,
    )?;
    validation::write_json(&args.repo, ArtifactSchema::Kickoff, &args.out, &report)?;
    crate::render::write_markdown(&args.md, &render_markdown(&report))?;
    Ok(())
}

pub fn build_kickoff(
    repo: &Path,
    intent: &str,
    changed: &[PathBuf],
    changed_from: Option<&str>,
    agent: &str,
    max_tokens: usize,
) -> Result<KickoffReport> {
    let catalog = RepoCatalog::load(repo)?;
    let changed_paths = normalize_changed_paths(repo, changed, changed_from)?;
    let route_decisions = build_route_decisions(&catalog, &changed_paths);
    let generated_zone_touches =
        build_generated_zone_touches(&catalog.generated_zones, &changed_paths);
    let ownership_boundaries =
        build_ownership_boundaries(&route_decisions, &generated_zone_touches);
    let forbidden_paths = build_forbidden_paths(&catalog);
    let proof_lanes = collect_proof_lanes(&route_decisions);
    let clarifying_questions = build_clarifying_questions(
        intent,
        &changed_paths,
        &route_decisions,
        &generated_zone_touches,
        &ownership_boundaries,
    );
    let step_limit = (max_tokens.max(1) / 1200).clamp(3, 8);
    let command_limit = (max_tokens.max(1) / 2000).clamp(2, 4);
    let expected_receipts = build_expected_receipts();
    let next_commands = build_next_commands(
        repo,
        intent,
        changed_from,
        &changed_paths,
        &route_decisions,
        &generated_zone_touches,
    );
    let mut implementation_steps = build_implementation_steps(
        &changed_paths,
        &route_decisions,
        &generated_zone_touches,
        &ownership_boundaries,
        &clarifying_questions,
        &next_commands,
    );
    implementation_steps.truncate(step_limit);
    let mut parallel_packets = build_parallel_packets(&route_decisions);
    if parallel_packets.len() > command_limit {
        parallel_packets.truncate(command_limit);
    }
    let stop_conditions = build_stop_conditions(
        &changed_paths,
        &route_decisions,
        &generated_zone_touches,
        &ownership_boundaries,
        &clarifying_questions,
    );
    Ok(KickoffReport {
        schema_version: "1.0.0".to_string(),
        command: "jankurai kickoff".to_string(),
        generated_at: now_string(),
        repo: repo.display().to_string(),
        intent: intent.to_string(),
        agent: agent.to_string(),
        changed_paths,
        read_first: read_first_files(),
        route_decisions,
        generated_zone_touches,
        ownership_boundaries,
        forbidden_paths,
        proof_lanes,
        clarifying_questions,
        implementation_steps,
        parallel_packets,
        stop_conditions,
        expected_receipts,
        next_commands,
    })
}

fn read_first_files() -> Vec<String> {
    vec![
        "AGENTS.md",
        "agent/JANKURAI_STANDARD.md",
        "docs/agent-native-standard.md",
        "docs/mission.md",
        "docs/moonshot.md",
        "README.md",
        "docs/artifact-contracts.md",
        "docs/testing.md",
        "agent/owner-map.json",
        "agent/test-map.json",
        "agent/generated-zones.toml",
        "agent/proof-lanes.toml",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn build_route_decisions(
    catalog: &RepoCatalog,
    changed_paths: &[String],
) -> Vec<KickoffRouteDecision> {
    changed_paths
        .iter()
        .map(|path| {
            let owner_route = catalog
                .owner_route_for_path(path)
                .map(|route| route.prefix)
                .unwrap_or_else(|| "unmapped".to_string());
            let owner = catalog
                .owner_for_path(path)
                .map(|owner| owner.to_string())
                .unwrap_or_else(|| "unmapped".to_string());
            let test_route = catalog.test_route_for_path(path);
            let test_command = test_route
                .as_ref()
                .map(|(_, spec)| spec.command.clone())
                .unwrap_or_else(|| "unmapped".to_string());
            let proof_lane = test_route
                .as_ref()
                .and_then(|(_, spec)| catalog.proof_lane_for_command(spec.command.trim()))
                .unwrap_or_else(|| "unmapped".to_string());
            let generated = generated_zone_match(&catalog.generated_zones, path);
            let (generated_zone, generated_source, generated_command, decision, reason) =
                if let Some(zone) = generated {
                    let source = if zone.source.is_empty() {
                        "unknown".to_string()
                    } else {
                        zone.source.clone()
                    };
                    let command = zone.command.clone();
                    let reason = if zone.source.is_empty() {
                        format!(
                            "generated path `{path}` needs a source contract before any edit"
                        )
                    } else {
                        format!(
                            "generated path `{path}` should be edited through source contract `{source}` and regenerated with `{command}`"
                        )
                    };
                    (true, source, command, "read-only".to_string(), reason)
                } else if owner != "unmapped" && proof_lane != "unmapped" {
                    (
                        false,
                        String::new(),
                        String::new(),
                        "allowed".to_string(),
                        format!(
                            "owner route `{owner_route}` and proof lane `{proof_lane}` cover `{path}`"
                        ),
                    )
                } else if owner == "unmapped" && proof_lane == "unmapped" {
                    (
                        false,
                        String::new(),
                        String::new(),
                        "human-review".to_string(),
                        format!("no owner-map or test-map route covers `{path}`"),
                    )
                } else if owner == "unmapped" {
                    (
                        false,
                        String::new(),
                        String::new(),
                        "human-review".to_string(),
                        format!("no owner-map route covers `{path}`"),
                    )
                } else {
                    (
                        false,
                        String::new(),
                        String::new(),
                        "human-review".to_string(),
                        format!("test route `{test_command}` is not backed by a proof lane for `{path}`"),
                    )
                };
            KickoffRouteDecision {
                path: path.clone(),
                owner,
                owner_route,
                test_command,
                proof_lane,
                generated_zone,
                generated_source,
                generated_command,
                decision,
                reason,
            }
        })
        .collect()
}

fn build_generated_zone_touches(
    zones: &[GeneratedZone],
    changed_paths: &[String],
) -> Vec<GeneratedZoneTouch> {
    let mut out = Vec::new();
    for path in changed_paths {
        for zone in zones {
            let zone_path = zone.path.trim().trim_matches('/');
            if path == zone_path || path.starts_with(&format!("{zone_path}/")) {
                out.push(GeneratedZoneTouch {
                    path: path.clone(),
                    zone: zone.path.clone(),
                    source: if zone.source.is_empty() {
                        "unknown".to_string()
                    } else {
                        zone.source.clone()
                    },
                    command: zone.command.clone(),
                    read_only: zone.read_only,
                    reason: if zone.source.is_empty() {
                        format!("generated path `{path}` has no source contract")
                    } else {
                        format!(
                            "generated path `{path}` should be edited through source contract `{}` and regenerated with `{}`",
                            zone.source, zone.command
                        )
                    },
                });
            }
        }
    }
    out
}

fn build_ownership_boundaries(
    route_decisions: &[KickoffRouteDecision],
    generated_zone_touches: &[GeneratedZoneTouch],
) -> Vec<OwnershipBoundary> {
    let mut grouped: BTreeMap<String, OwnershipBoundaryBuilder> = BTreeMap::new();
    let generated_paths: BTreeSet<String> = generated_zone_touches
        .iter()
        .map(|touch| touch.path.clone())
        .collect();
    for decision in route_decisions {
        let entry = grouped.entry(decision.owner.clone()).or_default();
        push_unique(&mut entry.paths, decision.path.clone());
        push_unique(&mut entry.owner_routes, decision.owner_route.clone());
        if decision.test_command != "unmapped" {
            push_unique(&mut entry.test_commands, decision.test_command.clone());
        }
        if decision.proof_lane != "unmapped" {
            push_unique(&mut entry.proof_lanes, decision.proof_lane.clone());
        }
        if generated_paths.contains(&decision.path) {
            entry.read_only = true;
        }
        if decision.decision == "human-review" {
            entry.human_review = true;
        }
    }
    grouped
        .into_iter()
        .map(|(owner, builder)| OwnershipBoundary {
            reason: if builder.read_only {
                format!("owner `{owner}` includes generated output and stays read-only")
            } else if builder.human_review {
                format!("owner `{owner}` needs human review before editing")
            } else {
                format!("owner `{owner}` is bounded to the listed paths")
            },
            owner,
            paths: builder.paths,
            owner_routes: builder.owner_routes,
            test_commands: builder.test_commands,
            proof_lanes: builder.proof_lanes,
            write_policy: if builder.read_only {
                "read-only".to_string()
            } else if builder.human_review {
                "human-review".to_string()
            } else {
                "edit".to_string()
            },
        })
        .collect()
}

#[derive(Default)]
struct OwnershipBoundaryBuilder {
    paths: Vec<String>,
    owner_routes: Vec<String>,
    test_commands: Vec<String>,
    proof_lanes: Vec<String>,
    read_only: bool,
    human_review: bool,
}

fn build_clarifying_questions(
    intent: &str,
    changed_paths: &[String],
    route_decisions: &[KickoffRouteDecision],
    generated_zone_touches: &[GeneratedZoneTouch],
    ownership_boundaries: &[OwnershipBoundary],
) -> Vec<ClarifyingQuestion> {
    let mut questions = Vec::new();
    if changed_paths.is_empty() {
        questions.push(ClarifyingQuestion {
            id: "scope".to_string(),
            question: "Which exact files or directories are in scope?".to_string(),
            reason: format!(
                "kickoff needs concrete changed paths before it can hand off bounded work for `{intent}`"
            ),
            suggested_paths: Vec::new(),
            blocking: true,
        });
    }
    for touch in generated_zone_touches {
        if touch.source == "unknown" {
            questions.push(ClarifyingQuestion {
                id: format!("generated-source-{}", slugify(&touch.path)),
                question: format!("What source contract owns generated path `{}`?", touch.path),
                reason: format!(
                    "generated path `{}` needs a source contract before editing `{}`",
                    touch.path, touch.zone
                ),
                suggested_paths: vec![touch.zone.clone()],
                blocking: true,
            });
        }
    }
    let mut owners: Vec<_> = ownership_boundaries
        .iter()
        .map(|boundary| boundary.owner.clone())
        .collect();
    owners.sort();
    owners.dedup();
    if owners.len() > 1 {
        questions.push(ClarifyingQuestion {
            id: "multi-owner".to_string(),
            question: format!(
                "This scope touches multiple owners: {}. Should it split into separate packets or stay coordinated?",
                owners.join(", ")
            ),
            reason: "multiple owners were touched and the work is not yet bounded to a single packet".to_string(),
            suggested_paths: changed_paths.to_vec(),
            blocking: true,
        });
    }
    let mut lane_questions = BTreeMap::new();
    for decision in route_decisions {
        let needs_lane_question = decision.path.starts_with("db/")
            || decision.proof_lane == "security"
            || decision.proof_lane == "release"
            || decision.proof_lane == "ux"
            || decision.path.starts_with("apps/web/")
            || decision.path.starts_with("ui/")
            || decision.path.starts_with(".github/")
            || decision.path.starts_with("release/");
        if needs_lane_question {
            let question = if decision.proof_lane == "unmapped" {
                format!(
                    "Which proof lane should own `{}`? The current route is not backed by a known lane.",
                    decision.path
                )
            } else {
                format!(
                    "Confirm whether `{}` should be routed through the `{}` proof lane before editing `{}`.",
                    decision.path, decision.proof_lane, decision.path
                )
            };
            lane_questions
                .entry(format!("lane-{}", slugify(&decision.path)))
                .or_insert_with(|| ClarifyingQuestion {
                    id: format!("lane-{}", slugify(&decision.path)),
                    question,
                    reason: "security/db/ux/release work needs an explicit lane decision"
                        .to_string(),
                    suggested_paths: vec![decision.path.clone()],
                    blocking: true,
                });
        }
        if decision.decision == "human-review" && decision.proof_lane == "unmapped" {
            lane_questions
                .entry(format!("lane-{}", slugify(&decision.path)))
                .or_insert_with(|| ClarifyingQuestion {
                    id: format!("lane-{}", slugify(&decision.path)),
                    question: format!(
                        "Which proof lane should own `{}`? The current route is not backed by a known lane.",
                        decision.path
                    ),
                    reason: "the path is not backed by a known proof lane".to_string(),
                    suggested_paths: vec![decision.path.clone()],
                    blocking: true,
                });
        }
    }
    questions.extend(lane_questions.into_values());
    if questions.is_empty() && intent.split_whitespace().count() <= 2 {
        questions.push(ClarifyingQuestion {
            id: "scope".to_string(),
            question: format!(
                "What bounded file list should define `{intent}`? The current intent is too broad to route safely."
            ),
            reason: "the request is too broad to map to a bounded patch".to_string(),
            suggested_paths: Vec::new(),
            blocking: true,
        });
    }
    dedupe_questions(questions)
}

fn build_implementation_steps(
    changed_paths: &[String],
    route_decisions: &[KickoffRouteDecision],
    generated_zone_touches: &[GeneratedZoneTouch],
    ownership_boundaries: &[OwnershipBoundary],
    clarifying_questions: &[ClarifyingQuestion],
    next_commands: &[String],
) -> Vec<ImplementationStep> {
    let mut steps = vec![ImplementationStep {
        order: 1,
        title: "Read the kickoff sources".to_string(),
        files: read_first_files(),
        commands: Vec::new(),
        notes: "Start from the repo guardrails and policy maps before editing.".to_string(),
    }];
    let mut order = 2usize;
    steps.push(ImplementationStep {
        order,
        title: "Confirm the file scope".to_string(),
        files: changed_paths.to_vec(),
        commands: Vec::new(),
        notes: if clarifying_questions.is_empty() {
            "Scope appears bounded enough to route.".to_string()
        } else {
            "Answer the blocking clarifying questions before editing.".to_string()
        },
    });
    order += 1;
    for decision in route_decisions {
        steps.push(ImplementationStep {
            order,
            title: format!("Handle {}", decision.path),
            files: vec![decision.path.clone()],
            commands: Vec::new(),
            notes: format!(
                "Route through owner `{}` and proof lane `{}`.",
                decision.owner_route, decision.proof_lane
            ),
        });
        order += 1;
    }
    for boundary in ownership_boundaries {
        steps.push(ImplementationStep {
            order,
            title: format!("Work owner {}", boundary.owner),
            files: boundary.paths.clone(),
            commands: Vec::new(),
            notes: format!(
                "Use proof lanes `{}` and respect `{}`.",
                join_paths(&boundary.proof_lanes),
                boundary.write_policy
            ),
        });
        order += 1;
    }
    for touch in generated_zone_touches {
        steps.push(ImplementationStep {
            order,
            title: format!("Regenerate {}", touch.zone),
            files: vec![touch.zone.clone()],
            commands: Vec::new(),
            notes: format!(
                "Edit source `{}` and regenerate with `{}`.",
                touch.source, touch.command
            ),
        });
        order += 1;
    }
    for command in next_commands.iter().take(3) {
        steps.push(ImplementationStep {
            order,
            title: "Run the next command".to_string(),
            files: changed_paths.to_vec(),
            commands: vec![command.clone()],
            notes: "Use the kickoff chain to build context, proof, and witness receipts."
                .to_string(),
        });
        order += 1;
    }
    if !clarifying_questions.is_empty() {
        steps.push(ImplementationStep {
            order,
            title: "Hold the patch".to_string(),
            files: Vec::new(),
            commands: Vec::new(),
            notes: "Do not widen the patch until every clarifying question is answered."
                .to_string(),
        });
        order += 1;
    }
    steps.push(ImplementationStep {
        order,
        title: "Escalate to repair".to_string(),
        files: Vec::new(),
        commands: vec!["cargo run -p jankurai -- repair-plan <repo> --from target/jankurai/repo-score.json --out target/jankurai/repair-plan.json --md target/jankurai/repair-plan.md".to_string()],
        notes: "If witness blocks the change, switch to repair-plan and then repair/exception handling.".to_string(),
    });
    steps
}

fn build_parallel_packets(route_decisions: &[KickoffRouteDecision]) -> Vec<ParallelPacket> {
    if route_decisions.len() < 2 {
        return Vec::new();
    }
    if route_decisions.iter().any(|decision| {
        decision.decision != "allowed"
            || decision.owner == "unmapped"
            || decision.proof_lane == "unmapped"
            || decision.generated_zone
    }) {
        return Vec::new();
    }
    let mut owners = BTreeSet::new();
    let mut lanes = BTreeSet::new();
    for decision in route_decisions {
        if !owners.insert(decision.owner.clone()) || !lanes.insert(decision.proof_lane.clone()) {
            return Vec::new();
        }
    }
    route_decisions
        .iter()
        .enumerate()
        .map(|(idx, decision)| ParallelPacket {
            name: format!("packet-{}", idx + 1),
            paths: vec![decision.path.clone()],
            owner: decision.owner.clone(),
            proof_lane: decision.proof_lane.clone(),
            commands: Vec::new(),
            expected_receipts: Vec::new(),
            stop_conditions: vec![
                "stop if the packet expands beyond its own owner boundary".to_string(),
                "stop if the packet needs a generated output edit".to_string(),
            ],
            reason: format!(
                "owner `{}` and proof lane `{}` are disjoint from the other packets",
                decision.owner, decision.proof_lane
            ),
        })
        .collect()
}

fn build_stop_conditions(
    changed_paths: &[String],
    route_decisions: &[KickoffRouteDecision],
    generated_zone_touches: &[GeneratedZoneTouch],
    ownership_boundaries: &[OwnershipBoundary],
    clarifying_questions: &[ClarifyingQuestion],
) -> Vec<String> {
    let mut conditions = vec![
        "stop if the work expands outside the listed ownership boundaries".to_string(),
        "stop if any generated zone is edited before its source contract and regeneration command are known".to_string(),
        "stop if a required proof lane or test command is missing for any changed path".to_string(),
        "stop if the change needs to write outside `target/jankurai/` or the repo's read-only areas".to_string(),
    ];
    if changed_paths.is_empty() {
        conditions.push(
            "stop until a bounded changed-path list or `--changed-from` ref is supplied"
                .to_string(),
        );
    }
    if !generated_zone_touches.is_empty() {
        for touch in generated_zone_touches {
            if touch.source == "unknown" {
                push_unique(
                    &mut conditions,
                    format!(
                        "stop until the source contract for generated path `{}` is identified",
                        touch.path
                    ),
                );
            } else {
                push_unique(
                    &mut conditions,
                    format!(
                        "stop until `{}` is updated before regenerating `{}`",
                        touch.source, touch.zone
                    ),
                );
            }
        }
    }
    if ownership_boundaries.len() > 1 {
        conditions
            .push("stop if a cross-owner edit is not split into separate packets".to_string());
    }
    if !clarifying_questions.is_empty() {
        conditions.push("stop until every clarifying question is answered".to_string());
    }
    for decision in route_decisions {
        if decision.decision == "human-review" {
            push_unique(
                &mut conditions,
                format!(
                    "stop until `{}` has explicit human review approval",
                    decision.path
                ),
            );
        }
    }
    conditions
}

fn build_expected_receipts() -> Vec<String> {
    vec![
        "target/jankurai/kickoff.json",
        "target/jankurai/kickoff.md",
        "target/jankurai/context-pack.json",
        "target/jankurai/context-pack.md",
        "target/jankurai/proof-plan.json",
        "target/jankurai/proof-plan.md",
        "target/jankurai/proof-receipts/",
        "target/jankurai/evidence-index.json",
        "target/jankurai/merge-witness.json",
        "target/jankurai/merge-witness.md",
        "target/jankurai/repair-plan.json",
        "target/jankurai/repair-plan.md",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn build_next_commands(
    repo: &Path,
    intent: &str,
    changed_from: Option<&str>,
    changed_paths: &[String],
    route_decisions: &[KickoffRouteDecision],
    generated_zone_touches: &[GeneratedZoneTouch],
) -> Vec<String> {
    if changed_paths.is_empty() {
        return vec![
            format!(
                "cargo run -p jankurai -- kickoff {} --intent {} --changed <PATH>... --out target/jankurai/kickoff.json --md target/jankurai/kickoff.md",
                shell_quote(&repo.display().to_string()),
                shell_quote(intent)
            ),
            format!(
                "cargo run -p jankurai -- context-pack {} --task {} --changed <PATH>... --out target/jankurai/context-pack.json --md target/jankurai/context-pack.md",
                shell_quote(&repo.display().to_string()),
                shell_quote(intent)
            ),
        ];
    }
    let changed_args = build_changed_args(changed_paths);
    let kickoff_args = if let Some(base) = changed_from {
        format!("--changed-from {}", shell_quote(base))
    } else {
        changed_args.clone()
    };
    let mut commands = vec![
        format!(
            "cargo run -p jankurai -- context-pack {} --task {} {} --out target/jankurai/context-pack.json --md target/jankurai/context-pack.md",
            shell_quote(&repo.display().to_string()),
            shell_quote(intent),
            changed_args
        ),
        format!(
            "cargo run -p jankurai -- prove {} {} --plan-out target/jankurai/proof-plan.json --plan-md target/jankurai/proof-plan.md --out-dir target/jankurai/proof-receipts --evidence-index target/jankurai/evidence-index.json",
            shell_quote(&repo.display().to_string()),
            changed_args
        ),
        format!(
            "cargo run -p jankurai -- witness {} {} --out target/jankurai/merge-witness.json --md target/jankurai/merge-witness.md",
            shell_quote(&repo.display().to_string()),
            changed_args
        ),
        format!(
            "cargo run -p jankurai -- kickoff {} --intent {} {} --out target/jankurai/kickoff.json --md target/jankurai/kickoff.md",
            shell_quote(&repo.display().to_string()),
            shell_quote(intent),
            kickoff_args
        ),
    ];
    if !generated_zone_touches.is_empty()
        || route_decisions
            .iter()
            .any(|decision| decision.decision == "human-review")
    {
        commands.push(format!(
            "cargo run -p jankurai -- repair-plan {} --from target/jankurai/repo-score.json --out target/jankurai/repair-plan.json --md target/jankurai/repair-plan.md",
            shell_quote(&repo.display().to_string())
        ));
    }
    commands
}

fn build_forbidden_paths(catalog: &RepoCatalog) -> Vec<String> {
    let mut forbidden = vec![
        "reference/".to_string(),
        "paper/tex/generated/".to_string(),
        "paper/jankurai.pdf".to_string(),
    ];
    for path in catalog.forbidden_generated_paths() {
        push_unique(&mut forbidden, path);
    }
    forbidden
}

fn collect_proof_lanes(route_decisions: &[KickoffRouteDecision]) -> Vec<String> {
    let mut lanes = Vec::new();
    for decision in route_decisions {
        if decision.proof_lane != "unmapped" {
            push_unique(&mut lanes, decision.proof_lane.clone());
        }
    }
    lanes
}

fn generated_zone_match<'a>(zones: &'a [GeneratedZone], path: &str) -> Option<&'a GeneratedZone> {
    zones.iter().find(|zone| {
        let zone_path = zone.path.trim().trim_matches('/');
        path == zone_path || path.starts_with(&format!("{zone_path}/"))
    })
}

fn normalize_changed_paths(
    repo: &Path,
    changed: &[PathBuf],
    changed_from: Option<&str>,
) -> Result<Vec<String>> {
    let mut paths = BTreeSet::new();
    for path in changed {
        let rel = normalize_changed_path(repo, path)?;
        insert_changed_path(&mut paths, rel, path)?;
    }
    if let Some(base_ref) = changed_from {
        for path in crate::audit::changed_paths_from_git(repo, base_ref)? {
            let rel = normalize_changed_path(repo, &path)?;
            insert_changed_path(&mut paths, rel, path.as_path())?;
        }
    }
    Ok(paths.into_iter().collect())
}

fn normalize_changed_path(root: &Path, path: &Path) -> Result<String> {
    if path.is_absolute() {
        if path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
            || !path.starts_with(root)
        {
            bail!(
                "changed path `{}` resolves outside repository root `{}`",
                path.display(),
                root.display()
            );
        }
        let rel = path.strip_prefix(root).with_context(|| {
            format!(
                "changed path `{}` resolves outside repository root `{}`",
                path.display(),
                root.display()
            )
        })?;
        return Ok(rel.to_string_lossy().replace('\\', "/"));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        bail!(
            "changed path `{}` resolves outside repository root `{}`",
            path.display(),
            root.display()
        );
    }
    let candidate = root.join(path);
    let rel = candidate.strip_prefix(root).with_context(|| {
        format!(
            "changed path `{}` resolves outside repository root `{}`",
            path.display(),
            root.display()
        )
    })?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

fn insert_changed_path(paths: &mut BTreeSet<String>, rel: String, original: &Path) -> Result<()> {
    let normalized = rel
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string();
    if normalized.is_empty() || normalized == "." {
        bail!(
            "changed path `{}` resolves to the repository root; pass explicit changed files or a non-root subdirectory",
            original.display()
        );
    }
    paths.insert(normalized);
    Ok(())
}

fn build_changed_args(changed_paths: &[String]) -> String {
    changed_paths
        .iter()
        .map(|path| format!("--changed {}", shell_quote(path)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn join_paths(paths: &[String]) -> String {
    if paths.is_empty() {
        return "none".to_string();
    }
    paths.join(", ")
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}


fn dedupe_questions(values: Vec<ClarifyingQuestion>) -> Vec<ClarifyingQuestion> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.id.clone()) {
            out.push(value);
        }
    }
    out
}

fn slugify(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn render_markdown(report: &KickoffReport) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# jankurai Kickoff");
    let _ = writeln!(out);
    let _ = writeln!(out, "- command: `{}`", report.command);
    let _ = writeln!(out, "- repo: `{}`", report.repo);
    let _ = writeln!(out, "- intent: {}", report.intent);
    let _ = writeln!(out, "- agent: `{}`", report.agent);
    let _ = writeln!(out, "- changed: `{}`", join_paths(&report.changed_paths));
    let _ = writeln!(out);
    let _ = writeln!(out, "## Read first");
    for path in &report.read_first {
        let _ = writeln!(out, "- `{}`", path);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Route decisions");
    for decision in &report.route_decisions {
        let _ = writeln!(
            out,
            "- `{}` -> `{}`: {}",
            decision.path, decision.decision, decision.reason
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Ownership boundaries");
    for boundary in &report.ownership_boundaries {
        let _ = writeln!(
            out,
            "- `{}`: `{}` [{}]",
            boundary.owner,
            join_paths(&boundary.paths),
            boundary.write_policy
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Forbidden paths");
    if report.forbidden_paths.is_empty() {
        let _ = writeln!(out, "- none");
    } else {
        for path in &report.forbidden_paths {
            let _ = writeln!(out, "- `{}`", path);
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Proof lanes");
    if report.proof_lanes.is_empty() {
        let _ = writeln!(out, "- none");
    } else {
        for lane in &report.proof_lanes {
            let _ = writeln!(out, "- `{}`", lane);
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Generated zone touches");
    if report.generated_zone_touches.is_empty() {
        let _ = writeln!(out, "- none");
    } else {
        for touch in &report.generated_zone_touches {
            let _ = writeln!(
                out,
                "- `{}` -> `{}` via `{}`",
                touch.path, touch.zone, touch.command
            );
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Clarifying questions");
    if report.clarifying_questions.is_empty() {
        let _ = writeln!(out, "- none");
    } else {
        for question in &report.clarifying_questions {
            let _ = writeln!(out, "- `{}`: {}", question.id, question.question);
            let _ = writeln!(out, "  - reason: {}", question.reason);
            let _ = writeln!(
                out,
                "  - suggested paths: `{}`",
                join_paths(&question.suggested_paths)
            );
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Implementation steps");
    for step in &report.implementation_steps {
        let _ = writeln!(out, "- {}. {}", step.order, step.title);
        let _ = writeln!(out, "  - files: `{}`", join_paths(&step.files));
        let _ = writeln!(out, "  - commands: `{}`", join_paths(&step.commands));
        let _ = writeln!(out, "  - notes: {}", step.notes);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Parallel packets");
    if report.parallel_packets.is_empty() {
        let _ = writeln!(out, "- none");
    } else {
        for packet in &report.parallel_packets {
            let _ = writeln!(out, "- `{}`: `{}`", packet.name, join_paths(&packet.paths));
            let _ = writeln!(out, "  - owner: `{}`", packet.owner);
            let _ = writeln!(out, "  - proof lane: `{}`", packet.proof_lane);
            let _ = writeln!(out, "  - commands: `{}`", join_paths(&packet.commands));
            let _ = writeln!(
                out,
                "  - expected receipts: `{}`",
                join_paths(&packet.expected_receipts)
            );
            let _ = writeln!(
                out,
                "  - stop conditions: `{}`",
                join_paths(&packet.stop_conditions)
            );
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Stop conditions");
    for stop in &report.stop_conditions {
        let _ = writeln!(out, "- {}", stop);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Expected receipts");
    for receipt in &report.expected_receipts {
        let _ = writeln!(out, "- `{}`", receipt);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Next commands");
    for command in &report.next_commands {
        let _ = writeln!(out, "- `{}`", command);
    }
    out
}
