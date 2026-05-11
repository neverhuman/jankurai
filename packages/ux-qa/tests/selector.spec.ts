import { expect, test } from "@playwright/test";
import { analyzePage, isBroadNthSelector } from "../src/index.js";

test("selector resolves the original element with stable priority", async ({ page }) => {
  await page.setContent(`
    <button data-jankurai-id="save-main">Save</button>
    <button data-testid="cancel-main">Cancel</button>
  `);

  const report = await analyzePage(page);
  const selectors = report.elements.map((element) => element.selector);

  expect(selectors).toContain('[data-jankurai-id="save-main"]');
  expect(selectors).toContain('[data-testid="cancel-main"]');
  expect(report.elements.every((element) => element.selectorResolved)).toBe(true);
  expect(report.elements.some((element) => isBroadNthSelector(element.selector))).toBe(false);
});
