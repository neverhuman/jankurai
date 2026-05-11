import { mkdir, stat, writeFile } from "node:fs/promises";
import { isAbsolute, join } from "node:path";
import { reportArtifactPath, sha256File } from "./receipts.js";
import type {
  UxQaArtifact,
  UxQaBaselineMode,
  UxQaConfig,
  UxQaDecision,
  UxQaReport,
  UxQaRoute,
  UxQaVisualBaselineSummary
} from "./types.js";

export async function evaluateVisualBaseline(args: {
  report: UxQaReport;
  screenshotPath?: string | undefined;
  route?: UxQaRoute | undefined;
  config: UxQaConfig;
  outputRoot: string;
}): Promise<{ summary: UxQaVisualBaselineSummary; artifacts: UxQaArtifact[] }> {
  const { report, screenshotPath, route, config, outputRoot } = args;
  const mode = route?.visualBaselineMode ?? config.visualBaselineMode ?? "pass";
  const owner = route?.baselineOwner ?? config.baselineOwner;
  const approvedBy = route?.baselineApprovedBy ?? config.baselineApprovedBy;
  const approvedAt = route?.baselineApprovedAt ?? config.baselineApprovedAt;
  const approvalNote = route?.baselineApprovalNote ?? config.baselineApprovalNote;
  const actualPath = screenshotPath ? reportArtifactPath(screenshotPath, outputRoot) : undefined;
  const actualSha256 = screenshotPath ? await sha256File(screenshotPath) : undefined;
  const baselineFilePath = resolveBaselineFilePath(report, route, config, outputRoot);
  const configured = Boolean(screenshotPath || baselineFilePath);

  if (!configured) {
    return {
      summary: buildSummary({
        mode,
        status: "not-configured",
        decision: "pass",
        owner,
        approvedBy,
        approvedAt,
        approvalNote
      }),
      artifacts: []
    };
  }

  if (!screenshotPath) {
    return {
      summary: buildSummary({
        mode,
        status: "not-configured",
        decision: "pass",
        baselinePath: baselineFilePath ? reportArtifactPath(baselineFilePath, outputRoot) : undefined,
        owner,
        approvedBy,
        approvedAt,
        approvalNote
      }),
      artifacts: []
    };
  }

  if (!baselineFilePath) {
    return {
      summary: buildSummary({
        mode,
        status: "not-configured",
        decision: "pass",
        actualPath,
        actualSha256,
        owner,
        approvedBy,
        approvedAt,
        approvalNote
      }),
      artifacts: []
    };
  }

  const baselineExists = await isFile(baselineFilePath);
  const baselinePath = reportArtifactPath(baselineFilePath, outputRoot);
  if (!baselineExists) {
    const { diffArtifact, diffPath } = await maybeWriteDiffPlaceholder({
      report,
      screenshotPath,
      actualPath,
      actualSha256,
      baselinePath,
      baselineFilePath,
      config,
      outputRoot,
      mode,
      status: "missing-baseline",
      baselineSha256: undefined,
      owner,
      approvedBy,
      approvedAt,
      approvalNote
    });
    return {
      summary: buildSummary({
        mode,
        status: "missing-baseline",
        decision: mode,
        actualPath,
        baselinePath,
        diffPath,
        actualSha256,
        owner,
        approvedBy,
        approvedAt,
        approvalNote
      }),
      artifacts: diffArtifact ? [diffArtifact] : []
    };
  }

  const baselineSha256 = await sha256File(baselineFilePath);
  const status = baselineSha256 === actualSha256 ? "matched" : "changed";
  const artifacts: UxQaArtifact[] = [visualBaselineArtifact(baselinePath, baselineSha256, report.viewport)];
  let diffPath: string | undefined;
  if (status === "changed") {
    const diff = await maybeWriteDiffPlaceholder({
      report,
      screenshotPath,
      actualPath,
      actualSha256,
      baselinePath,
      baselineFilePath,
      config,
      outputRoot,
      mode,
      status,
      baselineSha256,
      owner,
      approvedBy,
      approvedAt,
      approvalNote
    });
    if (diff.diffArtifact) artifacts.push(diff.diffArtifact);
    diffPath = diff.diffPath;
  }

  return {
    summary: buildSummary({
      mode,
      status,
      decision: status === "matched" ? "pass" : mode,
      actualPath,
      baselinePath,
      diffPath,
      actualSha256,
      baselineSha256,
      owner,
      approvedBy,
      approvedAt,
      approvalNote
    }),
    artifacts
  };
}

function resolveBaselineFilePath(
  report: UxQaReport,
  route: UxQaRoute | undefined,
  config: UxQaConfig,
  outputRoot: string
): string | undefined {
  if (route?.baselinePath) {
    return resolvePath(route.baselinePath, outputRoot);
  }
  const root = route?.baselinePath ? undefined : config.visualBaselineRoot;
  if (!root) return undefined;
  return join(resolvePath(root, outputRoot), `${visualIdentity(report, route)}.png`);
}

function visualIdentity(report: UxQaReport, route?: UxQaRoute): string {
  const base = route?.storyId ?? route?.id ?? report.storyId ?? report.routeId ?? report.url;
  const identity = report.state ? `${base}.${report.state}` : base;
  return safeFileName(`${identity}.${report.viewport.width}x${report.viewport.height}`);
}

function visualBaselineArtifact(path: string, sha256: string, viewport: UxQaReport["viewport"]): UxQaArtifact {
  return { kind: "visual-baseline", path, viewport, sha256 };
}

function buildSummary(args: {
  mode: UxQaBaselineMode;
  status: UxQaVisualBaselineSummary["status"];
  decision: UxQaDecision;
  actualPath?: string | undefined;
  baselinePath?: string | undefined;
  diffPath?: string | undefined;
  actualSha256?: string | undefined;
  baselineSha256?: string | undefined;
  owner?: string | undefined;
  approvedBy?: string | undefined;
  approvedAt?: string | undefined;
  approvalNote?: string | undefined;
}): UxQaVisualBaselineSummary {
  const summary: UxQaVisualBaselineSummary = {
    mode: args.mode,
    status: args.status,
    decision: args.decision
  };
  if (args.actualPath) summary.actualPath = args.actualPath;
  if (args.baselinePath) summary.baselinePath = args.baselinePath;
  if (args.diffPath) summary.diffPath = args.diffPath;
  if (args.actualSha256) summary.actualSha256 = args.actualSha256;
  if (args.baselineSha256) summary.baselineSha256 = args.baselineSha256;
  if (args.owner) summary.owner = args.owner;
  if (args.approvedBy) summary.approvedBy = args.approvedBy;
  if (args.approvedAt) summary.approvedAt = args.approvedAt;
  if (args.approvalNote) summary.approvalNote = args.approvalNote;
  return summary;
}

async function maybeWriteDiffPlaceholder(args: {
  report: UxQaReport;
  screenshotPath: string;
  actualPath?: string | undefined;
  actualSha256?: string | undefined;
  baselinePath: string;
  baselineFilePath: string;
  config: UxQaConfig;
  outputRoot: string;
  mode: UxQaBaselineMode;
  status: Exclude<UxQaVisualBaselineSummary["status"], "matched" | "not-configured">;
  baselineSha256?: string | undefined;
  owner?: string | undefined;
  approvedBy?: string | undefined;
  approvedAt?: string | undefined;
  approvalNote?: string | undefined;
}): Promise<{ diffArtifact?: UxQaArtifact; diffPath?: string }> {
  const diffRoot = args.config.visualDiffRoot;
  if (!diffRoot) return {};
  const resolvedRoot = resolvePath(diffRoot, args.outputRoot);
  await mkdir(resolvedRoot, { recursive: true });
  const diffPath = join(resolvedRoot, `${visualIdentity(args.report)}.json`);
  const payload = {
    mode: args.mode,
    status: args.status,
    decision: args.mode,
    screenshotPath: reportArtifactPath(args.screenshotPath, args.outputRoot),
    actualPath: args.actualPath,
    baselinePath: args.baselinePath,
    actualSha256: args.actualSha256,
    baselineSha256: args.baselineSha256,
    owner: args.owner,
    approvedBy: args.approvedBy,
    approvedAt: args.approvedAt,
    approvalNote: args.approvalNote,
    generatedAt: new Date().toISOString()
  };
  await writeFile(diffPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
  return {
    diffPath: reportArtifactPath(diffPath, args.outputRoot),
    diffArtifact: {
      kind: "visual-diff",
      path: reportArtifactPath(diffPath, args.outputRoot),
      viewport: args.report.viewport,
      sha256: await sha256File(diffPath)
    }
  };
}

async function isFile(path: string): Promise<boolean> {
  const info = await stat(path).catch(() => undefined);
  return info?.isFile() ?? false;
}

function resolvePath(path: string, root: string): string {
  return isAbsolute(path) ? path : join(root, path);
}

function safeFileName(value: string): string {
  return value.replace(/[^a-z0-9._-]+/gi, "-").replace(/^-+|-+$/g, "").slice(0, 120) || "ux-qa";
}
