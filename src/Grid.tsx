import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { GridSession } from "./types";
export default function Grid() {
  const [session, setSession] = useState<GridSession | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    let disposed = false;
    let stop = () => {};
    // Register before reading the snapshot so an opening event cannot be lost.
    void listen<GridSession>("grid-state", (e) => {
      if (!disposed) setSession(e.payload);
    })
      .then(async (off) => {
        if (disposed) {
          off();
          return null;
        }
        stop = off;
        return invoke<GridSession | null>("grid_snapshot");
      })
      .then((s) => {
        if (!disposed) setSession(s);
      })
      .catch((e) => {
        if (!disposed) setError(String(e));
      });
    return () => {
      disposed = true;
      stop();
    };
  }, []);
  useEffect(() => {
    let busy = false;
    const key = (e: KeyboardEvent) => {
      if (e.repeat || busy) return;
      let action: string | undefined;
      let region: number | undefined;
      if (/^[1-9]$/.test(e.key)) {
        action = "select";
        region = Number(e.key);
      } else
        action = (
          {
            Escape: "cancel",
            Enter: "confirm",
            Backspace: "restart",
            Tab: "monitor",
          } as Record<string, string>
        )[e.key];
      if (!action) return;
      e.preventDefault();
      busy = true;
      invoke<GridSession | null>("grid_command", {
        action,
        region: region ?? null,
      })
        .then((s) => {
          setSession(s);
          setError("");
        })
        .catch((e) => setError(String(e)))
        .finally(() => {
          busy = false;
        });
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
  if (!session)
    return (
      <div className="grid-overlay">
        <div className="grid-instructions">
          {error || "Preparing grid…"}
          <button
            onClick={() =>
              void invoke("grid_command", { action: "cancel", region: null })
            }
          >
            Cancel
          </button>
        </div>
      </div>
    );
  const monitor = session.monitors[session.monitor];
  const r = session.region;
  const x = r.x - monitor.x;
  const y = r.y - monitor.y;
  return (
    <div className="grid-overlay">
      <svg
        className="grid-canvas"
        viewBox={`0 0 ${monitor.width} ${monitor.height}`}
        preserveAspectRatio="none"
      >
        {Array.from({ length: 9 }, (_, i) => {
          const col = i % 3,
            row = Math.floor(i / 3);
          const left = x + Math.floor((col * r.width) / 3),
            top = y + Math.floor((row * r.height) / 3);
          const w =
              Math.floor(((col + 1) * r.width) / 3) -
              Math.floor((col * r.width) / 3),
            h =
              Math.floor(((row + 1) * r.height) / 3) -
              Math.floor((row * r.height) / 3);
          return (
            <g key={i}>
              <rect x={left} y={top} width={w} height={h} />
              <text
                x={left + w / 2}
                y={top + h / 2}
                fontSize={Math.min(48, Math.max(8, h / 3))}
              >
                {i + 1}
              </text>
            </g>
          );
        })}
      </svg>
      <div className="grid-instructions">
        <strong>KeyMouse grid</strong>
        <span>
          Monitor {session.monitor + 1}/{session.monitors.length} · Level{" "}
          {session.level + 1}/{session.depth}
        </span>
        <p>
          1–9 select · Enter confirm · Esc cancel · Backspace restart · Tab
          monitor
        </p>
        {error && <p role="alert">{error}</p>}
        <button
          onClick={() =>
            void invoke("grid_command", { action: "cancel", region: null })
          }
        >
          Cancel
        </button>
      </div>
    </div>
  );
}
