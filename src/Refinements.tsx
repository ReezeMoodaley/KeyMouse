import type { Config, Features } from "./types";
export default function Refinements({
  config,
  setConfig,
}: {
  config: Config;
  setConfig: (c: Config) => void;
}) {
  const f = config.features;
  const update = <K extends keyof Features>(key: K, value: Features[K]) =>
    setConfig({ ...config, features: { ...f, [key]: value } });
  const toggle = (
    label: string,
    checked: boolean,
    change: (v: boolean) => void,
  ) => (
    <label className="toggle-row">
      <strong>{label}</strong>
      <input
        aria-label={label}
        type="checkbox"
        role="switch"
        checked={checked}
        onChange={(e) => change(e.target.checked)}
      />
    </label>
  );
  const number = (
    label: string,
    value: number,
    min: number,
    max: number,
    change: (v: number) => void,
  ) => (
    <label className="number-field">
      <span>{label}</span>
      <input
        aria-label={label}
        type="number"
        value={value}
        min={min}
        max={max}
        onChange={(e) => change(Number(e.target.value))}
      />
    </label>
  );
  return (
    <>
      <section className="card">
        <h2>Tap versus Hold</h2>
        {toggle("Enable Tap versus Hold", f.tap.enabled, (enabled) =>
          update("tap", { ...f.tap, enabled }),
        )}
        <p>
          Discrete emits a step immediately and intentionally waits before
          holding. Responsive adds an immediate step and continues without a
          dead zone. Precision retains its own configured tap step.
        </p>
        <label>
          Tap strategy{" "}
          <select
            aria-label="Tap strategy"
            value={f.tap.strategy}
            onChange={(e) =>
              update("tap", {
                ...f.tap,
                strategy: e.target.value as Features["tap"]["strategy"],
              })
            }
          >
            <option value="discrete">Discrete Tap</option>
            <option value="responsive">Responsive Continuous</option>
          </select>
        </label>
        <div className="field-grid">
          {number("Tap step (pixels)", f.tap.step, 1, 10, (step) =>
            update("tap", { ...f.tap, step }),
          )}
          {number(
            "Tap hold threshold (ms)",
            f.tap.hold_ms,
            0,
            2000,
            (hold_ms) => update("tap", { ...f.tap, hold_ms }),
          )}
        </div>
      </section>
      <section className="card">
        <h2>Acceleration strategy</h2>
        {toggle(
          "Enable acceleration strategy",
          f.acceleration.enabled,
          (enabled) => update("acceleration", { ...f.acceleration, enabled }),
        )}
        <label>
          Strategy{" "}
          <select
            aria-label="Acceleration strategy"
            value={f.acceleration.strategy}
            onChange={(e) =>
              update("acceleration", {
                ...f.acceleration,
                strategy: e.target
                  .value as Features["acceleration"]["strategy"],
              })
            }
          >
            <option value="power">Power curve</option>
            <option value="linear">Linear</option>
            <option value="smoothstep">Smoothstep</option>
          </select>
        </label>
        <p>
          Uses the speeds and duration configured on Movement. Disabled uses the
          original power curve.
        </p>
      </section>
      <section className="card">
        <h2>Adaptive Braking</h2>
        {toggle("Enable Adaptive Braking", f.braking.enabled, (enabled) =>
          update("braking", { ...f.braking, enabled }),
        )}
        {toggle(
          "Brake perpendicular corrections",
          f.braking.perpendicular,
          (perpendicular) => update("braking", { ...f.braking, perpendicular }),
        )}
        <p>
          Constrains a fast reversal or a recent opposite re-press. Opposing
          keys cancel; they do not trigger braking. No inertia.
        </p>
        <div className="field-grid">
          {number(
            "Correction window (ms)",
            f.braking.window_ms,
            1,
            1000,
            (window_ms) => update("braking", { ...f.braking, window_ms }),
          )}
          {number(
            "Corrective speed cap (px/s)",
            f.braking.cap,
            1,
            10000,
            (cap) => update("braking", { ...f.braking, cap }),
          )}
          {number(
            "Braking recovery (ms)",
            f.braking.recovery_ms,
            1,
            2000,
            (recovery_ms) => update("braking", { ...f.braking, recovery_ms }),
          )}
          {number(
            "Braking minimum prior speed (px/s)",
            f.braking.minimum_speed,
            1,
            10000,
            (minimum_speed) =>
              update("braking", { ...f.braking, minimum_speed }),
          )}
        </div>
      </section>
      <section className="card">
        <h2>Per-Axis Acceleration</h2>
        {toggle("Enable Per-Axis Acceleration", f.axes.enabled, (enabled) =>
          update("axes", { ...f.axes, enabled }),
        )}
        <p>
          Each axis has its own clock; total vector speed stays within the
          active mode maximum. A short transition softens diagonal additions.
        </p>
        {number(
          "Axis transition (ms)",
          f.axes.transition_ms,
          1,
          500,
          (transition_ms) => update("axes", { ...f.axes, transition_ms }),
        )}
      </section>
      <section className="card">
        <h2>Precision magnifier</h2>
        {toggle("Enable precision magnifier", f.magnifier.enabled, (enabled) =>
          update("magnifier", { ...f.magnifier, enabled }),
        )}
        <p>
          A native lens appears while Precision is held. It does not take focus
          or intercept clicks; rendering runs separately from movement.
        </p>
        <div className="field-grid">
          {number("Magnifier zoom", f.magnifier.zoom, 1.5, 8, (zoom) =>
            update("magnifier", { ...f.magnifier, zoom }),
          )}
          {number(
            "Magnifier size (physical pixels)",
            f.magnifier.size,
            120,
            600,
            (size) => update("magnifier", { ...f.magnifier, size }),
          )}
        </div>
      </section>
    </>
  );
}
