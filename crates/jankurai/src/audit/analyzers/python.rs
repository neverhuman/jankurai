use crate::audit::helpers::*;
use crate::model::DimensionResult;

pub fn analyze(ctx: &AuditContext) -> DimensionResult {
    let python_files: Vec<_> = ctx
        .scope_files
        .iter()
        .filter(|f| {
            f.suffix == ".py"
                && !f.rel_path.contains("/tests/fixtures/")
                && !f.rel_path.starts_with("tests/fixtures/")
                && !python_scoring_exempt(ctx, &f.rel_path)
        })
        .cloned()
        .collect();
    let non_optimal = non_optimal_language_hits(ctx);
    if python_files.is_empty() {
        let mut score = 100;
        let mut evidence = vec!["no Python files in scope".into()];
        let mut notes = vec![];
        if !non_optimal.is_empty() {
            score -= 10;
            evidence.push("non-optimal product language marker".into());
            notes.push("runtime code should converge to Rust, TypeScript, SQL, generated contracts, and rare advanced-ML/data Python exceptions".into());
        }
        return make_dim(
            "Python containment and polyglot hygiene",
            score,
            evidence,
            notes,
        );
    }
    if all_scope_python_files_are_accepted_boundaries(ctx) {
        return make_dim(
            "Python containment and polyglot hygiene",
            100,
            vec!["scope Python files are covered by passed audited runtime boundaries".into()],
            vec![],
        );
    }
    let mut score = 40;
    let mut evidence = vec![];
    let mut notes = vec![];
    let bad_paths: Vec<_> = bad_python_path_hits(ctx)
        .into_iter()
        .map(|f| f.rel_path)
        .collect();
    if bad_paths.is_empty() {
        score += 30;
        evidence.push("Python stays inside exception-only non-product roots".into());
    } else {
        score -= 30;
        evidence.push(format!(
            "Python appears outside the allowed roots: {}",
            bad_paths[0]
        ));
        notes.push("Python leaks into product surface".into());
    }
    if python_files
        .iter()
        .any(|f| f.rel_path.starts_with("python/ai-service"))
    {
        score += 10;
        evidence.push("exception-only AI/data service path present".into());
    }
    let ratio = python_ratio(ctx);
    if ratio > 0.3 {
        score -= 35;
        evidence.push(format!(
            "Python is {:.0}% of runtime product code",
            ratio * 100.0
        ));
        notes.push("too much Python for the selected optimal stack".into());
    } else if ratio > 0.15 {
        score -= 15;
        evidence.push(format!(
            "Python is {:.0}% of runtime product code",
            ratio * 100.0
        ));
    }
    if !non_optimal.is_empty() {
        score -= 10;
        evidence.push(format!(
            "non-optimal product language marker: {}",
            non_optimal[0].rel_path
        ));
        notes.push(
            "runtime code should converge to Rust, TypeScript, SQL, generated contracts, and rare advanced-ML/data Python exceptions"
                .into(),
        );
    }
    if bad_paths
        .iter()
        .any(|p| p.contains("psycopg") || p.contains("sqlalchemy"))
    {
        score -= 20;
        evidence.push("Python directly touches DB truth outside AI service".into());
    }
    make_dim(
        "Python containment and polyglot hygiene",
        score,
        evidence,
        notes,
    )
}
