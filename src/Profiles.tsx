import { useState } from "react";
import type { Config, Profile } from "./types";
import { TerminalPanel } from "./Terminal";
import { usePresentation } from "./Presentation";
export default function Profiles({
  config,
  setConfig,
}: {
  config: Config;
  setConfig: (c: Config) => void;
}) {
  const [name, setName] = useState("Application"),
    [path, setPath] = useState(""),
    [selected, setSelected] = useState(0);
  const { data, error } = usePresentation();
  const p = config.profiles[selected];
  const edit = (next: Profile) =>
    setConfig({
      ...config,
      profiles: config.profiles.map((v, i) => (i === selected ? next : v)),
    });
  const overrides = [
    "normal",
    "precision",
    "turbo",
    "tap",
    "acceleration",
    "braking",
    "axes",
  ] as const;
  return (
    <div className="terminal-grid profile-workspace">
      <TerminalPanel title="Profile list" span={3}>
        <label className="toggle-row">
          <strong>Enable application profiles</strong>
          <input
            aria-label="Enable application profiles"
            role="switch"
            type="checkbox"
            checked={config.features.profiles_enabled}
            onChange={(e) =>
              setConfig({
                ...config,
                features: {
                  ...config.features,
                  profiles_enabled: e.target.checked,
                },
              })
            }
          />
        </label>
        <div className="profile-list">
          {config.profiles.map((profile, i) => (
            <button
              key={i}
              className={i === selected ? "active" : ""}
              aria-pressed={i === selected}
              onClick={() => setSelected(i)}
            >
              {profile.name} {profile.enabled ? "" : "(disabled)"}
            </button>
          ))}
        </div>
        <label>
          Profile name
          <input
            aria-label="Profile name"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <label>
          Executable path
          <input
            aria-label="Profile executable path"
            placeholder="Full executable path"
            value={path}
            onChange={(e) => setPath(e.target.value)}
          />
        </label>
        <button
          onClick={() => {
            setConfig({
              ...config,
              profiles: [
                ...config.profiles,
                {
                  name,
                  executable: path,
                  enabled: true,
                  normal: { ...config.normal },
                  precision: null,
                  turbo: null,
                  tap: null,
                  acceleration: null,
                  braking: null,
                  axes: null,
                },
              ],
            });
            setSelected(config.profiles.length);
          }}
        >
          Add profile from current movement
        </button>
      </TerminalPanel>
      <TerminalPanel title="Profile editor" span={6}>
        {p ? (
          <>
            <h3>{p.name}</h3>
            <label>
              Selected profile name
              <input
                aria-label="Selected profile name"
                value={p.name}
                onChange={(e) => edit({ ...p, name: e.target.value })}
              />
            </label>
            <label>
              Selected executable
              <input
                aria-label="Selected executable"
                value={p.executable}
                onChange={(e) => edit({ ...p, executable: e.target.value })}
              />
            </label>
            <label className="toggle-row">
              <span>Active</span>
              <input
                type="checkbox"
                aria-label={`${p.name} active`}
                checked={p.enabled}
                onChange={(e) => edit({ ...p, enabled: e.target.checked })}
              />
            </label>
            <label>
              Normal maximum speed
              <input
                aria-label={`${p.name} maximum speed`}
                type="number"
                min={p.normal?.min_speed ?? 1}
                max={10000}
                value={p.normal?.max_speed ?? config.normal.max_speed}
                onChange={(e) =>
                  edit({
                    ...p,
                    normal: {
                      ...(p.normal ?? config.normal),
                      max_speed: Number(e.target.value),
                    },
                  })
                }
              />
            </label>
            <p>
              Enable an override to capture current global settings. Disabled
              overrides inherit global values.
            </p>
            {overrides.map((key) => (
              <label className="toggle-row" key={key}>
                <span>Override {key}</span>
                <input
                  aria-label={`Override ${key}`}
                  type="checkbox"
                  checked={p[key] !== null}
                  onChange={(e) =>
                    edit({
                      ...p,
                      [key]: e.target.checked
                        ? structuredClone(
                            key === "normal" ||
                              key === "precision" ||
                              key === "turbo"
                              ? config[key]
                              : config.features[key],
                          )
                        : null,
                    })
                  }
                />
              </label>
            ))}
            <button
              onClick={() => {
                setConfig({
                  ...config,
                  profiles: config.profiles.filter((_, i) => i !== selected),
                });
                setSelected(Math.max(0, selected - 1));
              }}
            >
              Delete profile
            </button>
          </>
        ) : (
          <p>Select or create a profile to edit its movement overrides.</p>
        )}
      </TerminalPanel>
      <TerminalPanel title="Effective configuration" span={3}>
        <strong className="crt-readout">
          {data?.calibration_active
            ? "Calibration"
            : (data?.matching_profile ?? "Global settings")}
        </strong>
        <p>
          Foreground match sampled independently from the input worker. Draft
          changes apply after Save changes.
        </p>
        {data &&
          (["normal", "precision", "turbo"] as const).map((mode) => (
            <p key={mode}>
              {mode}: {data.effective[mode].min_speed}-
              {data.effective[mode].max_speed} px/s
              <br />
              {data.effective[mode].ramp_ms} ms ramp
            </p>
          ))}
        <p>Global feature switches take precedence over profile switches.</p>
      </TerminalPanel>
      <TerminalPanel title="Application matching diagnostics">
        <p className="path-text">
          Foreground executable: {data?.executable ?? "Unavailable"}
        </p>
        <p>
          Matching uses the full executable path, ignoring case and slash
          direction. The first enabled matching profile wins. During calibration
          profiles are suspended.
        </p>
        {error && <p role="status">{error}</p>}
      </TerminalPanel>
    </div>
  );
}
