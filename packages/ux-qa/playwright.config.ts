import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests",
  timeout: 20_000,
  use: {
    viewport: { width: 390, height: 844 }
  }
});
