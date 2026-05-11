use crate::model::Report;

pub fn render_junit(report: &Report) -> String {
    let failures = report.findings.len();
    let mut out = String::new();
    use std::fmt::Write;
    let _ = writeln!(
        out,
        r#"<testsuite name="jankurai" tests="{}" failures="{}">"#,
        failures.max(1),
        failures
    );
    if report.findings.is_empty() {
        let _ = writeln!(out, r#"  <testcase classname="jankurai" name="score"/>"#);
    } else {
        for finding in &report.findings {
            let name = xml_escape(&format!(
                "{} {}",
                finding
                    .rule_id
                    .as_deref()
                    .unwrap_or("HLT-000-SCORE-DIMENSION"),
                finding.path
            ));
            let _ = writeln!(out, r#"  <testcase classname="jankurai" name="{}">"#, name);
            let _ = writeln!(
                out,
                r#"    <failure type="{}" message="{}">{}</failure>"#,
                xml_escape(&finding.severity),
                xml_escape(&finding.problem),
                xml_escape(&finding.agent_fix)
            );
            let _ = writeln!(out, "  </testcase>");
        }
    }
    let _ = writeln!(out, "</testsuite>");
    out
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
