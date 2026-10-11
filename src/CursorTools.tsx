import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Config, Location } from "./types";
export default function CursorTools({
  config,
  setConfig,
  reload,
}: {
  config: Config;
  setConfig: (c: Config) => void;
  reload: () => Promise<void>;
}) {
  const [history, setHistory] = useState<{
    history: Location[];
    history_index: number;
  }>({ history: [], history_index: 0 });
  const [name, setName] = useState("Bookmark");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const poll = () =>
      invoke<{ history: Location[]; history_index: number }>(
        "cursor_tools_snapshot",
      )
        .then(setHistory)
        .catch((e) => setError(String(e)));
    void poll();
    const t = setInterval(() => void poll(), 1000);
    return () => clearInterval(t);
  }, []);
  const action = async (action: string, index?: number) => {
    setBusy(true);
    setError("");
    try {
      setHistory(
        await invoke("cursor_tool", { action, index: index ?? null, name }),
      );
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <>
      {error && <p role="alert">{error}</p>}
      <section className="card">
        <h2>Cursor history</h2>
        <label className="toggle-row">
          <strong>Enable cursor history</strong>
          <input
            aria-label="Enable cursor history"
            role="switch"
            type="checkbox"
            checked={config.features.history_enabled}
            onChange={(e) =>
              setConfig({
                ...config,
                features: {
                  ...config.features,
                  history_enabled: e.target.checked,
                },
              })
            }
          />
        </label>
        <p>
          Records movement stops and explicit grid/bookmark destinations.
          History navigation does not add entries. Save the toggle before using
          these controls.
        </p>
        <button
          disabled={
            busy ||
            !config.features.history_enabled ||
            history.history_index === 0
          }
          onClick={() => void action("back")}
        >
          Back
        </button>{" "}
        <button
          disabled={
            busy ||
            !config.features.history_enabled ||
            history.history_index >= history.history.length - 1
          }
          onClick={() => void action("forward")}
        >
          Forward
        </button>
        <p>{history.history.length} locations in this session.</p>
      </section>
      <section className="card">
        <h2>Bookmarks</h2>
        <label className="toggle-row">
          <strong>Enable bookmarks</strong>
          <input
            aria-label="Enable bookmarks"
            role="switch"
            type="checkbox"
            checked={config.features.bookmarks_enabled}
            onChange={(e) =>
              setConfig({
                ...config,
                features: {
                  ...config.features,
                  bookmarks_enabled: e.target.checked,
                },
              })
            }
          />
        </label>
        <p>
          Save the toggle, then capture in three seconds. Place the cursor at
          your destination during the countdown. Positions follow the saved
          monitor; missing or ambiguous monitors produce an error.
        </p>
        <input
          aria-label="Bookmark name"
          value={name}
          maxLength={80}
          onChange={(e) => setName(e.target.value)}
        />{" "}
        <button
          disabled={busy || !config.features.bookmarks_enabled}
          onClick={() => void action("capture")}
        >
          {busy ? "Working…" : "Capture in 3 seconds"}
        </button>
        {config.bookmarks.map((b, i) => (
          <div className="toggle-row" key={i}>
            <span>
              {b.name} · ({b.x}, {b.y})
            </span>
            <span>
              <button
                disabled={busy || !config.features.bookmarks_enabled}
                onClick={() => void action("bookmark", i)}
              >
                Go
              </button>{" "}
              <button
                disabled={busy || !config.features.bookmarks_enabled}
                onClick={() => void action("delete", i)}
              >
                Delete
              </button>
            </span>
          </div>
        ))}
      </section>
    </>
  );
}
