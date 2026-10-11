import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Config } from "./types";
interface Session {
  display: { x: number; y: number; width: number; height: number };
  start: [number, number];
  target: [number, number];
  size: number;
  measuring: boolean;
  completed: number;
  trials: number;
  group: string;
}
interface Result {
  group: string;
  elapsed_ms: number;
  generated_path: number;
  straight_distance: number;
  efficiency: number | null;
  corrections: number;
  overshoots: number;
  ambiguous: boolean;
  teleports: number;
  samples: number;
  dropped_samples: number;
  max_sample_gap_ms: number;
}
interface Snapshot {
  session: Session | null;
  results: Result[];
}
export function CalibrationOverlay() {
  const [s, set] = useState<Session | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let live = true;
    const subscription = listen<Snapshot>("calibration-state", (e) => {
      if (live) set(e.payload.session);
    });
    void subscription
      .then(() => invoke<Snapshot>("calibration_snapshot"))
      .then((v) => {
        if (live) set(v.session);
      })
      .catch((e) => setError(String(e)));
    return () => {
      live = false;
      void subscription.then((stop) => stop());
    };
  }, []);
  const action = async (action: string) => {
    setBusy(true);
    try {
      set((await invoke<Snapshot>("calibration_command", { action })).session);
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  if (!s) return null;
  const p = s.measuring ? s.target : s.start;
  const scale = window.devicePixelRatio || 1;
  return (
    <div className="calibration-overlay">
      <div className="calibration-info">
        {s.group} · trial {s.completed + 1}/{s.trials} ·{" "}
        {s.measuring
          ? "Move to the target and click"
          : "Click the start target to begin timing"}{" "}
        <button onClick={() => void action("cancel")}>Cancel</button>
        {error && <p role="alert">{error}</p>}
      </div>
      <button
        aria-label={s.measuring ? "Calibration target" : "Calibration start"}
        disabled={busy}
        className="calibration-target"
        style={{
          left: (p[0] - s.display.x - s.size / 2) / scale,
          top: (p[1] - s.display.y - s.size / 2) / scale,
          width: s.size / scale,
          height: s.size / scale,
        }}
        onClick={() => void action("target")}
      />
    </div>
  );
}
export default function Calibration({
  config,
  setConfig,
}: {
  config: Config;
  setConfig: (c: Config) => void;
}) {
  const [snapshot, set] = useState<Snapshot>({ session: null, results: [] });
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const c = config.features.calibration;
  useEffect(() => {
    const poll = () =>
      invoke<Snapshot>("calibration_snapshot")
        .then(set)
        .catch((e) => setError(String(e)));
    void poll();
    const t = setInterval(() => void poll(), 1000);
    return () => clearInterval(t);
  }, []);
  const action = async (action: string) => {
    setBusy(true);
    setError("");
    try {
      set(await invoke("calibration_command", { action }));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const update = (value: typeof c) =>
    setConfig({
      ...config,
      features: { ...config.features, calibration: value },
    });
  return (
    <>
      <section className="card">
        <h2>Calibration sessions</h2>
        <label className="toggle-row">
          <strong>Enable calibration</strong>
          <input
            aria-label="Enable calibration"
            role="switch"
            type="checkbox"
            checked={c.enabled}
            onChange={(e) => update({ ...c, enabled: e.target.checked })}
          />
        </label>
        <p>
          Save settings, then explicitly begin a session. Click the start and
          destination targets using your mouse layer. Baseline disables the four
          movement refinements; experimental uses saved global settings.
          Profiles are held out during a session. Timing starts at the start
          click, not when this panel opens.
        </p>
        <div className="field-grid">
          {(
            [
              ["Target size (pixels)", "target_size", 4, 100],
              ["Target distance (pixels)", "distance", 10, 3840],
              ["Trials per session", "trials", 1, 100],
            ] as const
          ).map(([label, key, min, max]) => (
            <label className="number-field" key={key}>
              {label}
              <input
                aria-label={label}
                type="number"
                min={min}
                max={max}
                value={c[key]}
                onChange={(e) =>
                  update({ ...c, [key]: Number(e.target.value) })
                }
              />
            </label>
          ))}
        </div>
        <button
          disabled={busy || !c.enabled}
          onClick={() => void action("baseline")}
        >
          Begin baseline
        </button>{" "}
        <button
          disabled={busy || !c.enabled}
          onClick={() => void action("experimental")}
        >
          Begin experimental
        </button>{" "}
        <button
          disabled={busy || !snapshot.session}
          onClick={() => void action("cancel")}
        >
          Cancel session
        </button>
        {error && <p role="alert">{error}</p>}
      </section>
      <section className="card">
        <h2>Trial results</h2>
        <p>
          Generated path sums actual cursor displacements around KeyMouse
          updates. Efficiency is straight distance / generated path and is
          omitted for teleports, unobserved movement or dropped samples.
          Physical movement cannot always be separated during an update. Compare
          several trials; one success is not evidence of superiority. Results
          remain in this process only.
        </p>
        <table>
          <thead>
            <tr>
              <th>Group</th>
              <th>Time (ms)</th>
              <th>Path (px)</th>
              <th>Efficiency</th>
              <th>Corrections</th>
              <th>Overshoots</th>
              <th>Quality</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.results.map((r, i) => (
              <tr key={i}>
                <td>{r.group}</td>
                <td>{r.elapsed_ms.toFixed(1)}</td>
                <td>{r.generated_path.toFixed(1)}</td>
                <td>
                  {r.efficiency === null
                    ? "Excluded"
                    : `${(r.efficiency * 100).toFixed(1)}%`}
                </td>
                <td>{r.corrections}</td>
                <td>{r.overshoots}</td>
                <td>
                  {r.ambiguous ? "Ambiguous " : ""}
                  {r.teleports ? `${r.teleports} teleports ` : ""}
                  {r.dropped_samples ? `${r.dropped_samples} dropped ` : ""}
                  {r.samples} samples
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </>
  );
}
