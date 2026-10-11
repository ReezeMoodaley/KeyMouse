import { useState } from "react";
import type { Config, Profile } from "./types";
export default function Profiles({
  config,
  setConfig,
}: {
  config: Config;
  setConfig: (c: Config) => void;
}) {
  const [name, setName] = useState("Application");
  const [path, setPath] = useState("");
  const edit = (i: number, p: Profile) =>
    setConfig({
      ...config,
      profiles: config.profiles.map((v, j) => (j === i ? p : v)),
    });
  return (
    <section className="card">
      <h2>Application profiles</h2>
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
      <p>
        Exact executable paths select movement overrides. Global feature toggles
        take precedence. Foreground checks happen on a separate service thread;
        profiles never remap controls or own mouse buttons.
      </p>
      <input
        aria-label="Profile name"
        value={name}
        onChange={(e) => setName(e.target.value)}
      />
      <input
        aria-label="Profile executable path"
        placeholder="C:\\Program Files\\Application\\app.exe"
        value={path}
        onChange={(e) => setPath(e.target.value)}
      />
      <button
        onClick={() =>
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
          })
        }
      >
        Add profile from current movement
      </button>
      {config.profiles.map((p, i) => (
        <div className="card" key={i}>
          <h3>{p.name}</h3>
          <p>{p.executable}</p>
          <label>
            <input
              type="checkbox"
              checked={p.enabled}
              onChange={(e) => edit(i, { ...p, enabled: e.target.checked })}
            />{" "}
            Active
          </label>
          <label>
            Normal maximum speed{" "}
            <input
              aria-label={`${p.name} maximum speed`}
              type="number"
              min={p.normal?.min_speed ?? 1}
              max={10000}
              value={p.normal?.max_speed ?? config.normal.max_speed}
              onChange={(e) =>
                edit(i, {
                  ...p,
                  normal: {
                    ...(p.normal ?? config.normal),
                    max_speed: Number(e.target.value),
                  },
                })
              }
            />
          </label>
          <p>Optional overrides use the current global values when captured.</p>
          {(
            [
              "normal",
              "precision",
              "turbo",
              "tap",
              "acceleration",
              "braking",
              "axes",
            ] as const
          ).map((key) => (
            <label className="toggle-row" key={key}>
              <span>Override {key}</span>
              <input
                type="checkbox"
                checked={p[key] !== null}
                onChange={(e) =>
                  edit(i, {
                    ...p,
                    [key]: e.target.checked
                      ? key === "normal" ||
                        key === "precision" ||
                        key === "turbo"
                        ? structuredClone(config[key])
                        : structuredClone(config.features[key])
                      : null,
                  })
                }
              />
            </label>
          ))}
          <button
            onClick={() =>
              setConfig({
                ...config,
                profiles: config.profiles.filter((_, j) => j !== i),
              })
            }
          >
            Delete profile
          </button>
        </div>
      ))}
    </section>
  );
}
