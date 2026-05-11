import { expect, test } from "@playwright/test";
import { createHash } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import { evaluateVisualBaseline } from "../src/index.js";
import type { UxQaReport, UxQaRoute } from "../src/index.js";

function minimalReport(): UxQaReport {
  return {
    schemaVersion: "1.4.0",
    toolVersion: "0.4.0",
    url: "http://example.com",
    routeId: "card",
    state: "loading",
    checkedAt: "2026-05-02T12:00:00.000Z",
    viewport: { width: 320, height: 240 },
    metrics: { scrollWidth: 320, clientWidth: 320, scrollHeight: 240, clientHeight: 240 },
    elements: [],
    violations: [],
    artifacts: [],
    summary: { errors: 0, warnings: 0, byRule: {} },
    decision: "pass"
  };
}

function route(mode: "pass" | "review" | "block" = "pass"): UxQaRoute {
  return {
    id: "card",
    url: "http://example.com",
    state: "loading",
    visualBaselineMode: mode,
    baselineOwner: "design"
  };
}

test("visual baseline missing in review mode yields a review decision", async ({}, testInfo) => {
  const screenshotPath = testInfo.outputPath("shot.txt");
  await writeFile(screenshotPath, "actual-bytes", "utf8");

  const result = await evaluateVisualBaseline({
    report: minimalReport(),
    screenshotPath,
    route: route("review"),
    config: {
      outputRoot: testInfo.outputDir,
      visualBaselineRoot: "baselines"
    },
    outputRoot: testInfo.outputDir
  });

  expect(result.summary.status).toBe("missing-baseline");
  expect(result.summary.decision).toBe("review");
  expect(result.summary.actualSha256).toMatch(/^sha256:[0-9a-f]{64}$/);
  expect(result.artifacts).toEqual([]);
});

test("visual baseline changed in block mode yields a block decision", async ({}, testInfo) => {
  const screenshotPath = testInfo.outputPath("shot.txt");
  const baselinePath = testInfo.outputPath("baselines/card.loading.320x240.png");
  const diffRoot = testInfo.outputPath("diffs");
  await writeFile(screenshotPath, "actual-bytes", "utf8");
  await mkdir(dirname(baselinePath), { recursive: true });
  await writeFile(baselinePath, "baseline-bytes", "utf8");

  const result = await evaluateVisualBaseline({
    report: minimalReport(),
    screenshotPath,
    route: route("block"),
    config: {
      outputRoot: testInfo.outputDir,
      visualBaselineRoot: "baselines",
      visualDiffRoot: "diffs"
    },
    outputRoot: testInfo.outputDir
  });

  expect(result.summary.status).toBe("changed");
  expect(result.summary.decision).toBe("block");
  expect(result.summary.diffPath).toContain("diffs");
  expect(result.artifacts.map((item) => item.kind)).toEqual(["visual-baseline", "visual-diff"]);
  expect(result.artifacts.every((item) => /^sha256:[0-9a-f]{64}$/.test(item.sha256 ?? ""))).toBe(true);
  expect(result.summary.baselinePath).toBe("baselines/card.loading.320x240.png");
  expect(result.summary.actualPath).toBe("shot.txt");
});

test("matching visual baseline stays pass", async ({}, testInfo) => {
  const screenshotPath = testInfo.outputPath("shot.txt");
  const baselinePath = testInfo.outputPath("baselines/card.loading.320x240.png");
  await writeFile(screenshotPath, "same-bytes", "utf8");
  await mkdir(dirname(baselinePath), { recursive: true });
  await writeFile(baselinePath, "same-bytes", "utf8");

  const result = await evaluateVisualBaseline({
    report: minimalReport(),
    screenshotPath,
    route: route("review"),
    config: {
      outputRoot: testInfo.outputDir,
      visualBaselineRoot: "baselines"
    },
    outputRoot: testInfo.outputDir
  });

  expect(result.summary.status).toBe("matched");
  expect(result.summary.decision).toBe("pass");
  expect(result.artifacts.map((item) => item.kind)).toEqual(["visual-baseline"]);
  expect(result.artifacts[0]?.sha256).toBe(`sha256:${createHash("sha256").update("same-bytes").digest("hex")}`);
});
