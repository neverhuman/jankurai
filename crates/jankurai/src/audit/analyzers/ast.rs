use crate::audit::helpers::product_code_files;
use crate::audit::helpers::AuditContext;
use crate::audit::scan::FindingHit;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct ImportEdge {
    pub source_file: String,
    pub target_module: String,
    pub line_number: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    pub edges: Vec<ImportEdge>,
    // source file -> target modules
    pub adjacency: BTreeMap<String, BTreeSet<String>>,
}

impl DependencyGraph {
    pub fn add_edge(&mut self, edge: ImportEdge) {
        self.adjacency
            .entry(edge.source_file.clone())
            .or_default()
            .insert(edge.target_module.clone());
        self.edges.push(edge);
    }
}

pub fn parse_rust_imports(file_path: &str, text: &str, graph: &mut DependencyGraph) {
    static RUST_USE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*use\s+([a-zA-Z0-9_:]+)").unwrap());
    for (i, line) in text.lines().enumerate() {
        if let Some(caps) = RUST_USE.captures(line) {
            if let Some(m) = caps.get(1) {
                let target_module = m.as_str().to_string();
                graph.add_edge(ImportEdge {
                    source_file: file_path.to_string(),
                    target_module,
                    line_number: i + 1,
                });
            }
        }
    }
}

pub fn parse_typescript_imports(file_path: &str, text: &str, graph: &mut DependencyGraph) {
    // Basic match: import { ... } from 'module'; or import x from 'module';
    static TS_IMPORT: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"^\s*import\s+.*from\s+['"]([^'"]+)['"]"#).unwrap());
    for (i, line) in text.lines().enumerate() {
        if let Some(caps) = TS_IMPORT.captures(line) {
            if let Some(m) = caps.get(1) {
                let target_module = m.as_str().to_string();
                graph.add_edge(ImportEdge {
                    source_file: file_path.to_string(),
                    target_module,
                    line_number: i + 1,
                });
            }
        }
    }
}

pub fn run_ast_pilot(ctx: &AuditContext) -> Vec<FindingHit> {
    let mut graph = DependencyGraph::default();

    for file in product_code_files(ctx) {
        if file.suffix == ".rs" {
            parse_rust_imports(&file.rel_path, &file.text, &mut graph);
        } else if file.suffix == ".ts" || file.suffix == ".tsx" {
            parse_typescript_imports(&file.rel_path, &file.text, &mut graph);
        }
    }

    let mut hits = Vec::new();
    let domain_forbidden = crate::boundaries::rust::DOMAIN_FORBIDDEN_IMPORTS;

    for edge in graph.edges {
        // Rust domain impurity check
        if edge.source_file.starts_with("crates/domain/") || edge.source_file.starts_with("domain/")
        {
            for forbidden in domain_forbidden {
                if edge.target_module.starts_with(forbidden) || edge.target_module == *forbidden {
                    hits.push(FindingHit {
                        path: edge.source_file.clone(),
                        line: Some(edge.line_number),
                        text: format!("use {}", edge.target_module),
                        matched_term: Some(forbidden.to_string()),
                        agent_fix: "extract IO/database operations to adapters and use dependency injection or generic traits".to_string(),
                        problem: format!("domain logic imports forbidden IO/DB module `{}`", edge.target_module),
                    });
                }
            }
        }

        // TypeScript UI layer checking for backend imports
        if (edge.source_file.starts_with("apps/web/") || edge.source_file.starts_with("frontend/"))
            && (edge.target_module.contains("backend")
                || edge.target_module.contains("adapters/db"))
        {
            hits.push(FindingHit {
                path: edge.source_file.clone(),
                line: Some(edge.line_number),
                text: format!("import ... from '{}'", edge.target_module),
                matched_term: Some(edge.target_module.clone()),
                agent_fix:
                    "use HTTP/API clients instead of directly importing backend code in the UI"
                        .to_string(),
                problem: format!(
                    "UI layer directly imports backend module `{}`",
                    edge.target_module
                ),
            });
        }
    }

    hits
}
