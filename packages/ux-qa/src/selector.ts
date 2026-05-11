export const SELECTOR_PRIORITY = [
  "data-jankurai-id",
  "data-testid",
  "role/name",
  "stable-id",
  "stable-class",
  "scoped-dom-path"
] as const;

export function isBroadNthSelector(selector: string): boolean {
  return /^[a-z]+:nth-of-type\(\d+\)$/i.test(selector.trim());
}
