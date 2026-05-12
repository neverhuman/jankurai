use anyhow::Result;

pub fn merge_marker(path: &str) -> String {
    format!("\n<!-- jankurai merge marker: review and merge canonical guidance for {path} -->\n")
}

pub fn merge_json(existing: &str, template: &str) -> Result<String> {
    let mut base: serde_json::Value = if existing.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(existing).unwrap_or_else(|_| serde_json::json!({}))
    };
    let new: serde_json::Value = serde_json::from_str(template)?;

    merge_json_values(&mut base, &new);
    Ok(serde_json::to_string_pretty(&base)?)
}

fn merge_json_values(base: &mut serde_json::Value, new: &serde_json::Value) {
    match (base, new) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(new_map)) => {
            for (k, v) in new_map {
                if !base_map.contains_key(k) {
                    base_map.insert(k.clone(), v.clone());
                } else {
                    let existing = base_map.get_mut(k).unwrap();
                    merge_json_values(existing, v);
                }
            }
        }
        (serde_json::Value::Array(base_arr), serde_json::Value::Array(new_arr)) => {
            for item in new_arr {
                if !base_arr.contains(item) {
                    base_arr.push(item.clone());
                }
            }
        }
        _ => {} // Don't overwrite existing primitives or mismatched types
    }
}

pub fn merge_toml(existing: &str, template: &str) -> Result<String> {
    let mut base: toml::Value = if existing.trim().is_empty() {
        toml::Value::Table(toml::map::Map::new())
    } else {
        toml::from_str(existing).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()))
    };
    let new: toml::Value = toml::from_str(template)?;

    merge_toml_values(&mut base, &new);
    // toml::to_string_pretty handles serialization cleanly
    Ok(toml::to_string_pretty(&base)?)
}

pub fn merge_standard_version_toml(existing: &str, template: &str) -> Result<String> {
    let mut base: toml::Value = if existing.trim().is_empty() {
        toml::Value::Table(toml::map::Map::new())
    } else {
        toml::from_str(existing).unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()))
    };
    let new: toml::Value = toml::from_str(template)?;

    merge_toml_values(&mut base, &new);
    if let (toml::Value::Table(base_map), toml::Value::Table(new_map)) = (&mut base, new) {
        for key in [
            "standard",
            "standard_version",
            "paper_edition",
            "auditor_version",
            "schema_version",
            "target_stack",
        ] {
            if let Some(value) = new_map.get(key) {
                base_map.insert(key.to_string(), value.clone());
            }
        }
    }
    Ok(toml::to_string_pretty(&base)?)
}

fn merge_toml_values(base: &mut toml::Value, new: &toml::Value) {
    match (base, new) {
        (toml::Value::Table(base_map), toml::Value::Table(new_map)) => {
            for (k, v) in new_map {
                if !base_map.contains_key(k) {
                    base_map.insert(k.clone(), v.clone());
                } else {
                    let existing = base_map.get_mut(k).unwrap();
                    merge_toml_values(existing, v);
                }
            }
        }
        (toml::Value::Array(base_arr), toml::Value::Array(new_arr)) => {
            for item in new_arr {
                if !base_arr.contains(item) {
                    base_arr.push(item.clone());
                }
            }
        }
        _ => {} // Don't overwrite existing primitives or mismatched types
    }
}

pub fn merge_lines(existing: &str, template: &str) -> Result<String> {
    let mut out = String::from(existing);
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    let existing_lines: std::collections::HashSet<&str> =
        existing.lines().map(|s| s.trim()).collect();
    let mut skip_existing_recipe_block = false;
    let mut append_new_recipe_block = false;
    for line in template.lines() {
        let trimmed = line.trim();
        let is_recipe_header = is_recipe_header(line);
        let is_indented = line.starts_with(' ') || line.starts_with('\t');

        if is_recipe_header {
            append_new_recipe_block = false;
            if existing_lines.contains(trimmed) {
                skip_existing_recipe_block = true;
                continue;
            }
            skip_existing_recipe_block = false;
            append_new_recipe_block = true;
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if skip_existing_recipe_block && is_indented {
            continue;
        }

        if append_new_recipe_block && is_indented {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        skip_existing_recipe_block = false;
        append_new_recipe_block = false;

        if trimmed.is_empty() || existing_lines.contains(trimmed) {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    Ok(out)
}

fn is_recipe_header(line: &str) -> bool {
    if line.starts_with(' ') || line.starts_with('\t') {
        return false;
    }
    let trimmed = line.trim();
    !trimmed.is_empty() && !trimmed.starts_with('#') && trimmed.contains(':')
}

#[cfg(test)]
mod tests {
    use super::{merge_lines, merge_standard_version_toml};

    #[test]
    fn merge_standard_version_toml_replaces_version_keys_and_preserves_extras() {
        let existing = r#"auditor_version = "1.1.0"
schema_version = "1.7.0"
custom_note = "keep"
"#;
        let template = r#"standard = "jankurai"
standard_version = "0.9.0"
paper_edition = "2026.05-ed8"
auditor_version = "1.2.0"
schema_version = "1.8.0"
target_stack = "rust-ts-vite-react-postgres-bounded-python"
"#;

        let merged = merge_standard_version_toml(existing, template).unwrap();

        assert!(merged.contains("auditor_version = \"1.2.0\""));
        assert!(merged.contains("schema_version = \"1.8.0\""));
        assert!(merged.contains("custom_note = \"keep\""));
    }

    #[test]
    fn merge_lines_skips_existing_just_recipe_body() {
        let existing = "fast:\n    cargo check --workspace\n";
        let template = "# jankurai scaffold Justfile\n\nfast:\n\tjankurai doctor --fail-on critical\n\nscore:\n\tjankurai audit . --mode advisory\n";

        let merged = merge_lines(existing, template).unwrap();

        assert!(merged.contains("fast:\n    cargo check --workspace\n"));
        assert!(!merged.contains(
            "fast:\n    cargo check --workspace\n# jankurai scaffold Justfile\njankurai doctor"
        ));
        assert!(!merged.contains("jankurai doctor --fail-on critical"));
        assert!(merged.contains("score:\n\tjankurai audit . --mode advisory\n"));
    }

    #[test]
    fn merge_lines_appends_complete_missing_just_recipe_even_if_body_line_exists() {
        let existing = "fast:\n\tjankurai audit . --mode advisory\n";
        let template = "score:\n\tjankurai audit . --mode advisory\n";

        let merged = merge_lines(existing, template).unwrap();

        assert!(merged.contains("score:\n\tjankurai audit . --mode advisory\n"));
    }
}
