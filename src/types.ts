export const actions = [
  "up",
  "down",
  "left",
  "right",
  "left_button",
  "right_button",
  "middle_button",
  "scroll_positive",
  "scroll_negative",
  "precision",
  "turbo",
  "horizontal",
] as const;
export type Action = (typeof actions)[number];
export const labels: Record<Action, string> = {
  up: "Cursor up",
  down: "Cursor down",
  left: "Cursor left",
  right: "Cursor right",
  left_button: "Left mouse button",
  right_button: "Right mouse button",
  middle_button: "Middle mouse button",
  scroll_positive: "Scroll positive",
  scroll_negative: "Scroll negative",
  precision: "Precision modifier",
  turbo: "Turbo modifier",
  horizontal: "Horizontal scroll modifier",
};
export interface Curve {
  min_speed: number;
  max_speed: number;
  ramp_ms: number;
  exponent: number;
}
export interface Config {
  version: number;
  enabled: boolean;
  bindings: Record<Action, number>;
  normal: Curve;
  precision: Curve & { hold_ms: number; step: number };
  turbo: Curve;
  scroll: {
    repeat: boolean;
    delay_ms: number;
    interval_ms: number;
    amount: number;
  };
  grid: { key: number; depth: number };
  start_minimized: boolean;
  start_at_login: boolean;
  theme: "system" | "light" | "dark";
}
export interface Status {
  running: boolean;
  enabled: boolean;
  moving: boolean;
  mode: string;
  last_action: string | null;
  error: string | null;
  devices: number;
  ticks: number;
  stalls: number;
  max_tick_ms: number;
  dispatch_p95_ms: number | null;
  dispatch_samples: number;
}
export interface Snapshot {
  config: Config;
  status: Status;
  warning: string | null;
  version: string;
}
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface GridSession {
  monitors: Rect[];
  monitor: number;
  region: Rect;
  level: number;
  depth: number;
}
export function keyName(key: number) {
  return key >= 0x70 && key <= 0x87
    ? `F${key - 0x6f}`
    : String.fromCharCode(key);
}
export function validate(c: Config): string[] {
  const errors: string[] = [];
  if (new Set(Object.values(c.bindings)).size !== actions.length)
    errors.push("Each action needs a distinct binding.");
  if (
    actions.some(
      (a) =>
        !Number.isInteger(c.bindings[a]) ||
        c.bindings[a] < 0x7c ||
        c.bindings[a] > 0x87,
    )
  )
    errors.push("Bindings must use F13–F24.");
  for (const mode of ["normal", "precision", "turbo"] as const) {
    const v = c[mode];
    if (
      ![v.min_speed, v.max_speed, v.ramp_ms, v.exponent].every(
        Number.isFinite,
      ) ||
      v.min_speed < 1 ||
      v.max_speed > 10000 ||
      v.max_speed < v.min_speed ||
      v.ramp_ms < 1 ||
      v.ramp_ms > 10000 ||
      v.exponent < 0.1 ||
      v.exponent > 5
    )
      errors.push(`${mode}: check speed, ramp and exponent bounds.`);
  }
  if (
    !Number.isFinite(c.precision.hold_ms) ||
    c.precision.hold_ms < 0 ||
    c.precision.hold_ms > 2000 ||
    c.precision.step < 1 ||
    c.precision.step > 10 ||
    !Number.isInteger(c.precision.step)
  )
    errors.push("Precision delay must be 0–2000 ms and step 1–10 pixels.");
  if (
    !Number.isInteger(c.scroll.amount) ||
    !Number.isInteger(c.scroll.delay_ms) ||
    !Number.isInteger(c.scroll.interval_ms) ||
    c.scroll.amount < 1 ||
    c.scroll.amount > 1200 ||
    c.scroll.delay_ms < 0 ||
    c.scroll.delay_ms > 2000 ||
    c.scroll.interval_ms < 10 ||
    c.scroll.interval_ms > 1000
  )
    errors.push(
      "Scroll amount must be 1–1200, delay 0–2000 ms and interval 10–1000 ms.",
    );
  if (!Number.isInteger(c.grid.depth) || c.grid.depth < 1 || c.grid.depth > 6)
    errors.push("Grid depth must be 1–6 levels.");
  return errors;
}
export function curveSpeed(c: Curve, t: number) {
  return (
    c.min_speed +
    (c.max_speed - c.min_speed) *
      Math.pow(Math.min(1, Math.max(0, t / c.ramp_ms)), c.exponent)
  );
}
