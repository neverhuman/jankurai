use crate::{ChangedSurface, ObligationSummary, ProofBindObligations, SurfaceSummary};

pub(crate) fn surface_summary(surfaces: &[ChangedSurface]) -> SurfaceSummary {
    let mut summary = SurfaceSummary {
        changed_surface_count: surfaces.len(),
        high_or_critical_surface_count: surfaces
            .iter()
            .filter(|surface| matches!(surface.severity.as_str(), "high" | "critical"))
            .count(),
        by_surface_type: std::collections::BTreeMap::new(),
        by_owner: std::collections::BTreeMap::new(),
        verdict: "pass".into(),
    };
    for surface in surfaces {
        *summary
            .by_surface_type
            .entry(surface.surface_type.clone())
            .or_default() += 1;
        *summary.by_owner.entry(surface.owner.clone()).or_default() += 1;
    }
    if summary.high_or_critical_surface_count > 0 {
        summary.verdict = "review".into();
    }
    summary
}

pub(crate) fn obligation_summary(
    surfaces: &[ChangedSurface],
    obligations: &[crate::ProofObligation],
    mode: crate::ProofBindMode,
) -> ObligationSummary {
    let satisfied = obligations
        .iter()
        .filter(|obligation| obligation.satisfied)
        .count();
    let missing = obligations.len().saturating_sub(satisfied);
    let high_or_critical_missing = obligations
        .iter()
        .filter(|obligation| {
            !obligation.satisfied && matches!(obligation.severity.as_str(), "high" | "critical")
        })
        .count();
    let verdict = if missing == 0 {
        "pass"
    } else if mode == crate::ProofBindMode::Required && high_or_critical_missing > 0 {
        "block"
    } else {
        "review"
    };
    ObligationSummary {
        total: obligations.len(),
        satisfied,
        missing,
        high_or_critical_missing,
        changed_surface_count: surfaces.len(),
        verdict: verdict.into(),
    }
}

pub fn render_markdown(
    witness: &crate::SurfaceWitness,
    obligations: &ProofBindObligations,
) -> String {
    let mut out = String::new();
    out.push_str("# jankurai ProofBind\n\n");
    out.push_str(&format!("- mode: `{}`\n", witness.mode));
    out.push_str(&format!(
        "- changed surfaces: `{}`\n",
        witness.summary.changed_surface_count
    ));
    out.push_str(&format!(
        "- high/critical surfaces: `{}`\n",
        witness.summary.high_or_critical_surface_count
    ));
    out.push_str(&format!(
        "- obligations: total=`{}` satisfied=`{}` missing=`{}` high_or_critical_missing=`{}` verdict=`{}`\n",
        obligations.summary.total,
        obligations.summary.satisfied,
        obligations.summary.missing,
        obligations.summary.high_or_critical_missing,
        obligations.summary.verdict
    ));
    out.push_str("\n## Surfaces\n");
    if witness.surfaces.is_empty() {
        out.push_str("- none\n");
    } else {
        for surface in &witness.surfaces {
            out.push_str(&format!(
                "- `{}` type=`{}` severity=`{}` owner=`{}` rules=`{}` lanes=`{}`\n",
                surface.path,
                surface.surface_type,
                surface.severity,
                surface.owner,
                surface.required_rules.join(","),
                surface.required_lanes.join(",")
            ));
        }
    }
    out.push_str("\n## Missing Obligations\n");
    let mut any = false;
    for obligation in obligations
        .obligations
        .iter()
        .filter(|obligation| !obligation.satisfied)
    {
        any = true;
        out.push_str(&format!(
            "- `{}` `{}` severity=`{}` repair=`{}`\n",
            obligation.path, obligation.surface_type, obligation.severity, obligation.repair_task
        ));
    }
    if !any {
        out.push_str("- none\n");
    }
    out
}
