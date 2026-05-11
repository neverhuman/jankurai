import { test, expect } from "@playwright/test";
import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { runCli } from "../src/cli.js";
import { analyzePage } from "../src/index.js";

test("reports edge and target-size violations", async ({ page }) => {
  await page.setContent(`
    <button data-testid="tiny" style="position:absolute; left:2px; top:2px; width:12px; height:12px">x</button>
    <button data-testid="neighbor" style="position:absolute; left:18px; top:2px; width:12px; height:12px">y</button>
  `);

  const report = await analyzePage(page);
  const rules = report.violations.map((item) => item.ruleId);

  expect(rules).toContain("edge-clearance");
  expect(rules).toContain("target-size");
});

test("reports clipping, wrapping, and overflow", async ({ page }) => {
  await page.setContent(`
    <style>body { margin: 0; } .wide { width: 800px; height: 1px; }</style>
    <button data-testid="wrapped" style="width:42px; white-space:normal">Long action label</button>
    <button data-testid="clipped" style="display:block; width:40px; height:18px; overflow:hidden">Clipped label</button>
    <div class="wide"></div>
  `);

  const report = await analyzePage(page);
  const rules = report.violations.map((item) => item.ruleId);

  expect(rules).toContain("button-wrap");
  expect(rules).toContain("text-clipping");
  expect(rules).toContain("horizontal-overflow");
});

test("reports overlapping interactive elements", async ({ page }) => {
  await page.setContent(`
    <button data-testid="first" style="position:absolute; left:40px; top:40px; width:80px; height:40px">First</button>
    <a data-testid="second" href="#" style="position:absolute; left:60px; top:50px; width:80px; height:40px; display:block">Second</a>
  `);

  const report = await analyzePage(page);

  expect(report.violations.map((item) => item.ruleId)).toContain("interactive-overlap");
});

test("reports focus, form label, and nested scrollbar issues", async ({ page }) => {
  await page.setContent(`
    <button data-testid="no-focus" style="outline:none; box-shadow:none">No focus ring</button>
    <input data-testid="unlabelled" style="display:block; margin-top:16px" />
    <section data-testid="scroller" style="width:80px; height:32px; overflow:auto">
      <div style="width:240px; height:80px">Scrollable content</div>
    </section>
  `);

  const report = await analyzePage(page);
  const rules = report.violations.map((item) => item.ruleId);

  expect(rules).toContain("focus-visible");
  expect(rules).toContain("form-label");
  expect(rules).toContain("nested-scrollbar");
});

test("CLI emits artifact-backed UX proof receipts", async ({}, testInfo) => {
  const pagePath = testInfo.outputPath("fixture.html");
  const reportPath = testInfo.outputPath("ux-qa.json");
  const artifactsDir = testInfo.outputPath("artifacts");
  await writeFile(pagePath, `
    <button data-testid="tiny" style="position:absolute; left:2px; top:2px; width:12px; height:12px">x</button>
    <button data-testid="neighbor" style="position:absolute; left:18px; top:2px; width:12px; height:12px">y</button>
  `, "utf8");

  const exitCode = await runCli([
    "audit",
    "--url",
    pathToFileURL(pagePath).toString(),
    "--out",
    reportPath,
    "--route-id",
    "fixture/tiny-controls",
    "--artifacts-dir",
    artifactsDir,
    "--screenshot",
    "--aria-snapshot",
    "--wait-for",
    "load",
    "--timeout-ms",
    "20000"
  ]);

  const payload = JSON.parse(await readFile(reportPath, "utf8"));
  const report = payload.reports[0];
  expect(exitCode).toBe(1);
  expect(report.schemaVersion).toBe("1.4.0");
  expect(report.toolVersion).toBe("0.4.0");
  expect(report.decision).toBe("block");
  expect(report.routeId).toBe("fixture/tiny-controls");
  expect(report.artifacts.map((item: { kind: string }) => item.kind)).toEqual(expect.arrayContaining(["screenshot", "aria-snapshot", "crop"]));
  expect(report.artifactCoverage.required).toEqual([]);
  expect(report.violations.some((item: { artifactPath?: string }) => item.artifactPath)).toBe(true);
});

test("CLI records state matrix coverage in reports", async ({}, testInfo) => {
  const pagePath = testInfo.outputPath("fixture.html");
  const configPath = testInfo.outputPath("ux-qa.toml");
  const reportPath = testInfo.outputPath("ux-qa.json");
  await writeFile(pagePath, `<main><p>Go</p></main>`, "utf8");
  await writeFile(configPath, `
requiredStates = ["loading", "success"]

[[routes]]
id = "dashboard"
url = "${pathToFileURL(pagePath).toString()}"
states = ["success"]
`, "utf8");

  const exitCode = await runCli([
    "audit",
    "--config",
    configPath,
    "--out",
    reportPath
  ]);

  const payload = JSON.parse(await readFile(reportPath, "utf8"));
  const report = payload.reports[0];
  expect(exitCode).toBe(1);
  expect(report.decision).toBe("block");
  expect(report.stateCoverage).toEqual({
    required: ["loading", "success"],
    declared: ["success"],
    missing: ["loading"]
  });
});
