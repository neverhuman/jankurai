use crate::model::{AgentFix, Finding};
use std::collections::HashSet;

pub fn build_agent_fix_queue(findings: &[Finding]) -> Vec<AgentFix> {
    let mut seen = HashSet::new();
    let mut ordered = findings.to_vec();
    ordered.sort_by_key(finding_priority);
    let mut queue = vec![];
    for finding in ordered.iter() {
        let key = (finding.path.clone(), finding.agent_fix.clone());
        if !seen.insert(key) {
            continue;
        }
        queue.push(AgentFix {
            path: finding.path.clone(),
            priority: finding.severity.clone(),
            rule_id: finding.rule_id.clone(),
            tlr: finding.tlr.clone(),
            lane: finding.lane.clone(),
            owner: finding.owner.clone(),
            task: finding.agent_fix.clone(),
            why: finding.problem.clone(),
        });
    }
    queue
}

fn finding_priority(f: &Finding) -> (i32, i32, String) {
    let severity = match f.severity.as_str() {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        _ => 3,
    };
    let tlr_priority = match f.tlr.as_deref().unwrap_or("") {
        "Security" => 0,
        "Business truth" => 1,
        "Contracts/data" => 2,
        "Verification" => 3,
        "Repair" => 4,
        "Context/setup" => 5,
        _ => 6,
    };
    (tlr_priority, severity, f.path.clone())
}
