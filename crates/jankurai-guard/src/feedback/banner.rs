//! PTY failure-banner formatting. [`format_banner`] turns a blocking
//! [`GuardDecision`] into a compact box-drawing block suitable for injecting
//! straight into the agent's terminal so the failure is impossible to miss.
//!
//! The function is pure: it takes a decision and returns a string, which makes
//! it golden-testable.

use crate::audit_client::GuardDecision;

/// Inner width of the banner box, in characters.
const WIDTH: usize = 72;

/// Formats `decision` as a box-drawing banner. The banner is safe to write
/// verbatim into a PTY: it uses only printable characters plus newlines.
pub fn format_banner(decision: &GuardDecision) -> String {
    let mut out = String::new();
    out.push_str(&top_rule());
    out.push_str(&row(&format!("JANKURAI GUARD: BLOCKED  {}", decision.path)));
    out.push_str(&divider());

    if decision.summary.is_empty() {
        out.push_str(&row("write rejected by audit"));
    } else {
        out.push_str(&row(&decision.summary));
    }

    let blocking: Vec<_> = decision.blocking.all().collect();
    if blocking.is_empty() {
        out.push_str(&row("no individual findings reported"));
    } else {
        for finding in blocking.iter().take(6) {
            let line = match finding.line {
                Some(n) => format!("[BLOCK] {} L{}  {}", finding.rule_id, n, finding.problem),
                None => format!("[BLOCK] {}  {}", finding.rule_id, finding.problem),
            };
            out.push_str(&row(&line));
            if !finding.agent_fix.is_empty() {
                out.push_str(&row(&format!("        fix: {}", finding.agent_fix)));
            }
        }
        if blocking.len() > 6 {
            out.push_str(&row(&format!("... and {} more", blocking.len() - 6)));
        }
    }

    if !decision.rerun_command.is_empty() {
        out.push_str(&divider());
        out.push_str(&row(&format!("re-run: {}", decision.rerun_command)));
    }
    out.push_str(&bottom_rule());
    out
}

/// The top border line of the box.
fn top_rule() -> String {
    format!("\r\n┌{}┐\r\n", "─".repeat(WIDTH + 2))
}

/// The bottom border line of the box.
fn bottom_rule() -> String {
    format!("└{}┘\r\n", "─".repeat(WIDTH + 2))
}

/// An interior divider line.
fn divider() -> String {
    format!("├{}┤\r\n", "─".repeat(WIDTH + 2))
}

/// A content row: the text is clipped or padded to the box width and wrapped in
/// the side borders. Long text is split across multiple rows.
fn row(text: &str) -> String {
    let mut out = String::new();
    for chunk in wrap(text, WIDTH) {
        let pad = WIDTH.saturating_sub(visible_len(&chunk));
        out.push_str(&format!("│ {chunk}{} │\r\n", " ".repeat(pad)));
    }
    out
}

/// Splits `text` into chunks no wider than `width` visible characters, breaking
/// on character boundaries (the banner content is plain ASCII-ish text).
fn wrap(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut count = 0usize;
    for ch in text.chars() {
        if count == width {
            chunks.push(std::mem::take(&mut current));
            count = 0;
        }
        current.push(ch);
        count += 1;
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// The number of visible characters in `text` (it carries no escape sequences,
/// so this is just the char count).
fn visible_len(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit_client::MockAuditClient;
    use crate::AuditClient;
    use std::path::Path;

    #[test]
    fn banner_contains_path_and_findings() {
        let decision = MockAuditClient::always_block()
            .audit(Path::new("."), Path::new("src/foo.rs"), b"x")
            .unwrap();
        let banner = format_banner(&decision);
        assert!(banner.contains("JANKURAI GUARD: BLOCKED"));
        assert!(banner.contains("src/foo.rs"));
        assert!(banner.contains("[BLOCK]"));
        assert!(banner.contains("re-run:"));
    }
}
