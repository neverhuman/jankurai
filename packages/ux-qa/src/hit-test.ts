import type { UxQaElement } from "./types.js";

export function hitTestObstructed(element: Pick<UxQaElement, "obstructedBy">): boolean {
  return element.obstructedBy !== null && element.obstructedBy !== "";
}
