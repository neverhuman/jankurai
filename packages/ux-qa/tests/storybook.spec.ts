import { expect, test } from "@playwright/test";
import { discoverStorybookStories, resolveStorybookRoutes, storybookIframeUrl } from "../src/index.js";

test("Storybook index discovery supports index.json", async () => {
  const originalFetch = globalThis.fetch;
  const mockFetch: typeof fetch = async () => new Response(JSON.stringify({
    entries: {
      "button--primary": { id: "button--primary", title: "Button", name: "Primary" }
    }
  }), { status: 200 });
  globalThis.fetch = mockFetch;
  try {
    const stories = await discoverStorybookStories("http://localhost:6006/");
    expect(stories).toEqual([{ id: "button--primary", title: "Button", name: "Primary" }]);
    expect(storybookIframeUrl("http://localhost:6006/", "button--primary")).toBe("http://localhost:6006/iframe.html?id=button--primary");
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("Storybook index discovery rejects malformed payloads", async () => {
  const originalFetch = globalThis.fetch;
  const mockFetch: typeof fetch = async () => new Response(JSON.stringify({
    entries: {
      "button--primary": { id: 123, title: "Button", name: "Primary" }
    }
  }), { status: 200 });
  globalThis.fetch = mockFetch;
  try {
    await expect(discoverStorybookStories("http://localhost:6006/")).rejects.toThrow(/invalid Storybook index/i);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("configured storyId metadata is merged onto discovered Storybook routes", () => {
  const routes = resolveStorybookRoutes(
    "http://localhost:6006",
    [{ id: "button--primary", title: "Button", name: "Primary" }],
    [{
      id: "button-primary",
      url: "http://ignored",
      storyId: "button--primary",
      states: ["loading", "success"],
      viewports: [{ width: 390, height: 844 }],
      visualBaselineMode: "review",
      baselinePath: "target/jankurai/ux-qa-baselines/button.png",
      baselineOwner: "design",
      baselineApprovedBy: "ux",
      stateQueryParam: "ux_state"
    }]
  );

  expect(routes).toEqual([{
    id: "button-primary",
    url: "http://localhost:6006/iframe.html?id=button--primary",
    storyId: "button--primary",
    states: ["loading", "success"],
    viewports: [{ width: 390, height: 844 }],
    visualBaselineMode: "review",
    baselinePath: "target/jankurai/ux-qa-baselines/button.png",
    baselineOwner: "design",
    baselineApprovedBy: "ux",
    stateQueryParam: "ux_state"
  }]);
});
