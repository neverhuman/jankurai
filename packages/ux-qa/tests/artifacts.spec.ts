import { expect, test } from "@playwright/test";
import { readFile, writeFile } from "node:fs/promises";
import { isAbsolute } from "node:path";
import { pathToFileURL } from "node:url";
import { runCli } from "../src/cli.js";

test("artifact paths are report-relative", async ({}, testInfo) => {
  const pagePath = testInfo.outputPath("fixture.html");
  const reportPath = testInfo.outputPath("ux-qa.json");
  const artifactsDir = testInfo.outputPath("artifacts");
  await writeFile(pagePath, `
    <button data-testid="tiny" style="position:absolute; left:2px; top:2px; width:12px; height:12px">x</button>
    <button data-testid="neighbor" style="position:absolute; left:18px; top:2px; width:12px; height:12px">y</button>
  `, "utf8");

  await runCli([
    "audit",
    "--url",
    pathToFileURL(pagePath).toString(),
    "--out",
    reportPath,
    "--artifacts-dir",
    artifactsDir,
    "--screenshot"
  ]);

  const payload = JSON.parse(await readFile(reportPath, "utf8"));
  const paths = payload.reports.flatMap((report: { artifacts: { path: string }[] }) => report.artifacts.map((item) => item.path));
  expect(paths.length).toBeGreaterThan(0);
  expect(paths.every((item: string) => !isAbsolute(item))).toBe(true);
});

test("policy-required screenshot aria and accessibility artifacts are emitted", async ({}, testInfo) => {
  const pagePath = testInfo.outputPath("a11y-fixture.html");
  const reportPath = testInfo.outputPath("ux-qa.json");
  const artifactsDir = testInfo.outputPath("policy-artifacts");
  const configPath = testInfo.outputPath("ux-qa.toml");
  await writeFile(pagePath, `
    <main>
      <img src="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw==">
      <button>Launch</button>
    </main>
  `, "utf8");
  await writeFile(configPath, `
outputRoot = "${testInfo.outputDir}"
artifactRoot = "${artifactsDir}"
readyState = "load"
timeoutMs = 20000
requiredStates = ["loading", "success"]
screenshotRequired = true
ariaSnapshotRequired = true
accessibilityScanRequired = true
`, "utf8");

  const code = await runCli([
    "audit",
    "--url",
    pathToFileURL(pagePath).toString(),
    "--out",
    reportPath,
    "--config",
    configPath
  ]);

  const payload = JSON.parse(await readFile(reportPath, "utf8"));
  const report = payload.reports[0];
  const kinds = report.artifacts.map((item: { kind: string }) => item.kind);
  expect(code).toBe(1);
  expect(report.schemaVersion).toBe("1.4.0");
  expect(kinds).toEqual(expect.arrayContaining(["screenshot", "aria-snapshot", "accessibility"]));
  expect(report.artifactCoverage.required).toEqual(["screenshot", "aria-snapshot", "accessibility"]);
  expect(report.artifactCoverage.missing).toEqual([]);
  expect(report.accessibility.artifactPath).toMatch(/\.a11y\.json$/);
  expect(report.stateCoverage.missing).toEqual(["loading", "success"]);
  expect(report.decision).toBe("block");
  expect(report.artifacts.every((item: { sha256?: string }) => /^sha256:[0-9a-f]{64}$/.test(item.sha256 ?? ""))).toBe(true);
});
