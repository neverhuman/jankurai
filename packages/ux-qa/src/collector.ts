import type { Page } from "playwright";
import type { UxQaElement, UxQaPageMetrics, UxQaViewport } from "./types.js";

const INTERACTIVE_SELECTOR = [
  "button",
  "a[href]",
  "input",
  "select",
  "textarea",
  "summary",
  "[role='button']",
  "[role='link']",
  "[role='menuitem']",
  "[role='tab']",
  "[role='checkbox']",
  "[role='radio']",
  "[tabindex]:not([tabindex='-1'])"
].join(",");

const LAYOUT_SELECTOR = [
  "header",
  "footer",
  "main",
  "nav",
  "section",
  "article",
  "form",
  "[data-ux-qa-region]",
  "[style*='overflow']",
  "[class*='overflow']",
  "[style*='position: fixed']",
  "[style*='position:fixed']",
  "[style*='position: sticky']",
  "[style*='position:sticky']"
].join(",");

type BrowserCollection = {
  viewport: UxQaViewport;
  metrics: UxQaPageMetrics;
  elements: UxQaElement[];
};

type CollectArgs = {
  selector: string;
  interactiveSelector: string;
};

export async function collectViewport(page: Page): Promise<UxQaViewport> {
  return (await collectPageState(page)).viewport;
}

export async function collectPageMetrics(page: Page): Promise<UxQaPageMetrics> {
  return (await collectPageState(page)).metrics;
}

export async function collectUxElements(page: Page): Promise<UxQaElement[]> {
  return (await collectPageState(page)).elements;
}

export async function collectPageState(page: Page): Promise<BrowserCollection> {
  return page.evaluate(collectInBrowser, {
    selector: `${INTERACTIVE_SELECTOR},${LAYOUT_SELECTOR}`,
    interactiveSelector: INTERACTIVE_SELECTOR
  });
}

function collectInBrowser({ selector, interactiveSelector }: CollectArgs): BrowserCollection {
  function isVisible(node: HTMLElement): boolean {
    const style = window.getComputedStyle(node);
    const rect = node.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0 && style.display !== "none" && style.visibility !== "hidden";
  }

  function cssEscape(value: string): string {
    if ("CSS" in window && typeof CSS.escape === "function") return CSS.escape(value);
    return value.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/'/g, "\\'");
  }

  function selectorResolves(selector: string, node: HTMLElement): boolean {
    try {
      const matches = Array.from(document.querySelectorAll(selector));
      return matches.length === 1 && matches[0] === node;
    } catch {
      return false;
    }
  }

  function accessibleName(node: HTMLElement): string {
    const labelledBy = node.getAttribute("aria-labelledby");
    if (labelledBy) {
      const labelText = labelledBy
        .split(/\s+/)
        .map((id) => document.getElementById(id)?.textContent?.trim() ?? "")
        .filter(Boolean)
        .join(" ");
      if (labelText) return labelText;
    }
    return node.getAttribute("aria-label") ?? node.getAttribute("title") ?? (node.textContent ?? "").replace(/\s+/g, " ").trim().slice(0, 80);
  }

  function stableSelector(node: HTMLElement): string {
    const jankuraiId = node.getAttribute("data-jankurai-id");
    if (jankuraiId) {
      const selector = `[data-jankurai-id="${cssEscape(jankuraiId)}"]`;
      if (selectorResolves(selector, node)) return selector;
    }
    const testId = node.getAttribute("data-testid");
    if (testId) {
      const selector = `[data-testid="${cssEscape(testId)}"]`;
      if (selectorResolves(selector, node)) return selector;
    }
    const role = node.getAttribute("role");
    const name = accessibleName(node);
    if (role && name) {
      const selector = `[role="${cssEscape(role)}"][aria-label="${cssEscape(name)}"]`;
      if (selectorResolves(selector, node)) return selector;
    }
    if (node.id) {
      const selector = `#${cssEscape(node.id)}`;
      if (selectorResolves(selector, node)) return selector;
    }
    const stableClass = Array.from(node.classList).find((item) => !/^(css|sc|_)?[a-z0-9_-]{0,3}$/i.test(item));
    if (stableClass) {
      const selector = `${node.tagName.toLowerCase()}.${cssEscape(stableClass)}`;
      if (selectorResolves(selector, node)) return selector;
    }
    return domPath(node);
  }

  function domPath(node: HTMLElement): string {
    const parts: string[] = [];
    let current: HTMLElement | null = node;
    while (current && current !== document.body && current !== document.documentElement) {
      const tag = current.tagName.toLowerCase();
      const siblings = current.parentElement ? Array.from(current.parentElement.children).filter((child) => child.tagName === current?.tagName) : [];
      const index = siblings.indexOf(current) + 1;
      parts.unshift(`${tag}:nth-of-type(${Math.max(1, index)})`);
      current = current.parentElement;
    }
    parts.unshift("body");
    return parts.join(" > ");
  }

  function countTextLines(node: HTMLElement): number {
    const range = document.createRange();
    range.selectNodeContents(node);
    const lineCount = Array.from(range.getClientRects()).filter((rect) => rect.width > 0 && rect.height > 0).length;
    range.detach();
    return Math.max(1, lineCount);
  }

  function hasVisibleFocus(node: HTMLElement): boolean {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const before = window.getComputedStyle(node);
    const beforeBackground = before.backgroundColor;
    const beforeBorder = before.borderColor;
    node.focus({ preventScroll: true });
    if (document.activeElement !== node) return true;

    const focusedStyle = window.getComputedStyle(node);
    const outlineWidth = Number.parseFloat(focusedStyle.outlineWidth || "0");
    const hasOutline = focusedStyle.outlineStyle !== "none" && outlineWidth > 0;
    const hasShadow = focusedStyle.boxShadow !== "none";
    const hasBorderChange = focusedStyle.borderColor !== beforeBorder;
    const hasBackgroundChange = focusedStyle.backgroundColor !== beforeBackground;
    const hasVisibleIndicator = hasOutline || hasShadow || hasBorderChange || hasBackgroundChange;

    previous?.focus({ preventScroll: true });
    if (!previous) node.blur();
    return hasVisibleIndicator;
  }

  function hasFormLabel(node: HTMLElement): boolean {
    const isField = node instanceof HTMLInputElement || node instanceof HTMLSelectElement || node instanceof HTMLTextAreaElement;
    if (!isField || (node instanceof HTMLInputElement && node.type === "hidden")) return true;
    if (node.getAttribute("aria-label") || node.getAttribute("aria-labelledby") || node.getAttribute("title")) return true;
    return (node.labels?.length ?? 0) > 0;
  }

  function ignoredHitTarget(node: Element | null): boolean {
    if (!(node instanceof HTMLElement)) return true;
    const style = window.getComputedStyle(node);
    return style.pointerEvents === "none" || node.hasAttribute("disabled") || node.hasAttribute("inert") || node.closest("[inert]") !== null;
  }

  function hitTest(node: HTMLElement): { hitTargetSelector: string | null; obstructedBy: string | null } {
    const rect = node.getBoundingClientRect();
    const x = Math.min(Math.max(rect.left + rect.width / 2, 0), window.innerWidth - 1);
    const y = Math.min(Math.max(rect.top + rect.height / 2, 0), window.innerHeight - 1);
    const hit = document.elementFromPoint(x, y);
    if (!hit || hit === node || node.contains(hit) || ignoredHitTarget(hit)) {
      return { hitTargetSelector: hit instanceof HTMLElement ? stableSelector(hit) : null, obstructedBy: null };
    }
    const blocker = hit instanceof HTMLElement ? hit : hit.parentElement;
    return {
      hitTargetSelector: blocker ? stableSelector(blocker) : null,
      obstructedBy: blocker ? stableSelector(blocker) : null
    };
  }

  function describeElement(node: HTMLElement, interactive: boolean): UxQaElement {
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    const text = (node.textContent ?? "").replace(/\s+/g, " ").trim().slice(0, 120);
    const selector = stableSelector(node);
    const hit = interactive ? hitTest(node) : { hitTargetSelector: null, obstructedBy: null };
    return {
      selector,
      tag: node.tagName.toLowerCase(),
      role: node.getAttribute("role"),
      interactive,
      name: accessibleName(node),
      text,
      box: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
      lineCount: countTextLines(node),
      scrollWidth: node.scrollWidth,
      scrollHeight: node.scrollHeight,
      clientWidth: node.clientWidth,
      clientHeight: node.clientHeight,
      overflowX: style.overflowX,
      overflowY: style.overflowY,
      position: style.position,
      zIndex: style.zIndex,
      pointerEvents: style.pointerEvents,
      disabled: node.hasAttribute("disabled"),
      inert: node.hasAttribute("inert") || node.closest("[inert]") !== null,
      focusVisible: interactive ? hasVisibleFocus(node) : true,
      labelled: hasFormLabel(node),
      hitTargetSelector: hit.hitTargetSelector,
      obstructedBy: hit.obstructedBy,
      selectorResolved: selectorResolves(selector, node)
    };
  }

  const nodes = Array.from(document.querySelectorAll<HTMLElement>(selector)).filter(isVisible);
  return {
    viewport: { width: window.innerWidth, height: window.innerHeight },
    metrics: {
      scrollWidth: document.documentElement.scrollWidth,
      clientWidth: document.documentElement.clientWidth,
      scrollHeight: document.documentElement.scrollHeight,
      clientHeight: document.documentElement.clientHeight
    },
    elements: nodes.map((node) => describeElement(node, node.matches(interactiveSelector)))
  };
}
