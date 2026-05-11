pub mod ast;
pub mod context;
pub mod contracts;
pub mod data;
pub mod observability;
pub mod ownership;
pub mod proof;
pub mod python;
pub mod security;
pub mod shape;
pub mod speed;
pub mod tool_adoption;
pub mod tuiwright;

use super::helpers::AuditContext;
use crate::model::ProfileStructureReadiness;
use crate::model::*;
use rayon::prelude::*;

pub fn all_dimensions(
    ctx: &AuditContext,
    profile_structure: &ProfileStructureReadiness,
) -> Vec<DimensionResult> {
    let analyzers: [fn(&AuditContext) -> DimensionResult; 10] = [
        contracts::analyze,
        proof::analyze,
        security::analyze,
        shape::analyze,
        data::analyze,
        observability::analyze,
        context::analyze,
        tool_adoption::analyze,
        python::analyze,
        speed::analyze,
    ];
    let mut dimensions = vec![ownership::analyze(ctx, profile_structure)];
    dimensions.extend(
        analyzers
            .par_iter()
            .map(|analyze| analyze(ctx))
            .collect::<Vec<_>>(),
    );
    dimensions
}

pub fn ux_qa_status(ctx: &AuditContext) -> UxQaReadiness {
    use super::helpers::*;

    let mut evidence = serde_json::json!({
        "storybook": paths_with(ctx, &[".storybook/", ".stories.", ".story."], &["@storybook", "storybook", "component story format", "csf"]),
        "playwright_visual": paths_with(ctx, &[], &["tohavescreenshot", "page.screenshot", "locator.screenshot", "visual comparisons", "screenshotpath"]),
        "visual_review": paths_with(ctx, &["backstop", "loki", "argos", "chromatic", "percy", "applitools"], &["@argos-ci", "argos", "chromatic", "percy", "applitools", "backstopjs", "loki", "visual regression", "visual review"]),
        "accessibility": paths_with(ctx, &[], &["@axe-core", "axe-core", "pa11y", "storybook-addon-a11y", "eslint-plugin-jsx-a11y", "accessibility testing", "wcag"]),
        "layout_stability": paths_with(ctx, &[], &["lighthouse", "lhci", "web-vitals", "cumulative layout shift", "layout shift", "cls"]),
        "api_mocks": paths_with(ctx, &[], &["msw", "mock service worker", "msw-storybook-addon", "mockserviceworker", "orval"]),
        "design_tokens": paths_with(ctx, &["tokens/", "design-tokens", "style-dictionary"], &["design tokens", "design-token", "style dictionary", "style-dictionary", "figma variables", "semantic tokens"]),
        "geometry_runtime": paths_with(ctx, &["packages/ux-qa", "ux-qa"], &["@jankurai/ux-qa", "jankurai-ux-qa", "analyzepage", "expectnouxviolations", "edge clearance", "target size", "getboundingclientrect"]),
        "artifact_backed_proof": paths_with(ctx, &["ux-qa-artifacts", "test-results", "playwright-report"], &["--artifacts-dir", "--screenshot", "--aria-snapshot", "artifactpath", "artifactsdir", "ariasnapshot", "tohavescreenshot", "tomatchariasnapshot", "page.screenshot", "trace"]),
    });
    if let Some(tuiwright) = tuiwright::analyze(ctx) {
        evidence
            .as_object_mut()
            .expect("ux evidence object")
            .insert(
                "tuiwright".into(),
                serde_json::to_value(tuiwright).expect("serialize tuiwright evidence"),
            );
    }
    let web_surface = has_web_surface(ctx);
    let missing = if !web_surface {
        vec![]
    } else {
        let mut v = vec![];
        let storybook = evidence
            .get("storybook")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true);
        let playwright = evidence
            .get("playwright_visual")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true);
        let visual = evidence
            .get("visual_review")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true);
        let accessibility = evidence
            .get("accessibility")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true);
        let layout = evidence
            .get("layout_stability")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true);
        let mocks = evidence
            .get("api_mocks")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true);
        let design = evidence
            .get("design_tokens")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true);
        let proof = evidence
            .get("artifact_backed_proof")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true);
        if storybook {
            v.push("Storybook state coverage".into());
        }
        if playwright {
            v.push("Playwright screenshot capture".into());
        }
        if visual {
            v.push("visual review or geometry runtime".into());
        }
        if accessibility {
            v.push("accessibility automation".into());
        }
        if layout {
            v.push("layout stability checks".into());
        }
        if mocks {
            v.push("generated API mocks".into());
        }
        if design {
            v.push("design token discipline".into());
        }
        if proof {
            v.push("artifact-backed UX proof receipts".into());
        }
        v
    };
    UxQaReadiness {
        web_surface,
        has_rendered_ux_lane: !web_surface || missing.is_empty(),
        missing_categories: missing,
        evidence,
        artifact: None,
    }
}
