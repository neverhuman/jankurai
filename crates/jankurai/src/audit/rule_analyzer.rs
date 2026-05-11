use super::finding_builder::FindingBuilder;
use super::helpers::AuditContext;

pub struct FindingDraft {
    pub rule_id: &'static str,
    pub path: String,
    pub problem: String,
    pub fix: String,
    pub evidence: Vec<String>,
    pub line: Option<usize>,
    pub matched_term: Option<String>,
    pub reason: Option<String>,
}

pub trait RuleAnalyzer {
    fn name(&self) -> &'static str;
    fn rules(&self) -> &'static [&'static str];
    fn analyze(&self, ctx: &AuditContext, findings: &mut FindingBuilder);
}
