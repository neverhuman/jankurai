use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Sql,
    TypeScript,
    Docker,
    Python,
    Ci,
    Git,
    GitTools,
    Release,
    Comments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidencePolicy {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matcher {
    ContainsAny(&'static [&'static str]),
    ContainsAll(&'static [&'static str]),
    NoActiveDetectors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofWindow {
    NearbySafetyComment,
    NearbySafetyDocs,
    NearbyAsyncContext,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageRule {
    pub id: &'static str,
    pub language: Language,
    pub hlt_rule_id: &'static str,
    pub severity: &'static str,
    pub category: &'static str,
    pub lane: &'static str,
    pub confidence: ConfidencePolicy,
    pub matcher: Matcher,
    pub proof_window: ProofWindow,
    pub problem: &'static str,
    pub fix: &'static str,
}

#[derive(Debug, Clone)]
pub struct LanguageFinding {
    pub rule_id: &'static str,
    pub matched_term: &'static str,
    pub path: String,
    pub line: Option<usize>,
    pub text: String,
    pub problem: String,
    pub reason: String,
    pub agent_fix: String,
    pub evidence: Vec<String>,
}

impl LanguageFinding {
    // Language findings are value objects; constructor arguments map one-to-one to fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rule_id: &'static str,
        matched_term: &'static str,
        path: impl Into<String>,
        line: Option<usize>,
        text: impl Into<String>,
        problem: impl Into<String>,
        reason: impl Into<String>,
        agent_fix: impl Into<String>,
        evidence: Vec<String>,
    ) -> Self {
        Self {
            rule_id,
            matched_term,
            path: path.into(),
            line,
            text: text.into(),
            problem: problem.into(),
            reason: reason.into(),
            agent_fix: agent_fix.into(),
            evidence,
        }
    }
}

pub fn all() -> &'static [LanguageRule] {
    static RULES: Lazy<Vec<LanguageRule>> = Lazy::new(|| {
        let mut rules = Vec::new();
        rules.extend_from_slice(crate::audit::language_rules::rust::catalog());
        rules.extend_from_slice(crate::audit::language_rules::sql::catalog());
        rules.extend_from_slice(crate::audit::language_rules::typescript::catalog());
        rules.extend_from_slice(crate::audit::language_rules::docker::catalog());
        rules.extend_from_slice(crate::audit::language_rules::python::catalog());
        rules.extend_from_slice(crate::audit::language_rules::ci::catalog());
        rules.extend_from_slice(crate::audit::language_rules::git::catalog());
        rules.extend_from_slice(crate::audit::language_rules::gittools::catalog());
        rules.extend_from_slice(crate::audit::language_rules::release::catalog());
        rules.extend_from_slice(crate::audit::language_rules::comments::catalog());
        rules
    });
    RULES.as_slice()
}
