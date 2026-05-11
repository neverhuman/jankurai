use super::super::helpers::AuditContext;
use crate::audit::scan;
use crate::model::FileInfo;
use quote::ToTokens;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, Serialize)]
pub struct TuiwrightEvidence {
    pub surface_detected: bool,
    pub flow_count: usize,
    pub test_files: Vec<String>,
    pub covered_flows: Vec<TuiwrightFlowEvidence>,
    pub assertion_count: usize,
    pub action_count: usize,
    pub artifact_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TuiwrightFlowEvidence {
    pub file: String,
    pub test_name: String,
    pub strength: String,
    pub assertion_signals: Vec<String>,
    pub action_signals: Vec<String>,
    pub artifact_signals: Vec<String>,
}

pub fn analyze(ctx: &AuditContext) -> Option<TuiwrightEvidence> {
    let mut flows = Vec::new();
    let mut test_files = BTreeSet::new();

    for file in &ctx.all_files {
        if !should_scan(file) {
            continue;
        }
        let Ok(parsed) = syn::parse_file(&file.text) else {
            continue;
        };
        collect_flows(&file.rel_path, &parsed.items, &mut flows, &mut test_files);
    }

    if flows.is_empty() {
        return None;
    }

    let mut artifact_counts = BTreeMap::new();
    let mut assertion_count = 0usize;
    let mut action_count = 0usize;
    for flow in &flows {
        assertion_count += flow.assertion_signals.len();
        action_count += flow.action_signals.len();
        for artifact in &flow.artifact_signals {
            *artifact_counts.entry(artifact.clone()).or_insert(0) += 1;
        }
    }

    Some(TuiwrightEvidence {
        surface_detected: true,
        flow_count: flows.len(),
        test_files: test_files.into_iter().collect(),
        covered_flows: flows,
        assertion_count,
        action_count,
        artifact_counts,
    })
}

fn collect_flows(
    file: &str,
    items: &[syn::Item],
    out: &mut Vec<TuiwrightFlowEvidence>,
    test_files: &mut BTreeSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Fn(func) if is_test_function(&func.attrs) => {
                let signals = function_signals(&func.block);
                if !signals.spawned || signals.assertion_signals.is_empty() {
                    continue;
                }
                test_files.insert(file.to_string());
                out.push(TuiwrightFlowEvidence {
                    file: file.to_string(),
                    test_name: func.sig.ident.to_string(),
                    strength: if signals.action_signals.is_empty()
                        && signals.artifact_signals.is_empty()
                    {
                        "assertion-backed".into()
                    } else {
                        "interaction-backed".into()
                    },
                    assertion_signals: signals.assertion_signals,
                    action_signals: signals.action_signals,
                    artifact_signals: signals.artifact_signals,
                });
            }
            syn::Item::Mod(module) => {
                if let Some((_, nested_items)) = &module.content {
                    collect_flows(file, nested_items, out, test_files);
                }
            }
            _ => {}
        }
    }
}

fn should_scan(file: &FileInfo) -> bool {
    file.suffix == ".rs"
        && !scan::is_generated_or_reference_path(&file.rel_path)
        && !file.rel_path.contains("/fixtures/")
        && !file.rel_path.starts_with("fixtures/")
}

struct FunctionSignals {
    spawned: bool,
    assertion_signals: Vec<String>,
    action_signals: Vec<String>,
    artifact_signals: Vec<String>,
}

fn function_signals(block: &syn::Block) -> FunctionSignals {
    let text = block.to_token_stream().to_string().to_ascii_lowercase();
    let compact = text.split_whitespace().collect::<String>();
    FunctionSignals {
        spawned: contains_any(
            &compact,
            &["page::spawn", "spawnconfig::new", "spawnconfig{"],
        ) || contains_any(
            &text,
            &["page :: spawn", "spawnconfig :: new", "spawnconfig {"],
        ),
        assertion_signals: signals_for(
            &compact,
            &[
                "wait_for_text",
                "wait_for_regex",
                "expect_screen",
                "expect_locator",
                "to_contain_text",
                "to_match_regex",
                "to_be_visible",
                "to_have_text",
                "wait_until_idle",
            ],
        ),
        action_signals: signals_for(
            &compact,
            &["press(", "type_text(", "paste(", "click_cell(", "resize("],
        ),
        artifact_signals: signals_for(
            &compact,
            &["screenshot(", "stop_recording_gif(", "trace_path("],
        ),
    }
}

fn is_test_function(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("test")
            || attr
                .path()
                .segments
                .last()
                .map(|segment| segment.ident == "test")
                .unwrap_or(false)
    })
}

fn signals_for(text: &str, needles: &[&str]) -> Vec<String> {
    needles
        .iter()
        .filter(|needle| text.contains(**needle))
        .map(|needle| needle.trim_end_matches('(').to_string())
        .collect()
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}
