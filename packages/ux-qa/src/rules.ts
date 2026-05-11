import type { UxQaConfig, UxQaElement, UxQaPageMetrics, UxQaViolation, UxQaViewport } from "./types.js";

const CLIPPING_OVERFLOW = new Set(["hidden", "clip", "auto", "scroll"]);

export function runUxRules(
  elements: UxQaElement[],
  viewport: UxQaViewport,
  metrics: UxQaPageMetrics,
  config: UxQaConfig = {}
): UxQaViolation[] {
  const interactive = elements.filter((element) => element.interactive);
  return [
    ...edgeClearanceViolations(interactive, viewport, config.edgeClearancePx ?? 16),
    ...targetSizeViolations(interactive, config.minimumTargetPx ?? 24),
    ...interactiveOverlapViolations(interactive),
    ...textClippingViolations(elements),
    ...buttonWrapViolations(interactive, config.allowButtonWrap ?? false),
    ...horizontalOverflowViolations(metrics),
    ...stickyObstructionViolations(elements, interactive),
    ...zIndexTokenViolations(elements, config.maximumZIndex ?? 1000),
    ...focusVisibleViolations(interactive),
    ...formLabelViolations(interactive),
    ...nestedScrollbarViolations(elements, config.allowNestedScrollbars ?? false)
  ];
}

function edgeClearanceViolations(elements: UxQaElement[], viewport: UxQaViewport, minimum: number): UxQaViolation[] {
  return elements.flatMap((element) => {
    const gaps = edgeGaps(element, viewport);
    if (Math.min(gaps.left, gaps.right, gaps.top, gaps.bottom) >= minimum) return [];
    return [violation("edge-clearance", element, `interactive element is closer than ${minimum}px to a viewport edge`)];
  });
}

function targetSizeViolations(elements: UxQaElement[], minimum: number): UxQaViolation[] {
  return elements.flatMap((element, index) => {
    if (element.box.width >= minimum && element.box.height >= minimum) return [];
    const crowded = elements.some((other, otherIndex) => otherIndex !== index && circlesIntersect(element, other, minimum));
    if (!crowded) return [];
    return [violation("target-size", element, `target is below ${minimum}x${minimum}px and lacks spacing`)];
  });
}

function interactiveOverlapViolations(elements: UxQaElement[]): UxQaViolation[] {
  const findings: UxQaViolation[] = [];
  for (let left = 0; left < elements.length; left += 1) {
    for (let right = left + 1; right < elements.length; right += 1) {
      const first = elements[left];
      const second = elements[right];
      if (first && second && overlapArea(first, second) > 8) {
        findings.push(violation("interactive-overlap", first, `overlaps ${second.selector}`));
      }
    }
  }
  return findings;
}

function textClippingViolations(elements: UxQaElement[]): UxQaViolation[] {
  return elements.flatMap((element) => {
    const clippedX = element.scrollWidth > element.clientWidth + 1 && CLIPPING_OVERFLOW.has(element.overflowX);
    const clippedY = element.scrollHeight > element.clientHeight + 1 && CLIPPING_OVERFLOW.has(element.overflowY);
    if (!clippedX && !clippedY) return [];
    return [violation("text-clipping", element, "content is clipped inside its rendered box")];
  });
}

function buttonWrapViolations(elements: UxQaElement[], allowed: boolean): UxQaViolation[] {
  if (allowed) return [];
  return elements.flatMap((element) => {
    const isButton = element.tag === "button" || element.role === "button";
    return isButton && element.lineCount > 1 ? [violation("button-wrap", element, "button text wraps onto multiple lines")] : [];
  });
}

function horizontalOverflowViolations(metrics: UxQaPageMetrics): UxQaViolation[] {
  if (metrics.scrollWidth <= metrics.clientWidth + 1) return [];
  return [{
    ruleId: "horizontal-overflow",
    severity: "error",
    message: "document creates horizontal overflow",
    selector: "document",
    evidence: `scrollWidth ${metrics.scrollWidth}px exceeds clientWidth ${metrics.clientWidth}px`
  }];
}

function stickyObstructionViolations(elements: UxQaElement[], interactive: UxQaElement[]): UxQaViolation[] {
  void elements;
  return interactive.flatMap((element) => {
    if (!element.obstructedBy) return [];
    return [violation("sticky-obstruction", element, `center hit-test resolves to ${element.obstructedBy}`)];
  });
}

function zIndexTokenViolations(elements: UxQaElement[], maximum: number): UxQaViolation[] {
  return elements.flatMap((element) => {
    const parsed = Number.parseInt(element.zIndex, 10);
    if (Number.isNaN(parsed) || parsed <= maximum) return [];
    return [violation("z-index-token", element, `z-index ${parsed} exceeds token ceiling ${maximum}`)];
  });
}

function focusVisibleViolations(elements: UxQaElement[]): UxQaViolation[] {
  return elements.flatMap((element) => {
    if (element.focusVisible) return [];
    return [violation("focus-visible", element, "interactive element has no visible focus indicator")];
  });
}

function formLabelViolations(elements: UxQaElement[]): UxQaViolation[] {
  return elements.flatMap((element) => {
    if (element.labelled) return [];
    return [violation("form-label", element, "form control lacks a label or accessible name")];
  });
}

function nestedScrollbarViolations(elements: UxQaElement[], allowed: boolean): UxQaViolation[] {
  if (allowed) return [];
  return elements.flatMap((element) => {
    if (element.tag === "textarea") return [];
    const scrollsX = element.scrollWidth > element.clientWidth + 1 && ["auto", "scroll"].includes(element.overflowX);
    const scrollsY = element.scrollHeight > element.clientHeight + 1 && ["auto", "scroll"].includes(element.overflowY);
    if (!scrollsX && !scrollsY) return [];
    return [warning("nested-scrollbar", element, "element creates a nested scrollbar")];
  });
}

function edgeGaps(element: UxQaElement, viewport: UxQaViewport): Record<"left" | "right" | "top" | "bottom", number> {
  return {
    left: element.box.x,
    right: viewport.width - element.box.x - element.box.width,
    top: element.box.y,
    bottom: viewport.height - element.box.y - element.box.height
  };
}

function circlesIntersect(left: UxQaElement, right: UxQaElement, diameter: number): boolean {
  const dx = centerX(left) - centerX(right);
  const dy = centerY(left) - centerY(right);
  return Math.hypot(dx, dy) < diameter;
}

function overlapArea(left: UxQaElement, right: UxQaElement): number {
  const width = Math.max(0, Math.min(rightEdge(left), rightEdge(right)) - Math.max(left.box.x, right.box.x));
  const height = Math.max(0, Math.min(bottomEdge(left), bottomEdge(right)) - Math.max(left.box.y, right.box.y));
  return width * height;
}

function centerX(element: UxQaElement): number {
  return element.box.x + element.box.width / 2;
}

function centerY(element: UxQaElement): number {
  return element.box.y + element.box.height / 2;
}

function rightEdge(element: UxQaElement): number {
  return element.box.x + element.box.width;
}

function bottomEdge(element: UxQaElement): number {
  return element.box.y + element.box.height;
}

function violation(ruleId: UxQaViolation["ruleId"], element: UxQaElement, evidence: string): UxQaViolation {
  return { ruleId, severity: "error", message: evidence, selector: element.selector, evidence, box: element.box };
}

function warning(ruleId: UxQaViolation["ruleId"], element: UxQaElement, evidence: string): UxQaViolation {
  return { ruleId, severity: "warning", message: evidence, selector: element.selector, evidence, box: element.box };
}
