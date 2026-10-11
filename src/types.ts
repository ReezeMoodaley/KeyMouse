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
  profiles: Profile[];
  bookmarks: Location[];
  features: Features;
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
export interface Profile {
  name: string;
  executable: string;
  enabled: boolean;
  normal: Curve | null;
  precision: Config["precision"] | null;
  turbo: Curve | null;
  tap: Features["tap"] | null;
  acceleration: Features["acceleration"] | null;
  braking: Features["braking"] | null;
  axes: Features["axes"] | null;
}
export interface Location {
  name: string;
  x: number;
  y: number;
  display: {
    name: string;
    x: number;
    y: number;
    width: number;
    height: number;
  };
  u: number;
  v: number;
}
export interface Features {
  tap: {
    enabled: boolean;
    strategy: "discrete" | "responsive";
    step: number;
    hold_ms: number;
  };
  acceleration: {
    enabled: boolean;
    strategy: "power" | "linear" | "smoothstep";
  };
  braking: {
    enabled: boolean;
    perpendicular: boolean;
    window_ms: number;
    cap: number;
    recovery_ms: number;
    minimum_speed: number;
  };
  axes: { enabled: boolean; transition_ms: number };
  magnifier: { enabled: boolean; zoom: number; size: number };
  calibration: {
    enabled: boolean;
    target_size: number;
    distance: number;
    trials: number;
  };
  history_enabled: boolean;
  bookmarks_enabled: boolean;
  profiles_enabled: boolean;
}
export const defaultFeatures: Features = {
  tap: { enabled: false, strategy: "discrete", step: 1, hold_ms: 150 },
  acceleration: { enabled: false, strategy: "power" },
  braking: {
    enabled: false,
    perpendicular: false,
    window_ms: 180,
    cap: 120,
    recovery_ms: 350,
    minimum_speed: 200,
  },
  axes: { enabled: false, transition_ms: 80 },
  magnifier: { enabled: false, zoom: 3, size: 240 },
  calibration: { enabled: false, target_size: 20, distance: 500, trials: 5 },
  history_enabled: false,
  bookmarks_enabled: false,
  profiles_enabled: false,
};
export interface Status {
  normal_max_speed?: number;
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
  service_error?: string | null;
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
  const f = c.features;
  const range = (v: number, min: number, max: number, integer = false) =>
    Number.isFinite(v) &&
    v >= min &&
    v <= max &&
    (!integer || Number.isInteger(v));
  if (
    !range(f.tap.step, 1, 10, true) ||
    !range(f.tap.hold_ms, 0, 2000) ||
    !range(f.braking.window_ms, 1, 1000) ||
    !range(f.braking.cap, 1, 10000) ||
    !range(f.braking.recovery_ms, 1, 2000) ||
    !range(f.braking.minimum_speed, 1, 10000) ||
    !range(f.axes.transition_ms, 1, 500) ||
    !range(f.magnifier.zoom, 1.5, 8) ||
    !range(f.magnifier.size, 120, 600, true) ||
    !range(f.calibration.target_size, 4, 100, true) ||
    !range(f.calibration.distance, 10, 3840, true) ||
    !range(f.calibration.trials, 1, 100, true)
  )
    errors.push("Refinement parameters are outside their supported ranges.");
  if (
    c.profiles.some(
      (p) =>
        !p.name.trim() || !/^([a-z]:[\\/]|\\\\).+\.exe$/i.test(p.executable),
    ) ||
    new Set(
      c.profiles.map((p) => p.executable.replaceAll("/", "\\").toLowerCase()),
    ).size !== c.profiles.length
  )
    errors.push("Profiles need a name and unique absolute executable paths.");
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
