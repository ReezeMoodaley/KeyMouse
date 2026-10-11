import { describe, it, expect } from "vitest";
import {
  contrast,
  defaultTheme,
  exportTheme,
  importTheme,
  initialThemeState,
  presets,
  themeStyle,
  validateTheme,
  validateThemeState,
} from "./themes";
describe("theme schema and CRT rendering contract", () => {
  it("round trips every preset and preserves recommended defaults", () => {
    for (const theme of presets)
      expect(importTheme(exportTheme(theme))).toEqual(theme);
    expect(defaultTheme.effects).toMatchObject({
      glow: "subtle",
      distortion: true,
      intensity: 20,
      flicker: false,
    });
  });
  it.each(["version", "color", "css", "offset", "type", "field"])(
    "rejects invalid %s imports",
    (kind) => {
      const t = structuredClone(defaultTheme) as unknown as Record<
        string,
        unknown
      >;
      if (kind === "version") t.version = 2;
      if (kind === "color")
        (t.colors as Record<string, string>)["bg-app"] =
          "url(https://example.com)";
      if (kind === "css") t.css = "body{}";
      if (kind === "offset")
        (t.effects as Record<string, unknown>).displacement = 4;
      if (kind === "type")
        (t.effects as Record<string, unknown>).flicker = "true";
      if (kind === "field") delete t.name;
      expect(() => validateTheme(t)).toThrow();
    },
  );
  it("rejects malformed JSON, oversized imports and duplicate/reserved custom IDs", () => {
    expect(() => importTheme("{")).toThrow();
    expect(() => importTheme(" ".repeat(64001))).toThrow();
    const s = initialThemeState();
    s.customs = [defaultTheme];
    expect(() => validateThemeState(s)).toThrow();
    const custom = { ...defaultTheme, id: "custom-one" };
    s.customs = [custom, custom];
    expect(() => validateThemeState(s)).toThrow();
  });
  it.each([0, 20, 40, 70, 100])(
    "bounds shadow offsets at %s percent",
    (intensity) => {
      const s = themeStyle({
        ...defaultTheme,
        effects: {
          ...defaultTheme.effects,
          intensity,
          displacement: 1,
          misalignment: 100,
        },
      }) as Record<string, string>;
      expect(parseFloat(s["--km-crt-ghost-offset"])).toBeCloseTo(
        intensity / 100,
      );
      expect(parseFloat(s["--km-crt-channel-offset"])).toBeCloseTo(
        intensity / 100,
      );
      expect(s["--km-crt-shadow"]).not.toContain("transform");
    },
  );
  it("zero intensity and disabled distortion remove all CRT layers without removing glow", () => {
    for (const effects of [
      { ...defaultTheme.effects, intensity: 0 },
      { ...defaultTheme.effects, distortion: false },
    ]) {
      const s = themeStyle({ ...defaultTheme, effects }) as Record<
        string,
        string
      >;
      expect(s["--km-crt-shadow"]).toBe("0 0 0 transparent");
      expect(s["--km-crt-ghost-offset"]).toBe("0px");
      expect(s["--km-crt-channel-offset"]).toBe("0px");
      expect(s["--km-crt-bloom-radius"]).toBe("0px");
      expect(s["--km-glow-text"]).toContain(
        defaultTheme.colors["accent-primary"],
      );
    }
  });
  it("glow levels scale independently from CRT distortion", () => {
    for (const glow of ["off", "subtle", "standard", "strong"] as const) {
      const s = themeStyle({
        ...defaultTheme,
        effects: { ...defaultTheme.effects, glow },
      }) as Record<string, string>;
      expect(s["--km-glow-text"] === "0 0 0 transparent").toBe(glow === "off");
      expect(s["--km-crt-shadow"]).not.toBe("0 0 0 transparent");
    }
  });
  it("advanced settings change layers and compact windows reduce displacement", () => {
    const clean = themeStyle(defaultTheme),
      altered = themeStyle({
        ...defaultTheme,
        effects: {
          ...defaultTheme.effects,
          ghost: 90,
          displacement: 1,
          misalignment: 90,
          bloom: 90,
        },
      });
    expect(altered["--km-crt-shadow" as keyof typeof altered]).not.toBe(
      clean["--km-crt-shadow" as keyof typeof clean],
    );
    const small = themeStyle(defaultTheme, false, 0.65) as Record<
      string,
      string
    >;
    expect(parseFloat(small["--km-crt-ghost-offset"])).toBeCloseTo(
      0.2 * 0.6 * 0.65,
    );
  });
  it("derives every shadow from theme colors, including neutral monochrome", () => {
    for (const t of presets) {
      const s = themeStyle(t) as Record<string, string>;
      expect(s["--km-glow-text"]).toContain(t.colors["accent-primary"]);
      expect(s["--km-crt-shadow"]).toContain(t.colors["accent-secondary"]);
    }
    const mono = presets[3];
    expect(mono.colors["accent-primary"]).toBe("#EEEEEE");
    expect(mono.colors["accent-secondary"]).toBe("#EEEEEE");
  });
  it("scanlines, noise and flicker work independently of distortion/glow; reduced motion disables flicker", () => {
    const t = {
      ...defaultTheme,
      effects: {
        ...defaultTheme.effects,
        glow: "off" as const,
        distortion: false,
        scanlines: true,
        noise: true,
        flicker: true,
      },
    };
    const s = themeStyle(t) as Record<string, string>;
    expect(s["--km-scanline-display"]).toBe("block");
    expect(s["--km-noise-display"]).toBe("block");
    expect(s["--km-flicker-animation"]).toContain("km-flicker");
    expect(
      (themeStyle(t, true) as Record<string, string>)["--km-flicker-animation"],
    ).toBe("none");
  });
  it("keeps default text contrast readable for all built-in palettes", () => {
    for (const t of presets)
      for (const key of [
        "text-primary",
        "text-secondary",
        "text-muted",
      ] as const)
        expect(
          contrast(t.colors[key], t.colors["bg-panel"]),
        ).toBeGreaterThanOrEqual(4.5);
  });
});
