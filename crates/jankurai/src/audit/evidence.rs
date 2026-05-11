use crate::model::FileInfo;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

pub fn operational_command_text(files: &[FileInfo]) -> String {
    operational_command_lines(files).join("\n")
}

pub fn operational_command_lines(files: &[FileInfo]) -> Vec<String> {
    let mut lines = Vec::new();
    for file in files {
        match file.name.as_str() {
            "Justfile" | "justfile" | "Makefile" | "makefile" => {
                lines.extend(shell_lines(&file.text));
            }
            "package.json" => {
                lines.extend(package_scripts(&file.text));
            }
            "Taskfile.yml" | "Taskfile.yaml" | "taskfile.yml" | "taskfile.yaml" => {
                lines.extend(shell_lines(&file.text));
            }
            _ if file.rel_path.starts_with(".github/workflows/") => {
                lines.extend(github_workflow_commands(&file.text));
            }
            _ => {}
        }
    }
    lines
}

fn package_scripts(text: &str) -> Vec<String> {
    let parsed = match serde_json::from_str::<JsonValue>(text) {
        Ok(parsed) => parsed,
        Err(_) => return vec![],
    };
    parsed
        .get("scripts")
        .and_then(|scripts| scripts.as_object())
        .into_iter()
        .flat_map(|scripts| scripts.values())
        .filter_map(|value| value.as_str())
        .flat_map(shell_lines)
        .collect()
}

fn github_workflow_commands(text: &str) -> Vec<String> {
    let parsed = match serde_yaml::from_str::<YamlValue>(text) {
        Ok(parsed) => parsed,
        Err(_) => return vec![],
    };
    let mut lines = Vec::new();
    let Some(jobs) = parsed.get("jobs").and_then(|jobs| jobs.as_mapping()) else {
        return lines;
    };
    for job in jobs.values() {
        let Some(steps) = job.get("steps").and_then(|steps| steps.as_sequence()) else {
            continue;
        };
        for step in steps {
            if let Some(run) = step.get("run").and_then(|run| run.as_str()) {
                lines.extend(shell_lines(run));
            }
            if let Some(uses) = step.get("uses").and_then(|uses| uses.as_str()) {
                lines.push(format!("uses: {}", uses.to_ascii_lowercase()));
            }
        }
    }
    lines
}

fn shell_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('#'))
        .filter(|line| !line.starts_with("//"))
        .filter(|line| !line.starts_with("echo "))
        .filter(|line| *line != "{" && *line != "}")
        .map(|line| line.to_ascii_lowercase())
        .collect()
}
