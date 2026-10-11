import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  actions,
  keyName,
  labels,
  validate,
  type Action,
  type Config,
  type Status,
} from "./types";
import { TerminalPanel, MetricReadout } from "./Terminal";
export interface PresentationSnapshot {
  executable: string | null;
  matching_profile: string | null;
  effective: Config;
  displays: {
    name: string;
    x: number;
    y: number;
    width: number;
    height: number;
  }[];
  scale_factor: number;
  calibration_active: boolean;
}
export function usePresentation() {
  const [data, set] = useState<PresentationSnapshot | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    let live = true;
    const poll = async () => {
      try {
        const s = await invoke<PresentationSnapshot>("presentation_snapshot");
        if (!s?.effective || !Array.isArray(s.displays))
          throw Error("Presentation information unavailable");
        if (live) {
          set(s);
          setError("");
        }
      } catch (e) {
        if (live) setError(String(e));
      }
    };
    void poll();
    const timer = setInterval(() => void poll(), 1500);
    return () => {
      live = false;
      clearInterval(timer);
    };
  }, []);
  return { data, error };
}
export function useObservedEvents(status: Status | null) {
  const [events, set] = useState<{ time: string; action: string }[]>([]);
  useEffect(() => {
    if (status?.last_action) {
      const action = status.last_action;
      set((old) =>
        old[0]?.action === action
          ? old
          : [{ time: new Date().toLocaleTimeString(), action }, ...old].slice(
              0,
              20,
            ),
      );
    }
  }, [status?.last_action]);
  return events;
}
export function RecentEvents({
  events,
}: {
  events: ReturnType<typeof useObservedEvents>;
}) {
  return (
    <>
      <p>
        Recognized action changes observed while this window is open. Sampling
        can miss fast transitions; ordinary typing is never recorded.
      </p>
      {events.length ? (
        <ol className="event-log">
          {events.map((e, i) => (
            <li key={i}>
              <time>{e.time}</time>
              <span>{e.action}</span>
            </li>
          ))}
        </ol>
      ) : (
        <p>No action changes observed yet.</p>
      )}
    </>
  );
}
export function ProfileReadout({
  data,
  status,
}: {
  data: PresentationSnapshot | null;
  status: Status | null;
}) {
  return (
    <>
      <strong className="crt-readout">
        {data?.calibration_active
          ? "Calibration"
          : (data?.matching_profile ?? "Global settings")}
      </strong>
      <p>
        {data
          ? "Foreground match sampled separately from the worker."
          : "Waiting for foreground information."}
      </p>
      <p className="path-text">
        {data?.executable ?? "Executable unavailable"}
      </p>
      <dl>
        <dt>Worker Normal limit</dt>
        <dd>{status?.normal_max_speed ?? "Unavailable"} px/s</dd>
      </dl>
    </>
  );
}
export function DiagnosticsScreen({
  config,
  status,
  data,
  events,
  onReload,
  onDisable,
}: {
  config: Config;
  status: Status | null;
  data: PresentationSnapshot | null;
  events: ReturnType<typeof useObservedEvents>;
  onReload: () => void;
  onDisable: () => void;
}) {
  return (
    <div className="terminal-grid">
      <TerminalPanel title="Input engine" span={4}>
        <MetricReadout
          label="Worker"
          value={status?.running ? "Running" : "Stopped"}
        />
        <p>
          {status?.enabled ? "Controls enabled" : "Controls disabled"} ·{" "}
          {status?.moving ? "Moving" : "Idle"}
        </p>
        <p>{status?.last_action ?? "No recognized action yet"}</p>
      </TerminalPanel>
      <TerminalPanel title="Configuration" span={4}>
        <p>
          {validate(config).length
            ? "Draft has validation errors"
            : "Draft passes frontend validation"}
        </p>
        <p>
          Schema version {config.version}. Rust validates saved configuration.
        </p>
        <ProfileReadout data={data} status={status} />
      </TerminalPanel>
      <TerminalPanel title="Display / DPI" span={4}>
        <p>
          Settings window scale:{" "}
          {data ? `${data.scale_factor.toFixed(2)}×` : "Unavailable"}
        </p>
        {data?.displays.map((d, i) => (
          <p key={i}>
            {d.name || `Display ${i + 1}`}
            <br />
            {d.width} × {d.height} physical px at ({d.x}, {d.y})
          </p>
        ))}
        <p>
          Scale belongs to this window; per-monitor scale is not measured here.
        </p>
      </TerminalPanel>
      <TerminalPanel title="Performance metrics" span={8}>
        <div className="readout-grid">
          <MetricReadout
            label="Movement ticks"
            value={status?.ticks ?? "Unavailable"}
          />
          <MetricReadout
            label="Stale-tick cancellations"
            value={status?.stalls ?? "Unavailable"}
          />
          <MetricReadout
            label="Maximum scheduled interval"
            value={status?.max_tick_ms.toFixed(2) ?? "Unavailable"}
            unit="ms"
          />
          <MetricReadout
            label="First pixel dispatch p95"
            value={status?.dispatch_p95_ms?.toFixed(2) ?? "No samples"}
            unit="ms"
          />
        </div>
        <p>
          {status?.dispatch_samples ?? 0} dispatch samples. Includes
          acceleration and fractional pixel delay; this is not hardware latency
          certification.
        </p>
      </TerminalPanel>
      <TerminalPanel title="Device status" span={4}>
        <MetricReadout
          label="Raw Input keyboard devices"
          value={status?.devices ?? "Unavailable"}
        />
        <p>Device count does not verify Silakka54 firmware or layers.</p>
      </TerminalPanel>
      <TerminalPanel title="Application events" span={8}>
        <RecentEvents events={events} />
      </TerminalPanel>
      <TerminalPanel title="Recovery tools" span={4}>
        <button onClick={onReload}>Reload saved configuration</button>
        <button disabled={!status?.enabled} onClick={onDisable}>
          Disable engine
        </button>
        <p>
          Emergency stop: Ctrl + Alt + Shift + K. A stopped native worker needs
          an application restart.
        </p>
      </TerminalPanel>
    </div>
  );
}
export function KeymapWorkspace({
  config,
  setConfig,
  status,
  children,
}: {
  config: Config;
  setConfig: (c: Config) => void;
  status: Status | null;
  children: React.ReactNode;
}) {
  const [selected, setSelected] = useState<Action>("up");
  const conflicts = validate(config).filter((s) => s.includes("binding"));
  return (
    <div className="terminal-grid">
      <TerminalPanel title="Source / layer">
        <p>
          Manual Silakka54 mouse-layer template · F13–F24 Raw Input bindings.
          Configure physical positions in Vial; this app does not read keyboard
          firmware.
        </p>
      </TerminalPanel>
      <TerminalPanel title="Silakka54 keyboard diagram" span={8}>
        <div className="keymap-diagram">
          {[0, 1].map((half) => (
            <div className="keymap-half" key={half}>
              {Array.from({ length: 27 }, (_, position) => {
                const i = half * 27 + position;
                const action = actions[position];
                return (
                  <button
                    key={i}
                    aria-label={`Template position ${i + 1}${action ? `: ${labels[action]}` : ""}`}
                    className={action === selected ? "active" : ""}
                    onClick={() => {
                      if (action) setSelected(action);
                    }}
                    disabled={!action}
                  >
                    <small>{i + 1}</small>
                    {action ? keyName(config.bindings[action]) : "·"}
                  </button>
                );
              })}
            </div>
          ))}
        </div>
        <p>
          Illustrative positions, not detected assignments. Both halves repeat a
          suggested mouse-layer cluster.
        </p>
      </TerminalPanel>
      <TerminalPanel title="Selected key details" span={4}>
        <h3>{labels[selected]}</h3>
        <label>
          Selected binding
          <select
            aria-label="Selected binding"
            value={config.bindings[selected]}
            onChange={(e) =>
              setConfig({
                ...config,
                bindings: {
                  ...config.bindings,
                  [selected]: Number(e.target.value),
                },
              })
            }
          >
            {Array.from({ length: 12 }, (_, i) => (
              <option key={i} value={0x7c + i}>
                {keyName(0x7c + i)}
              </option>
            ))}
          </select>
        </label>
        <p>Changes share the binding matrix draft. Save changes to apply.</p>
      </TerminalPanel>
      <div className="col-8">{children}</div>
      <TerminalPanel title="Conflict inspector / Input tester" span={4}>
        <p>
          {conflicts.length
            ? conflicts.join(" ")
            : "All action bindings are distinct."}
        </p>
        <p>Last recognized backend action:</p>
        <strong>{status?.last_action ?? "None yet"}</strong>
        <p>
          Only dedicated controls are reported. Test after saving your mappings.
        </p>
      </TerminalPanel>
    </div>
  );
}
