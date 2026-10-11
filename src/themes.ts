import base from "./theme-default.json";
import type { CSSProperties } from "react";

export type ColorToken = keyof typeof base.colors;
export interface Effects {
  glow: "off" | "subtle" | "standard" | "strong";
  distortion: boolean;
  intensity: number;
  ghost: number;
  displacement: number;
  misalignment: number;
  bloom: number;
  scanlines: boolean;
  noise: boolean;
  flicker: boolean;
}
export interface Theme {
  version: 1;
  id: string;
  name: string;
  colors: Record<ColorToken, string>;
  radius: number;
  density: "compact" | "comfortable" | "spacious";
  fontSize: number;
  effects: Effects;
}
export interface ThemeState {
  version: 1;
  revision: number;
  active: Theme;
  customs: Theme[];
}
export const defaultTheme = base as Theme;
const preset = (
  id: string,
  name: string,
  colors: Partial<Theme["colors"]>,
): Theme => ({
  ...structuredClone(defaultTheme),
  id,
  name,
  colors: { ...base.colors, ...colors },
});
export const presets: readonly Theme[] = [
  defaultTheme,
  preset("amber-terminal", "Amber Terminal", {
    "bg-app": "#050301",
    "bg-sidebar": "#090601",
    "bg-panel": "#120D05",
    "bg-panel-raised": "#1B1409",
    "text-primary": "#FFEBCD",
    "text-secondary": "#CDBA9B",
    "text-muted": "#B39E80",
    "accent-primary": "#FFC470",
    "accent-secondary": "#E5AA60",
    "accent-tertiary": "#FFE3AA",
    "border-subtle": "#3A2B17",
    "border-strong": "#705332",
    "border-active": "#FFC470",
    "control-bg": "#1B1409",
    "control-hover": "#3A2B17",
    "control-on": "#FFC470",
    "control-text": "#FFEBCD",
    "control-focus": "#FFE3AA",
    "status-success": "#FFC470",
  }),
  preset("cyber-cyan", "Cyber Cyan", {
    "bg-app": "#010405",
    "bg-sidebar": "#03090B",
    "bg-panel": "#061115",
    "bg-panel-raised": "#0B1C22",
    "text-primary": "#DEF6FA",
    "text-secondary": "#A6C4CB",
    "text-muted": "#849FA8",
    "accent-primary": "#78DEEE",
    "accent-secondary": "#B8A8EF",
    "accent-tertiary": "#9AF2CF",
    "border-subtle": "#1D3942",
    "border-strong": "#36616E",
    "border-active": "#78DEEE",
    "control-bg": "#0B1C22",
    "control-hover": "#1D3942",
    "control-on": "#78DEEE",
    "control-text": "#DEF6FA",
    "control-focus": "#9AF2CF",
    "status-success": "#9AF2CF",
  }),
  preset(
    "monochrome",
    "Monochrome",
    Object.fromEntries(
      Object.keys(base.colors).map((k) => [
        k,
        k.startsWith("bg-")
          ? (
              {
                "bg-app": "#030303",
                "bg-sidebar": "#080808",
                "bg-panel": "#101010",
                "bg-panel-raised": "#191919",
              } as Record<string, string>
            )[k]
          : k === "control-bg"
            ? "#191919"
            : k === "control-hover" || k === "border-subtle"
              ? "#333333"
              : k === "border-strong"
                ? "#555555"
                : k === "text-muted"
                  ? "#A0A0A0"
                  : k === "text-secondary"
                    ? "#BDBDBD"
                    : "#EEEEEE",
      ]),
    ) as Theme["colors"],
  ),
];
for (const theme of presets) {
  Object.freeze(theme.colors);
  Object.freeze(theme.effects);
  Object.freeze(theme);
}
Object.freeze(presets);
export const initialThemeState = (): ThemeState => ({
  version: 1,
  revision: 0,
  active: structuredClone(defaultTheme),
  customs: [],
});
const keys = (v: unknown, expected: readonly string[]) =>
  v &&
  typeof v === "object" &&
  !Array.isArray(v) &&
  Object.keys(v).length === expected.length &&
  expected.every((k) => Object.hasOwn(v, k));
const range = (v: unknown, low: number, high: number) =>
  typeof v === "number" && Number.isFinite(v) && v >= low && v <= high;
export function validateTheme(v: unknown): Theme {
  if (
    !keys(v, [
      "version",
      "id",
      "name",
      "colors",
      "radius",
      "density",
      "fontSize",
      "effects",
    ])
  )
    throw Error("Theme must contain exactly the supported versioned fields.");
  const t = v as Theme;
  if (
    t.version !== 1 ||
    typeof t.id !== "string" ||
    !/^[a-zA-Z0-9_-]{1,80}$/.test(t.id) ||
    typeof t.name !== "string" ||
    !t.name.trim() ||
    t.name.length > 80
  )
    throw Error("Invalid theme version, identifier or name.");
  if (
    !keys(t.colors, Object.keys(base.colors)) ||
    !Object.values(t.colors).every(
      (c) => typeof c === "string" && /^#[0-9a-fA-F]{6}$/.test(c),
    )
  )
    throw Error(
      "Colors must be six-digit hexadecimal values; CSS is not accepted.",
    );
  if (
    !range(t.radius, 0, 16) ||
    !range(t.fontSize, 12, 18) ||
    !["compact", "comfortable", "spacious"].includes(t.density)
  )
    throw Error("Invalid geometry or typography.");
  const e = t.effects;
  if (
    !keys(e, Object.keys(base.effects)) ||
    !["off", "subtle", "standard", "strong"].includes(e.glow) ||
    !["distortion", "scanlines", "noise", "flicker"].every(
      (k) => typeof e[k as keyof Effects] === "boolean",
    ) ||
    !["intensity", "ghost", "misalignment", "bloom"].every((k) =>
      range(e[k as keyof Effects], 0, 100),
    ) ||
    !range(e.displacement, 0, 1)
  )
    throw Error("Invalid CRT parameters.");
  return structuredClone(t);
}
export function validateThemeState(v: unknown): ThemeState {
  if (!keys(v, ["version", "revision", "active", "customs"]))
    throw Error("Invalid theme store.");
  const s = v as ThemeState;
  if (
    s.version !== 1 ||
    !Number.isSafeInteger(s.revision) ||
    s.revision < 0 ||
    !Array.isArray(s.customs) ||
    s.customs.length > 100
  )
    throw Error("Invalid theme store version or size.");
  const customs = s.customs.map(validateTheme);
  if (
    new Set(customs.map((t) => t.id)).size !== customs.length ||
    customs.some((t) => presets.some((p) => p.id === t.id))
  )
    throw Error(
      "Custom theme identifiers must be unique and cannot replace presets.",
    );
  return { ...s, active: validateTheme(s.active), customs };
}
export function importTheme(json: string) {
  if (json.length > 64000) throw Error("Theme file is too large.");
  return validateTheme(JSON.parse(json));
}
export const exportTheme = (theme: Theme) =>
  JSON.stringify(validateTheme(theme), null, 2);
export function themeStyle(
  t: Theme,
  reducedMotion = false,
  scale = 1,
): CSSProperties {
  const css: Record<string, string | number> = {};
  for (const [k, v] of Object.entries(t.colors)) css[`--km-${k}`] = v;
  const e = t.effects,
    strength = e.distortion ? e.intensity / 100 : 0;
  const glow = { off: 0, subtle: 0.18, standard: 0.35, strong: 0.55 }[e.glow];
  const tint = (color: string, opacity: number) =>
    `color-mix(in srgb, ${color} ${Math.round(opacity * 10000) / 100}%, transparent)`;
  const accent = t.colors["accent-primary"];
  const halo = glow
    ? `0 0 2px ${tint(accent, glow)}, 0 0 7px ${tint(accent, glow * 0.45)}`
    : "0 0 0 transparent";
  // Shadows sit behind opaque glyphs. Give the displaced edges enough opacity
  // to remain visible, while keeping low master settings restrained.
  const energy = strength ** 1.3;
  const ghostOffset = strength * e.displacement * scale;
  const ghostOpacity = energy * Math.sqrt(e.ghost / 100) * 0.95;
  const bloomRadius = strength * e.bloom * 0.08;
  const layers = strength
    ? `0 0 ${1 + strength}px ${tint(accent, energy * 0.45)}, 0 0 ${bloomRadius}px ${tint(accent, energy * Math.sqrt(e.bloom / 100) * 0.5)}, ${ghostOffset}px ${-ghostOffset * 0.65}px 0 ${tint(accent, ghostOpacity)}, ${-strength * e.misalignment * 0.01 * scale}px ${strength * e.misalignment * 0.01 * scale}px 0 ${tint(t.colors["accent-secondary"], energy * Math.sqrt(e.misalignment / 100) * 0.65)}`
    : "0 0 0 transparent";
  Object.assign(css, {
    "--km-radius": `${t.radius}px`,
    "--km-font-size": `${t.fontSize}px`,
    "--km-space": `${{ compact: 12, comfortable: 16, spacious: 20 }[t.density]}px`,
    "--km-content-padding": `${{ compact: 16, comfortable: 24, spacious: 28 }[t.density]}px`,
    "--km-glow-text": halo,
    "--km-glow-accent": glow
      ? `0 0 3px ${tint(accent, glow)}, 0 0 11px ${tint(accent, glow * 0.65)}`
      : "0 0 0 transparent",
    "--km-crt-shadow": layers,
    "--km-crt-distortion-strength": strength,
    "--km-crt-ghost-offset": `${ghostOffset}px`,
    "--km-crt-channel-offset": `${strength * e.misalignment * 0.01 * scale}px`,
    "--km-crt-bloom-radius": `${bloomRadius}px`,
    "--km-scanline-display": e.scanlines ? "block" : "none",
    "--km-noise-display": e.noise ? "block" : "none",
    "--km-scanline-opacity": e.scanlines ? 0.055 : 0,
    "--km-noise-opacity": e.noise ? 0.025 : 0,
    "--km-flicker-animation":
      e.flicker && !reducedMotion ? "km-flicker 6s steps(1) infinite" : "none",
  });
  return css as CSSProperties;
}
export const newCustomId = () => `custom-${crypto.randomUUID()}`;
export function contrast(fg: string, bg: string) {
  const lum = (hex: string) => {
    const c = [1, 3, 5]
      .map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
      .map((v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
    return c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
  };
  const a = lum(fg),
    b = lum(bg);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}
