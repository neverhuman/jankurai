import { readFile } from "node:fs/promises";
import type { UxQaBaselineMode, UxQaConfig, UxQaRoute, UxQaSeverity, UxQaState, UxQaViewport } from "./types.js";

export async function readUxQaConfig(path: string | undefined): Promise<UxQaConfig> {
  if (!path) return {};
  const text = await readFile(path, "utf8");
  if (path.endsWith(".json")) return parseJsonConfig(text);
  return parseTomlSubset(text);
}

function parseJsonConfig(text: string): UxQaConfig {
  const parsed: unknown = JSON.parse(text);
  if (!isRecord(parsed)) throw new Error("invalid ux-qa config: expected a JSON object");

  const config: UxQaConfig = {};
  const artifactRoot = readString(parsed, "artifactRoot");
  if (artifactRoot !== undefined) config.artifactRoot = artifactRoot;
  const visualBaselineRoot = readString(parsed, "visualBaselineRoot");
  if (visualBaselineRoot !== undefined) config.visualBaselineRoot = visualBaselineRoot;
  const visualDiffRoot = readString(parsed, "visualDiffRoot");
  if (visualDiffRoot !== undefined) config.visualDiffRoot = visualDiffRoot;
  const edgeClearancePx = readNumber(parsed, "edgeClearancePx");
  if (edgeClearancePx !== undefined) config.edgeClearancePx = edgeClearancePx;
  const minimumTargetPx = readNumber(parsed, "minimumTargetPx");
  if (minimumTargetPx !== undefined) config.minimumTargetPx = minimumTargetPx;
  const allowButtonWrap = readBoolean(parsed, "allowButtonWrap");
  if (allowButtonWrap !== undefined) config.allowButtonWrap = allowButtonWrap;
  const maximumZIndex = readNumber(parsed, "maximumZIndex");
  if (maximumZIndex !== undefined) config.maximumZIndex = maximumZIndex;
  const allowNestedScrollbars = readBoolean(parsed, "allowNestedScrollbars");
  if (allowNestedScrollbars !== undefined) config.allowNestedScrollbars = allowNestedScrollbars;
  const decisionThreshold = readUnion(parsed, "decisionThreshold", isUxQaSeverity);
  if (decisionThreshold !== undefined) config.decisionThreshold = decisionThreshold;
  const readyState = readUnion(parsed, "readyState", isReadyState);
  if (readyState !== undefined) config.readyState = readyState;
  const timeoutMs = readNumber(parsed, "timeoutMs");
  if (timeoutMs !== undefined) config.timeoutMs = timeoutMs;
  const outputRoot = readString(parsed, "outputRoot");
  if (outputRoot !== undefined) config.outputRoot = outputRoot;
  const storybookUrl = readString(parsed, "storybookUrl");
  if (storybookUrl !== undefined) config.storybookUrl = storybookUrl;
  const baselineOwner = readString(parsed, "baselineOwner");
  if (baselineOwner !== undefined) config.baselineOwner = baselineOwner;
  const baselineApprovedBy = readString(parsed, "baselineApprovedBy");
  if (baselineApprovedBy !== undefined) config.baselineApprovedBy = baselineApprovedBy;
  const baselineApprovedAt = readString(parsed, "baselineApprovedAt");
  if (baselineApprovedAt !== undefined) config.baselineApprovedAt = baselineApprovedAt;
  const baselineApprovalNote = readString(parsed, "baselineApprovalNote");
  if (baselineApprovalNote !== undefined) config.baselineApprovalNote = baselineApprovalNote;
  const requiredStates = readArray(parsed, "requiredStates", isUxQaState);
  if (requiredStates !== undefined) config.requiredStates = requiredStates;
  const visualBaselineMode = readUnion(parsed, "visualBaselineMode", isUxQaBaselineMode);
  if (visualBaselineMode !== undefined) config.visualBaselineMode = visualBaselineMode;
  const stateQueryParam = readString(parsed, "stateQueryParam");
  if (stateQueryParam !== undefined) config.stateQueryParam = stateQueryParam;
  const screenshotRequired = readBoolean(parsed, "screenshotRequired");
  if (screenshotRequired !== undefined) config.screenshotRequired = screenshotRequired;
  const ariaSnapshotRequired = readBoolean(parsed, "ariaSnapshotRequired");
  if (ariaSnapshotRequired !== undefined) config.ariaSnapshotRequired = ariaSnapshotRequired;
  const accessibilityScanRequired = readBoolean(parsed, "accessibilityScanRequired");
  if (accessibilityScanRequired !== undefined) config.accessibilityScanRequired = accessibilityScanRequired;
  const routes = readArray(parsed, "routes", isUxQaRoute);
  if (routes !== undefined) config.routes = routes;
  const viewports = readArray(parsed, "viewports", isUxQaViewportRecord);
  if (viewports !== undefined) config.viewports = viewports;
  return config;
}

function parseTomlSubset(text: string): UxQaConfig {
  const config: UxQaConfig = {};
  const routes: UxQaRoute[] = [];
  const viewports: UxQaViewport[] = [];
  let currentRoute: Partial<UxQaRoute> | null = null;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    if (line === "[[routes]]") {
      if (currentRoute?.id && currentRoute.url) routes.push(currentRoute as UxQaRoute);
      currentRoute = {};
      continue;
    }
    const match = /^([A-Za-z0-9_-]+)\s*=\s*(.+)$/.exec(line);
    if (!match?.[1] || !match[2]) continue;
    const key = match[1];
    const value = parseValue(match[2]);
    if (currentRoute) {
      if (key === "viewport") {
        currentRoute.viewports = [...(currentRoute.viewports ?? []), parseViewport(String(value))];
      } else if (key === "viewports" && Array.isArray(value)) {
        currentRoute.viewports = value.map((item) => parseViewport(String(item)));
      } else {
        (currentRoute as Record<string, unknown>)[key] = value;
      }
    } else {
      if (key === "viewport") {
        viewports.push(parseViewport(String(value)));
      } else if (key === "viewports" && Array.isArray(value)) {
        viewports.push(...value.map((item) => parseViewport(String(item))));
      } else {
        (config as Record<string, unknown>)[key] = value;
      }
    }
  }
  if (currentRoute?.id && currentRoute.url) routes.push(currentRoute as UxQaRoute);
  if (routes.length > 0) config.routes = routes;
  if (viewports.length > 0) config.viewports = viewports;
  return config;
}

function parseValue(value: string): string | number | boolean | Array<string | number | boolean> {
  const trimmed = value.trim();
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (/^\d+$/.test(trimmed)) return Number.parseInt(trimmed, 10);
  if (trimmed.startsWith("[") && trimmed.endsWith("]")) return parseTomlArray(trimmed);
  return trimmed.replace(/^"|"$/g, "");
}

function parseTomlArray(value: string): Array<string | number | boolean> {
  const parsed: unknown = JSON.parse(value.replace(/,\s*]/g, "]"));
  if (!Array.isArray(parsed)) throw new Error(`invalid array value ${value}`);
  return parsed.map((item) => {
    if (typeof item === "string" || typeof item === "number" || typeof item === "boolean") return item;
    throw new Error(`invalid array item in ${value}`);
  });
}

function parseViewport(value: string): UxQaViewport {
  const match = /^(\d+)x(\d+)$/.exec(value);
  if (!match?.[1] || !match[2]) throw new Error(`invalid viewport ${value}`);
  return { width: Number.parseInt(match[1], 10), height: Number.parseInt(match[2], 10) };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isBoolean(value: unknown): value is boolean {
  return typeof value === "boolean";
}

function isUxQaSeverity(value: unknown): value is UxQaSeverity {
  return value === "error" || value === "warning";
}

function isReadyState(value: unknown): value is NonNullable<UxQaConfig["readyState"]> {
  return value === "domcontentloaded" || value === "load" || value === "networkidle";
}

function isUxQaBaselineMode(value: unknown): value is UxQaBaselineMode {
  return value === "pass" || value === "review" || value === "block";
}

function isUxQaState(value: unknown): value is UxQaState {
  return (
    value === "loading" ||
    value === "empty" ||
    value === "error" ||
    value === "success" ||
    value === "permission-denied"
  );
}

function isUxQaViewportRecord(value: unknown): value is UxQaViewport {
  return isRecord(value) && isNumber(value.width) && isNumber(value.height);
}

function isUxQaRoute(value: unknown): value is UxQaRoute {
  if (!isRecord(value) || !isString(value.id) || !isString(value.url)) return false;
  if (value.storyId !== undefined && !isString(value.storyId)) return false;
  if (value.states !== undefined && (!Array.isArray(value.states) || !value.states.every(isUxQaState))) return false;
  if (value.viewports !== undefined && (!Array.isArray(value.viewports) || !value.viewports.every(isUxQaViewportRecord))) return false;
  if (value.state !== undefined && !isUxQaState(value.state)) return false;
  if (value.stateQueryParam !== undefined && !isString(value.stateQueryParam)) return false;
  if (value.visualBaselineMode !== undefined && !isUxQaBaselineMode(value.visualBaselineMode)) return false;
  if (value.baselinePath !== undefined && !isString(value.baselinePath)) return false;
  if (value.baselineOwner !== undefined && !isString(value.baselineOwner)) return false;
  if (value.baselineApprovedBy !== undefined && !isString(value.baselineApprovedBy)) return false;
  if (value.baselineApprovedAt !== undefined && !isString(value.baselineApprovedAt)) return false;
  if (value.baselineApprovalNote !== undefined && !isString(value.baselineApprovalNote)) return false;
  return true;
}

function readString(source: Record<string, unknown>, key: string): string | undefined {
  const value = source[key];
  if (value === undefined) return undefined;
  if (!isString(value)) throw new Error(`invalid ux-qa config: expected ${key} to be a string`);
  return value;
}

function readNumber(source: Record<string, unknown>, key: string): number | undefined {
  const value = source[key];
  if (value === undefined) return undefined;
  if (!isNumber(value)) throw new Error(`invalid ux-qa config: expected ${key} to be a number`);
  return value;
}

function readBoolean(source: Record<string, unknown>, key: string): boolean | undefined {
  const value = source[key];
  if (value === undefined) return undefined;
  if (!isBoolean(value)) throw new Error(`invalid ux-qa config: expected ${key} to be a boolean`);
  return value;
}

function readUnion<T>(source: Record<string, unknown>, key: string, predicate: (value: unknown) => value is T): T | undefined {
  const value = source[key];
  if (value === undefined) return undefined;
  if (!predicate(value)) throw new Error(`invalid ux-qa config: unexpected ${key} value`);
  return value;
}

function readArray<T>(source: Record<string, unknown>, key: string, predicate: (value: unknown) => value is T): T[] | undefined {
  const value = source[key];
  if (value === undefined) return undefined;
  if (!Array.isArray(value) || !value.every(predicate)) {
    throw new Error(`invalid ux-qa config: expected ${key} to be an array of valid values`);
  }
  return value;
}
