use crate::shared::sanitize;

pub(crate) fn surface_id(surface_type: &str, path: &str, symbol: &str) -> String {
    format!(
        "surface:{}:{}:{}",
        sanitize(surface_type),
        sanitize(path),
        sanitize(symbol)
    )
}

pub(crate) fn rust_public_symbols(text: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("pub ") else {
            continue;
        };
        if rest.starts_with("fn ")
            || rest.starts_with("async fn ")
            || rest.starts_with("struct ")
            || rest.starts_with("enum ")
            || rest.starts_with("trait ")
            || rest.starts_with("mod ")
            || rest.starts_with("type ")
        {
            if let Some(name) = rust_symbol_name(rest) {
                symbols.push(name);
            }
        }
    }
    symbols.sort();
    symbols.dedup();
    symbols.truncate(24);
    symbols
}

fn rust_symbol_name(rest: &str) -> Option<String> {
    let rest = if let Some(rest) = rest.strip_prefix("async ") {
        rest
    } else {
        rest
    };
    let mut matched = None;
    for prefix in ["fn ", "struct ", "enum ", "trait ", "mod ", "type "] {
        if let Some(value) = rest.strip_prefix(prefix) {
            matched = Some(value);
            break;
        }
    }
    let rest = matched?;
    let name = rest
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .next()?
        .trim();
    if name.is_empty() {
        None
    } else {
        Some(name.into())
    }
}

pub(crate) fn contains_authz_marker(path: &str, text: &str) -> bool {
    path.contains("auth")
        || text.contains("authorize")
        || text.contains("authorization")
        || text.contains("permission")
        || text.contains("tenant_id")
        || text.contains("owner_id")
        || text.contains("role")
}

pub(crate) fn contains_input_marker(path: &str, text: &str) -> bool {
    path.contains("parser")
        || path.contains("request")
        || text.contains("from_str")
        || text.contains("parse(")
        || text.contains("deserialize")
        || text.contains("inner_html")
        || text.contains(&["select", " * from"].concat())
        || text.contains("format!(\"select")
}

pub(crate) fn contains_process_sink(text: &str) -> bool {
    text.contains("unsafe ")
        || text.contains(&["command", "::new"].concat())
        || text.contains("std::process")
        || text.contains("remove_file")
        || text.contains("remove_dir")
        || text.contains("fs::write")
}

pub(crate) fn contains_destructive_sql(text: &str) -> bool {
    [
        "drop table",
        "drop column",
        "truncate",
        "delete from",
        "alter table",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

pub(crate) fn is_agent_tool_surface(path: &str, text: &str) -> bool {
    path.starts_with("agent/")
        || path.starts_with(".agents/")
        || path.starts_with(".cursor/")
        || path.starts_with(".github/workflows/")
        || path.contains("mcp")
        || text.contains("mcp")
        || text.contains("tool")
}
