import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Snapshot } from "./types";
import { usePresentation } from "./Presentation";
export default function MiniControlPanel() {
  const [snapshot, set] = useState<Snapshot | null>(null),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const { data } = usePresentation();
  const [calibrationGroup, setCalibrationGroup] = useState("baseline");
  useEffect(() => {
    let live = true;
    const poll = () =>
      invoke<Snapshot>("get_snapshot")
        .then((s) => {
          if (live) set(s);
        })
        .catch((e) => {
          if (live) setError(String(e));
        });
    void poll();
    const timer = setInterval(() => void poll(), 1000);
    return () => {
      live = false;
      clearInterval(timer);
    };
  }, []);
  const work = async (task: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await task();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const s = snapshot;
  return (
    <div className="mini-panel effects-surface">
      <header>
        <strong className="crt-text">KEYMOUSE_</strong>
        <small>MINI CONTROL PANEL</small>
      </header>
      {error && <p role="alert">{error}</p>}
      {s ? (
        <>
          <section className="card">
            <h2>Engine status</h2>
            <button
              className="status-pill crt-text"
              disabled={busy}
              onClick={() =>
                void work(async () =>
                  set(
                    await invoke("set_enabled", { enabled: !s.status.enabled }),
                  ),
                )
              }
            >
              {s.status.running
                ? s.status.enabled
                  ? "Engine enabled"
                  : "Engine disabled"
                : "Engine stopped"}
            </button>
            <p>
              Mode: {s.status.mode} · {s.status.moving ? "Moving" : "Idle"}
            </p>
            <p>
              Profile:{" "}
              {data?.calibration_active
                ? "Calibration"
                : (data?.matching_profile ?? "Global settings")}
            </p>
            <small>Foreground profile sampled independently.</small>
          </section>
          <section className="card">
            <h2>Saved speed summaries</h2>
            {(["normal", "precision", "turbo"] as const).map((mode) => (
              <div className="mini-speed" key={mode}>
                <span>{mode}</span>
                <strong>
                  {s.config[mode].min_speed}–{s.config[mode].max_speed}
                </strong>
                <small>px/s</small>
              </div>
            ))}
            <small>
              Worker Normal limit: {s.status.normal_max_speed ?? "Unavailable"}{" "}
              px/s
            </small>
          </section>
          <section className="card">
            <h2>Quick features</h2>
            {(["tap", "braking", "axes", "magnifier"] as const).map((k) => (
              <label className="toggle-row" key={k}>
                <span>
                  {
                    {
                      tap: "Tap versus hold",
                      braking: "Adaptive Braking",
                      axes: "Per-axis acceleration",
                      magnifier: "Precision magnifier",
                    }[k]
                  }
                </span>
                <input
                  role="switch"
                  type="checkbox"
                  aria-label={k}
                  disabled={busy}
                  checked={s.config.features[k].enabled}
                  onChange={(e) => {
                    const enabled = e.target.checked;
                    void work(async () => {
                      const fresh = await invoke<Snapshot>("get_snapshot");
                      set(
                        await invoke("save_config", {
                          config: {
                            ...fresh.config,
                            features: {
                              ...fresh.config.features,
                              [k]: { ...fresh.config.features[k], enabled },
                            },
                          },
                        }),
                      );
                    });
                  }}
                />
              </label>
            ))}
          </section>
          <div className="mini-actions">
            {[
              ["Overview", "Open Full App"],
              ["Theme Studio", "Open Theme Studio"],
            ].map(([page, label]) => (
              <button
                key={page}
                disabled={busy}
                onClick={() =>
                  void work(async () => {
                    await invoke("open_page", { page });
                  })
                }
              >
                {label}
              </button>
            ))}
            <label>
              Calibration group
              <select
                aria-label="Calibration group"
                value={calibrationGroup}
                onChange={(e) => setCalibrationGroup(e.target.value)}
              >
                <option value="baseline">Baseline</option>
                <option value="experimental">Experimental</option>
              </select>
            </label>
            <button
              disabled={
                busy ||
                !s.config.features.calibration.enabled ||
                !s.status.enabled ||
                !s.status.running
              }
              onClick={() =>
                void work(async () => {
                  await invoke("calibration_command", {
                    action: calibrationGroup,
                  });
                })
              }
            >
              Start Calibration
            </button>
            <button
              disabled={busy}
              onClick={() =>
                void work(async () => {
                  await invoke("open_page", { page: "Calibration" });
                })
              }
            >
              Calibration setup
            </button>
          </div>
          <p className="fine">
            Calibration uses saved settings. Enable it in the full app before
            starting a session.
          </p>
        </>
      ) : (
        <p>Connecting to KeyMouse…</p>
      )}
    </div>
  );
}
