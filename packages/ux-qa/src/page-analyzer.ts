import type { Page } from "playwright";
import { collectPageState } from "./collector.js";
import { UxQaAssertionError } from "./errors.js";
import { runUxRules } from "./rules.js";
import type { UxQaConfig, UxQaDecision, UxQaReport, UxQaRuleId, UxQaRunContext, UxQaSummary } from "./types.js";

export const UX_QA_SCHEMA_VERSION = "1.4.0";
export const UX_QA_TOOL_VERSION = "0.4.0";

export async function analyzePage(page: Page, config: UxQaConfig = {}, context: UxQaRunContext = {}): Promise<UxQaReport> {
  const { viewport, metrics, elements } = await collectPageState(page);
  const violations = runUxRules(elements, viewport, metrics, config);
  const requiredStates = context.requiredStates ?? config.requiredStates ?? [];
  const declaredStates = context.declaredStates ?? [];
  const report: UxQaReport = {
    schemaVersion: UX_QA_SCHEMA_VERSION,
    toolVersion: UX_QA_TOOL_VERSION,
    url: page.url(),
    checkedAt: new Date().toISOString(),
    viewport,
    metrics,
    elements,
    violations,
    artifacts: [],
    artifactCoverage: {
      required: [],
      present: [],
      missing: []
    },
    summary: summarizeViolations(violations),
    stateCoverage: {
      required: requiredStates,
      declared: declaredStates,
      missing: requiredStates.filter((state) => !declaredStates.includes(state))
    },
    decision: decide(violations, config.decisionThreshold ?? "error")
  };
  if (context.routeId) report.routeId = context.routeId;
  if (context.storyId) report.storyId = context.storyId;
  if (context.state) report.state = context.state;
  if (context.browserName) report.browserName = context.browserName;
  return report;
}

export async function expectNoUxViolations(page: Page, config: UxQaConfig = {}): Promise<void> {
  const report = await analyzePage(page, config);
  if (report.violations.length === 0) return;
  const summary = report.violations.map((item) => `${item.ruleId} ${item.selector}: ${item.evidence}`).join("\n");
  throw new UxQaAssertionError(`jankurai UX QA found ${report.violations.length} violation(s)\n${summary}`);
}

function summarizeViolations(violations: UxQaReport["violations"]): UxQaSummary {
  const byRule: Partial<Record<UxQaRuleId, number>> = {};
  for (const violation of violations) {
    byRule[violation.ruleId] = (byRule[violation.ruleId] ?? 0) + 1;
  }
  return {
    errors: violations.filter((item) => item.severity === "error").length,
    warnings: violations.filter((item) => item.severity === "warning").length,
    byRule
  };
}

function decide(violations: UxQaReport["violations"], threshold: "error" | "warning"): UxQaDecision {
  if (violations.length === 0) return "pass";
  if (threshold === "warning") return "block";
  return violations.some((item) => item.severity === "error") ? "block" : "warn";
}
