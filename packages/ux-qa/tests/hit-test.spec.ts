import { expect, test } from "@playwright/test";
import { analyzePage } from "../src/index.js";

test("hit-test reports sticky obstruction", async ({ page }) => {
  await page.setContent(`
    <button data-testid="target" style="position:absolute; left:40px; top:40px; width:120px; height:40px">Target</button>
    <div data-testid="overlay" style="position:fixed; left:30px; top:30px; width:160px; height:80px; background:black"></div>
  `);

  const report = await analyzePage(page);

  expect(report.violations.map((item) => item.ruleId)).toContain("sticky-obstruction");
});

test("hit-test ignores pointer-events-none overlays", async ({ page }) => {
  await page.setContent(`
    <button data-testid="target" style="position:absolute; left:40px; top:40px; width:120px; height:40px">Target</button>
    <div data-testid="overlay" style="pointer-events:none; position:fixed; left:30px; top:30px; width:160px; height:80px; background:black"></div>
  `);

  const report = await analyzePage(page);

  expect(report.violations.map((item) => item.ruleId)).not.toContain("sticky-obstruction");
});
