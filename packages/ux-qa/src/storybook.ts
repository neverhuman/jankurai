import type { UxQaRoute } from "./types.js";

export interface StorybookStory {
  id: string;
  title?: string;
  name?: string;
}

export async function discoverStorybookStories(baseUrl: string): Promise<StorybookStory[]> {
  const root = baseUrl.replace(/\/+$/, "");
  const response = await fetch(`${root}/index.json`);
  if (!response.ok) throw new Error(`failed to read Storybook index: ${response.status}`);
  const payload = await parseStorybookIndex(await response.json());
  const entries = payload.entries ?? payload.stories ?? {};
  return Object.entries(entries).map(([id, story]) => ({ ...story, id: story.id ?? id }));
}

export function storybookIframeUrl(baseUrl: string, storyId: string): string {
  return `${baseUrl.replace(/\/+$/, "")}/iframe.html?id=${encodeURIComponent(storyId)}`;
}

export function resolveStorybookRoutes(
  baseUrl: string,
  stories: StorybookStory[],
  configuredRoutes?: UxQaRoute[]
): UxQaRoute[] {
  if (!configuredRoutes?.length) {
    return stories.map((story) => ({
      id: story.id,
      storyId: story.id,
      url: storybookIframeUrl(baseUrl, story.id)
    }));
  }

  const missingStoryId = configuredRoutes.find((route) => !route.storyId);
  if (missingStoryId) {
    throw new Error(`configured Storybook route ${missingStoryId.id} is missing storyId`);
  }

  const storiesById = new Map(stories.map((story) => [story.id, story]));
  return configuredRoutes.map((route) => {
    const story = storiesById.get(route.storyId!);
    if (!story) {
      throw new Error(`configured Storybook route ${route.id} references missing storyId ${route.storyId}`);
    }
    return {
      ...route,
      storyId: story.id,
      url: storybookIframeUrl(baseUrl, story.id)
    };
  });
}

interface StorybookIndexPayload {
  entries?: Record<string, StorybookStory>;
  stories?: Record<string, StorybookStory>;
}

function parseStorybookIndex(value: unknown): StorybookIndexPayload {
  if (!isRecord(value)) throw new Error("invalid Storybook index: expected an object");
  const payload: StorybookIndexPayload = {};
  if (value.entries !== undefined) payload.entries = parseStorybookStories(value.entries, "entries");
  if (value.stories !== undefined) payload.stories = parseStorybookStories(value.stories, "stories");
  return payload;
}

function parseStorybookStories(value: unknown, field: string): Record<string, StorybookStory> {
  if (!isRecord(value)) throw new Error(`invalid Storybook index: expected ${field} to be an object`);
  const stories: Record<string, StorybookStory> = {};
  for (const [id, item] of Object.entries(value)) {
    stories[id] = parseStorybookStory(item, id, field);
  }
  return stories;
}

function parseStorybookStory(value: unknown, id: string, field: string): StorybookStory {
  if (!isRecord(value)) throw new Error(`invalid Storybook index: expected ${field}.${id} to be an object`);
  const story: StorybookStory = { id };
  if (value.id !== undefined) {
    if (typeof value.id !== "string") throw new Error(`invalid Storybook index: expected ${field}.${id}.id to be a string`);
    story.id = value.id;
  }
  if (value.title !== undefined) {
    if (typeof value.title !== "string") throw new Error(`invalid Storybook index: expected ${field}.${id}.title to be a string`);
    story.title = value.title;
  }
  if (value.name !== undefined) {
    if (typeof value.name !== "string") throw new Error(`invalid Storybook index: expected ${field}.${id}.name to be a string`);
    story.name = value.name;
  }
  return story;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
