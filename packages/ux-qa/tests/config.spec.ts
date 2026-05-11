import { expect, test } from "@playwright/test";
import { writeFile } from "node:fs/promises";
import { readUxQaConfig } from "../src/index.js";

test("config parses route matrix and viewports", async ({}, testInfo) => {
  const path = testInfo.outputPath("ux-qa.toml");
  await writeFile(path, `
artifactRoot = "target/jankurai/ux-qa"
storybookUrl = "http://localhost:6006"
requiredStates = ["loading", "success"]
viewports = ["390x844", "1440x900"]
readyState = "networkidle"
timeoutMs = 45000
screenshotRequired = true
ariaSnapshotRequired = true
accessibilityScanRequired = true

[[routes]]
id = "dashboard"
url = "http://localhost:3000/dashboard"
states = ["loading", "success"]
viewports = ["390x844"]
`, "utf8");

  const config = await readUxQaConfig(path);

  expect(config.artifactRoot).toBe("target/jankurai/ux-qa");
  expect(config.storybookUrl).toBe("http://localhost:6006");
  expect(config.requiredStates).toEqual(["loading", "success"]);
  expect(config.viewports).toEqual([{ width: 390, height: 844 }, { width: 1440, height: 900 }]);
  expect(config.readyState).toBe("networkidle");
  expect(config.timeoutMs).toBe(45000);
  expect(config.screenshotRequired).toBe(true);
  expect(config.ariaSnapshotRequired).toBe(true);
  expect(config.accessibilityScanRequired).toBe(true);
  expect(config.routes?.[0]).toEqual({
    id: "dashboard",
    url: "http://localhost:3000/dashboard",
    states: ["loading", "success"],
    viewports: [{ width: 390, height: 844 }]
  });
});

test("config rejects malformed JSON shapes", async ({}, testInfo) => {
  const path = testInfo.outputPath("ux-qa.json");
  await writeFile(path, JSON.stringify({
    artifactRoot: "target/jankurai/ux-qa",
    requiredStates: ["loading", "bogus"]
  }), "utf8");

  await expect(readUxQaConfig(path)).rejects.toThrow(/invalid ux-qa config/i);
});
