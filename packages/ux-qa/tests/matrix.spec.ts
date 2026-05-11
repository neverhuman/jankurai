import { expect, test } from "@playwright/test";
import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { runCli } from "../src/cli.js";

test("CLI expands route states through a state query parameter", async ({}, testInfo) => {
  const pagePath = testInfo.outputPath("state-fixture.html");
  const configPath = testInfo.outputPath("ux-qa.toml");
  const reportPath = testInfo.outputPath("ux-qa.json");
  await writeFile(pagePath, "<main>State fixture</main>", "utf8");
  await writeFile(configPath, `
outputRoot = "${testInfo.outputDir}"
stateQueryParam = "ux_state"
viewports = ["390x844"]

[[routes]]
id = "dashboard"
url = "${pathToFileURL(pagePath).toString()}"
states = ["loading", "success"]
`, "utf8");

  const exitCode = await runCli([
    "audit",
    "--config",
    configPath,
    "--out",
    reportPath
  ]);

  const payload = JSON.parse(await readFile(reportPath, "utf8"));
  const reports = payload.reports as Array<{ state?: string; url: string; stateCoverage: { declared: string[]; missing: string[] } }>;
  expect(exitCode).toBe(0);
  expect(reports).toHaveLength(2);
  expect(reports.map((report) => report.state)).toEqual(["loading", "success"]);
  expect(reports.map((report) => new URL(report.url).searchParams.get("ux_state"))).toEqual(["loading", "success"]);
  expect(reports.every((report) => report.stateCoverage.declared.join(",") === "loading,success")).toBe(true);
  expect(reports.every((report) => report.stateCoverage.missing.length === 0)).toBe(true);
});
