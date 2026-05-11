import { createRequire } from "node:module";
import type { Page } from "playwright";
import type { UxQaAccessibilitySummary } from "./types.js";

const require = createRequire(import.meta.url);
const axeCore = require("axe-core") as { source: string };

export interface AxeRunResult {
  violations?: unknown[];
  incomplete?: unknown[];
  passes?: unknown[];
}

export async function runAccessibilityScan(page: Page): Promise<AxeRunResult> {
  await page.addScriptTag({ content: axeCore.source });
  return await page.evaluate(async () => {
    const win = window as typeof window & {
      axe?: {
        run: (context?: unknown, options?: unknown) => Promise<unknown>;
      };
    };
    if (!win.axe) throw new Error("axe-core did not load");
    return await win.axe.run(document);
  }) as AxeRunResult;
}

export function summarizeAccessibility(
  result: AxeRunResult,
  artifactPath: string | undefined
): UxQaAccessibilitySummary {
  const summary = {
    violations: result.violations?.length ?? 0,
    incomplete: result.incomplete?.length ?? 0,
    passes: result.passes?.length ?? 0
  };
  return artifactPath ? { ...summary, artifactPath } : summary;
}
