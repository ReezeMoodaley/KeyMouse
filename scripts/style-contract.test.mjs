// @vitest-environment node
import { readFileSync } from "node:fs";
import { it, expect } from "vitest";
it("decorative effects preserve input geometry and stop hidden/reduced-motion animation", () => {
  const css = readFileSync(
    new URL("../src/terminal.css", import.meta.url),
    "utf8",
  );
  expect(css).toMatch(
    /effects-surface::before,\s*.effects-surface::after[^}]+pointer-events: none/s,
  );
  expect(css).toContain('[data-hidden="true"]');
  expect(css).toContain("animation-play-state: paused");
  expect(css).toContain("prefers-reduced-motion");
  expect(css).not.toMatch(/(?:^|[;{])\s*(?:filter:\s*(?:blur|url)|transform:)/);
  expect(css).toMatch(/calibration-overlay[^}]+animation: none/s);
  expect(css).not.toMatch(/#[0-9a-f]{3,8}\b/i);
});
