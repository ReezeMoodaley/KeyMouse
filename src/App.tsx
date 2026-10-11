import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Refinements from "./Refinements";
import CursorTools from "./CursorTools";
import Profiles from "./Profiles";
import Calibration from "./Calibration";
import Guide from "./Guide";
import {
  ArrowUpRight,
  Activity,
  Command,
  Grid3X3,
  Keyboard,
  MousePointer2,
  Settings2,
  SlidersHorizontal,
  SunMoon,
  Check,
  Save,
  RotateCcw,
  Power,
  AlertCircle,
  BookOpen,
} from "lucide-react";
import {
  actions,
  labels,
  keyName,
  validate,
  curveSpeed,
  type Config,
  type Curve,
  type Snapshot,
  type Status,
} from "./types";
const pages = [
  ["Overview", MousePointer2],
  ["Movement", SlidersHorizontal],
  ["Refinements", SlidersHorizontal],
  ["Cursor tools", MousePointer2],
  ["Application profiles", Settings2],
  ["Calibration", Activity],
  ["Precision guide", BookOpen],
  ["Keybindings", Keyboard],
  ["Mouse & scrolling", Settings2],
  ["Grid navigation", Grid3X3],
  ["Diagnostics", Activity],
  ["General", SunMoon],
] as const;
function NumberField({
  label,
  value,
  min,
  max,
  step = 1,
  unit,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  unit: string;
  onChange: (n: number) => void;
}) {
  return (
    <label className="number-field">
      <span>{label}</span>
      <div>
        <input
          type="number"
          aria-label={label}
          value={Number.isFinite(value) ? value : ""}
          min={min}
          max={max}
          step={step}
          onChange={(e) =>
            onChange(e.target.value === "" ? NaN : Number(e.target.value))
          }
        />
        <small>{unit}</small>
      </div>
    </label>
  );
}
function Toggle({
  label,
  description,
  checked,
  onChange,
}: {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (n: boolean) => void;
}) {
  return (
    <label className="toggle-row">
      <span>
        <strong>{label}</strong>
        {description && <small>{description}</small>}
      </span>
      <input
        type="checkbox"
        role="switch"
        aria-label={label}
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
    </label>
  );
}
export function CurveGraph({ curve }: { curve: Curve }) {
  const points = Array.from({ length: 61 }, (_, i) => {
    const x = i / 60;
    return `${32 + x * 526},${180 - (curveSpeed(curve, x * curve.ramp_ms) / Math.max(1, curve.max_speed)) * 150}`;
  }).join(" ");
  return (
    <div className="curve">
      <svg viewBox="0 0 590 218" role="img" aria-label="Acceleration curve">
        <defs>
          <linearGradient id="curveFill" x1="0" y1="0" x2="0" y2="1">
            <stop stopColor="currentColor" stopOpacity=".2" />
            <stop offset="1" stopColor="currentColor" stopOpacity="0" />
          </linearGradient>
        </defs>
        {[30, 80, 130, 180].map((y) => (
          <line
            key={y}
            x1="32"
            x2="558"
            y1={y}
            y2={y}
            stroke="currentColor"
            opacity=".12"
          />
        ))}
        <polygon points={`32,180 ${points} 558,180`} fill="url(#curveFill)" />
        <polyline
          points={points}
          fill="none"
          stroke="currentColor"
          strokeWidth="3"
          strokeLinejoin="round"
        />
        <text x="32" y="208">
          0 ms
        </text>
        <text x="480" y="208">
          {curve.ramp_ms} ms
        </text>
        <text x="38" y="23">
          {curve.max_speed} px/s
        </text>
      </svg>
      <small>Time held → speed · normalized on diagonals</small>
    </div>
  );
}
function Bindings({
  config,
  setConfig,
  filter = actions,
}: {
  config: Config;
  setConfig: (c: Config) => void;
  filter?: readonly (typeof actions)[number][];
}) {
  return (
    <div className="bindings">
      {filter.map((a) => (
        <label className="binding" key={a}>
          <span>{labels[a]}</span>
          <select
            aria-label={labels[a]}
            value={config.bindings[a]}
            onChange={(e) =>
              setConfig({
                ...config,
                bindings: { ...config.bindings, [a]: Number(e.target.value) },
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
      ))}
    </div>
  );
}
export default function App() {
  const [page, setPage] = useState<string>("Overview");
  const [saved, setSaved] = useState<Config | null>(null);
  const [draft, setDraft] = useState<Config | null>(null);
  const [status, setStatus] = useState<Status | null>(null);
  const [version, setVersion] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [mode, setMode] = useState<"normal" | "precision" | "turbo">("normal");
  const accept = useCallback((s: Snapshot) => {
    setSaved(s.config);
    setDraft(s.config);
    setStatus(s.status);
    setVersion(s.version);
    setWarning(s.service_error ?? s.warning);
  }, []);
  const load = useCallback(async () => {
    try {
      accept(await invoke<Snapshot>("get_snapshot"));
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [accept]);
  useEffect(() => {
    void load();
    const timer = setInterval(() => {
      invoke<Snapshot>("get_snapshot")
        .then((s) => {
          setStatus(s.status);
          setWarning(s.service_error ?? s.warning);
          setSaved(s.config);
          setDraft((c) => (c ? { ...c, enabled: s.config.enabled } : s.config));
        })
        .catch((e) => setError(String(e)));
    }, 1000);
    return () => clearInterval(timer);
  }, [load]);
  useEffect(() => {
    document.documentElement.dataset.theme = draft?.theme ?? "system";
  }, [draft?.theme]);
  const run = async (task: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    setNotice("");
    try {
      await task();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const dirty =
    !!draft && !!saved && JSON.stringify(draft) !== JSON.stringify(saved);
  const errors = draft ? validate(draft) : [];
  const save = () =>
    run(async () => {
      if (draft) {
        const result = await invoke<Snapshot>("save_config", { config: draft });
        accept(result);
        setNotice(
          result.status.running
            ? "Settings saved and applied."
            : "Settings saved. Restart KeyMouse to retry the native engine.",
        );
      }
    });
  const enable = () =>
    run(async () => {
      if (saved) {
        const s = await invoke<Snapshot>("set_enabled", {
          enabled: !status?.enabled,
        });
        setSaved(s.config);
        setStatus(s.status);
        setDraft((c) => (c ? { ...c, enabled: s.config.enabled } : s.config));
      }
    });
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-icon">
            <MousePointer2 size={22} />
          </div>
          <span>
            KeyMouse<small>KEYBOARD → CURSOR</small>
          </span>
        </div>
        <div className="nav-label">WORKSPACE</div>
        <nav>
          {pages.map(([name, Icon]) => (
            <button
              key={name}
              className={page === name ? "selected" : ""}
              onClick={() => setPage(name)}
            >
              <Icon size={18} />
              {name}
              {page === name && <span className="nav-dot" />}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="keyboard-chip">
            <Command size={16} />
            <span>
              Silakka54 ready<small>QMK / Vial compatible</small>
            </span>
          </div>
          <small>KeyMouse {version && `v${version}`} · Windows</small>
        </div>
      </aside>
      <main>
        <header className="topbar">
          <span>
            Workspace <span className="slash">/</span> {page}
          </span>
          <button
            className={`status-pill ${status?.enabled && status.running ? "on" : ""}`}
            onClick={enable}
            disabled={busy || !saved}
          >
            <span className="dot" />
            {status?.enabled && status.running
              ? "Engine enabled"
              : "Engine disabled"}
            <Power size={13} />
          </button>
        </header>
        <div className="content">
          <div className="page-title">
            <div>
              <span className="eyebrow">YOUR CURSOR, UNDER CONTROL</span>
              <h1>{page}</h1>
              <p>
                {page === "Overview"
                  ? "A little precision. A lot less reaching for your mouse."
                  : page === "Movement"
                    ? "Shape how your cursor responds to every press."
                    : page === "Keybindings"
                      ? "Dedicated controls that stay out of your typing."
                      : page === "Diagnostics"
                        ? "A clear view of the native engine, without keyboard history."
                        : page === "Precision guide"
                          ? "Understand each feature and build a combination that suits your aiming."
                          : "Make KeyMouse work the way you do."}
              </p>
            </div>
            {dirty && <span className="unsaved">Unsaved changes</span>}
          </div>
          {(error || status?.error) && (
            <div className="alert" role="alert">
              <AlertCircle size={18} />
              <span>{error || status?.error}</span>
              {!draft && <button onClick={() => void load()}>Retry</button>}
            </div>
          )}
          {warning && <div className="alert warning">{warning}</div>}
          {notice && (
            <div className="notice" role="status">
              <Check size={16} />
              {notice}
            </div>
          )}
          {!draft ? (
            <section className="card">
              <h2>Connecting to the native engine</h2>
              <p>
                Launch KeyMouse with <code>npm run desktop</code>. The browser
                preview cannot capture Raw Input or change your cursor.
              </p>
            </section>
          ) : (
            <>
              {page === "Overview" && (
                <>
                  <section className="hero-card">
                    <div>
                      <span className="eyebrow">LESS REACH. MORE FLOW.</span>
                      <h2>
                        Your keyboard.
                        <br />
                        Your entire desktop.
                      </h2>
                      <p>
                        Precise taps, smooth acceleration, and fast travel.
                        <br />
                        All from your mouse layer.
                      </p>
                      <button
                        className="primary"
                        onClick={() => setPage("Movement")}
                      >
                        Tune movement <ArrowUpRight size={16} />
                      </button>
                    </div>
                    <div className="hero-keys">
                      <div className="key-row">
                        <kbd className="modifier">
                          F22<small>precision</small>
                        </kbd>
                        <kbd>
                          F13<small>↑</small>
                        </kbd>
                        <kbd className="modifier">
                          F23<small>turbo</small>
                        </kbd>
                      </div>
                      <div className="key-row">
                        <kbd>
                          F15<small>←</small>
                        </kbd>
                        <kbd>
                          F14<small>↓</small>
                        </kbd>
                        <kbd>
                          F16<small>→</small>
                        </kbd>
                      </div>
                      <span>Default mouse-layer layout</span>
                    </div>
                  </section>
                  <div className="stats">
                    <article className="card">
                      <Activity size={18} />
                      <small>ENGINE HEALTH</small>
                      <h3>{status?.running ? "Running" : "Unavailable"}</h3>
                      <p>
                        {status?.moving
                          ? "Moving cursor"
                          : status?.enabled
                            ? "Waiting for input"
                            : "Controls disabled"}
                      </p>
                    </article>
                    <article className="card">
                      <SlidersHorizontal size={18} />
                      <small>ACTIVE PROFILE</small>
                      <h3>{status?.mode || "Normal"}</h3>
                      <p>
                        {saved?.normal.min_speed}–{saved?.normal.max_speed} px/s
                        · {saved?.normal.ramp_ms} ms ramp
                      </p>
                    </article>
                    <article className="card">
                      <Grid3X3 size={18} />
                      <small>GO STRAIGHT THERE</small>
                      <h3>Grid navigation</h3>
                      <p>Ctrl + Alt + {keyName(draft.grid.key)}</p>
                      <button
                        className="text-button"
                        onClick={() =>
                          void run(async () => {
                            await invoke("open_grid");
                          })
                        }
                      >
                        Open grid <ArrowUpRight size={14} />
                      </button>
                    </article>
                  </div>
                  <section className="card">
                    <div className="section-head">
                      <div>
                        <h2>Your controls</h2>
                        <p>Current saved mappings · momentary modifiers</p>
                      </div>
                      <button
                        className="text-button"
                        onClick={() => setPage("Keybindings")}
                      >
                        Edit bindings <ArrowUpRight size={14} />
                      </button>
                    </div>
                    <div className="mapping-grid">
                      {actions.map((a) => (
                        <div key={a}>
                          <span>{labels[a]}</span>
                          <kbd>
                            {keyName(saved?.bindings[a] ?? draft.bindings[a])}
                          </kbd>
                        </div>
                      ))}
                    </div>
                  </section>
                  <div className="tip">
                    <Keyboard size={20} />
                    <p>
                      <strong>Built for your Silakka54.</strong> Assign F13–F24
                      to your Vial mouse layer. Hold precision for pixel-level
                      control, or turbo to cross your displays.
                    </p>
                  </div>
                </>
              )}
              {page === "Refinements" && (
                <Refinements config={draft} setConfig={setDraft} />
              )}
              {page === "Cursor tools" && (
                <CursorTools
                  config={draft}
                  setConfig={setDraft}
                  reload={load}
                />
              )}
              {page === "Application profiles" && (
                <Profiles config={draft} setConfig={setDraft} />
              )}
              {page === "Calibration" && (
                <Calibration config={draft} setConfig={setDraft} />
              )}
              {page === "Precision guide" && (
                <Guide config={draft} onNavigate={setPage} />
              )}
              {page === "Movement" && (
                <>
                  <div className="tabs">
                    {(["normal", "precision", "turbo"] as const).map((m) => (
                      <button
                        key={m}
                        className={m === mode ? "active" : ""}
                        onClick={() => setMode(m)}
                      >
                        {m[0].toUpperCase() + m.slice(1)}
                      </button>
                    ))}
                  </div>
                  <section className="card">
                    <div className="section-head">
                      <div>
                        <h2>
                          {mode[0].toUpperCase() + mode.slice(1)} movement
                        </h2>
                        <p>
                          {mode === "precision"
                            ? "Exact initial steps, followed by delayed continuous movement."
                            : mode === "turbo"
                              ? "Cover long distances with a short acceleration ramp."
                              : "Predictable acceleration from a gentle start."}
                        </p>
                      </div>
                      <span className="badge">
                        {mode === "normal"
                          ? "Default"
                          : mode === "precision"
                            ? "Priority 1"
                            : "Priority 2"}
                      </span>
                    </div>
                    <CurveGraph curve={draft[mode]} />
                    <div className="field-grid">
                      {(
                        [
                          ["Minimum speed", "min_speed", 1, 10000, 1, "px/s"],
                          ["Maximum speed", "max_speed", 1, 10000, 1, "px/s"],
                          [
                            "Acceleration duration",
                            "ramp_ms",
                            1,
                            10000,
                            1,
                            "ms",
                          ],
                          [
                            "Acceleration exponent",
                            "exponent",
                            0.1,
                            5,
                            0.1,
                            "power",
                          ],
                        ] as const
                      ).map(([label, key, min, max, step, unit]) => (
                        <NumberField
                          key={key}
                          label={label}
                          value={draft[mode][key]}
                          min={min}
                          max={max}
                          step={step}
                          unit={unit}
                          onChange={(n) =>
                            setDraft({
                              ...draft,
                              [mode]: { ...draft[mode], [key]: n },
                            })
                          }
                        />
                      ))}
                      {mode === "precision" && (
                        <>
                          <NumberField
                            label="Precision hold delay"
                            value={draft.precision.hold_ms}
                            min={0}
                            max={2000}
                            unit="ms"
                            onChange={(n) =>
                              setDraft({
                                ...draft,
                                precision: { ...draft.precision, hold_ms: n },
                              })
                            }
                          />
                          <NumberField
                            label="Precision step size"
                            value={draft.precision.step}
                            min={1}
                            max={10}
                            unit="px"
                            onChange={(n) =>
                              setDraft({
                                ...draft,
                                precision: { ...draft.precision, step: n },
                              })
                            }
                          />
                        </>
                      )}
                    </div>
                  </section>
                  <div className="tip">
                    <MousePointer2 size={20} />
                    <p>
                      Precision takes priority over turbo. Diagonal continuous
                      movement is normalized; diagonal taps step once per axis.
                      Activating precision while moving does not add a tap.
                    </p>
                  </div>
                </>
              )}
              {page === "Keybindings" && (
                <>
                  <section className="card">
                    <div className="section-head">
                      <div>
                        <h2>Mouse-layer bindings</h2>
                        <p>
                          F13–F24 keep ordinary typing free from cursor actions.
                        </p>
                      </div>
                      <button
                        className="secondary"
                        onClick={() =>
                          void run(async () => {
                            const d = await invoke<Config>("default_config");
                            setDraft({ ...draft, bindings: d.bindings });
                          })
                        }
                      >
                        <RotateCcw size={14} />
                        Default bindings
                      </button>
                    </div>
                    <Bindings config={draft} setConfig={setDraft} />
                  </section>
                  <div className="tip">
                    <Keyboard size={20} />
                    <p>
                      In Vial, replace QMK mouse movement, button and wheel
                      keycodes with these function keys. KeyMouse reads Raw
                      Input in the background. It does not suppress keystrokes;
                      another application can still receive F13–F24.
                    </p>
                  </div>
                </>
              )}
              {page === "Mouse & scrolling" && (
                <>
                  <section className="card">
                    <h2>Wheel behaviour</h2>
                    <p>
                      Hold the horizontal modifier to send horizontal wheel
                      events.
                    </p>
                    <Toggle
                      label="Repeat while held"
                      description="Send the first scroll immediately, then repeat after the delay."
                      checked={draft.scroll.repeat}
                      onChange={(repeat) =>
                        setDraft({
                          ...draft,
                          scroll: { ...draft.scroll, repeat },
                        })
                      }
                    />
                    <div className="field-grid">
                      <NumberField
                        label="Scroll amount"
                        value={draft.scroll.amount}
                        min={1}
                        max={1200}
                        unit="wheel units"
                        onChange={(amount) =>
                          setDraft({
                            ...draft,
                            scroll: { ...draft.scroll, amount },
                          })
                        }
                      />
                      <NumberField
                        label="Scroll repeat delay"
                        value={draft.scroll.delay_ms}
                        min={0}
                        max={2000}
                        unit="ms"
                        onChange={(delay_ms) =>
                          setDraft({
                            ...draft,
                            scroll: { ...draft.scroll, delay_ms },
                          })
                        }
                      />
                      <NumberField
                        label="Scroll repeat interval"
                        value={draft.scroll.interval_ms}
                        min={10}
                        max={1000}
                        unit="ms"
                        onChange={(interval_ms) =>
                          setDraft({
                            ...draft,
                            scroll: { ...draft.scroll, interval_ms },
                          })
                        }
                      />
                    </div>
                    <p className="fine">
                      120 wheel units = one standard notch. Positive horizontal
                      scroll moves right. Application wheel handling varies.
                    </p>
                  </section>
                  <section className="card">
                    <h2>Buttons and wheel controls</h2>
                    <p>
                      Hold a button key to drag; release it to release the mouse
                      button.
                    </p>
                    <Bindings
                      config={draft}
                      setConfig={setDraft}
                      filter={[
                        "left_button",
                        "right_button",
                        "middle_button",
                        "scroll_positive",
                        "scroll_negative",
                        "horizontal",
                      ]}
                    />
                  </section>
                </>
              )}
              {page === "Grid navigation" && (
                <>
                  <section className="card">
                    <div className="section-head">
                      <div>
                        <h2>Jump, then refine</h2>
                        <p>
                          A hierarchical 3 × 3 grid on any connected monitor.
                        </p>
                      </div>
                      <button
                        className="primary"
                        disabled={busy}
                        onClick={() =>
                          void run(async () => {
                            await invoke("open_grid");
                          })
                        }
                      >
                        Open grid <Grid3X3 size={16} />
                      </button>
                    </div>
                    <div className="field-grid">
                      <label className="number-field">
                        <span>Activation shortcut</span>
                        <div>
                          <small>Ctrl + Alt +</small>
                          <select
                            aria-label="Grid activation key"
                            value={draft.grid.key}
                            onChange={(e) =>
                              setDraft({
                                ...draft,
                                grid: {
                                  ...draft.grid,
                                  key: Number(e.target.value),
                                },
                              })
                            }
                          >
                            {[
                              ...Array.from({ length: 26 }, (_, i) => 0x41 + i),
                              ...Array.from({ length: 12 }, (_, i) => 0x70 + i),
                            ].map((k) => (
                              <option key={k} value={k}>
                                {keyName(k)}
                              </option>
                            ))}
                          </select>
                        </div>
                      </label>
                      <NumberField
                        label="Refinement levels"
                        value={draft.grid.depth}
                        min={1}
                        max={6}
                        unit="levels"
                        onChange={(depth) =>
                          setDraft({ ...draft, grid: { ...draft.grid, depth } })
                        }
                      />
                    </div>
                    <div className="grid-demo">
                      {Array.from({ length: 9 }, (_, i) => (
                        <span key={i}>{i + 1}</span>
                      ))}
                    </div>
                    <p>
                      Press 1–9 to refine a region. The cursor jumps to its
                      center at the final level. Enter confirms early, Escape
                      cancels, Backspace restarts, and Tab cycles monitors.
                    </p>
                  </section>
                  <div className="tip">
                    <AlertCircle size={20} />
                    <p>
                      The overlay deliberately takes keyboard focus while
                      active. This protects typing in other applications without
                      a global suppression hook. Switching away cancels the
                      grid. Continuous cursor controls pause until it closes.
                    </p>
                  </div>
                </>
              )}
              {page === "Diagnostics" && (
                <section className="card">
                  <h2>Native engine</h2>
                  <p>
                    Aggregate timing and recognized controls only. No typed text
                    or keyboard history.
                  </p>
                  <dl className="diagnostics">
                    <dt>Worker</dt>
                    <dd>
                      {status?.running
                        ? "Running · keymouse-input-engine"
                        : "Stopped"}
                    </dd>
                    <dt>Configuration</dt>
                    <dd>
                      {errors.length
                        ? "Draft has validation errors"
                        : "Saved configuration validated by Rust"}
                    </dd>
                    <dt>Keyboard devices</dt>
                    <dd>{status?.devices ?? "Unknown"}</dd>
                    <dt>Last recognized action</dt>
                    <dd>{status?.last_action ?? "None yet"}</dd>
                    <dt>Movement ticks</dt>
                    <dd>{status?.ticks ?? 0}</dd>
                    <dt>Stale-tick cancellations</dt>
                    <dd>{status?.stalls ?? 0}</dd>
                    <dt>Maximum scheduled tick interval</dt>
                    <dd>{status?.max_tick_ms.toFixed(2) ?? "0"} ms</dd>
                    <dt>Effective Normal speed limit</dt>
                    <dd>
                      {status?.normal_max_speed ?? draft.normal.max_speed} px/s
                    </dd>
                    <dt>First pixel dispatch p95</dt>
                    <dd>
                      {status?.dispatch_p95_ms == null
                        ? "No samples"
                        : `${status.dispatch_p95_ms.toFixed(2)} ms (${status.dispatch_samples} samples)`}
                    </dd>
                  </dl>
                  <p className="fine">
                    First pixel timing includes acceleration and
                    fractional-pixel delay. It is not a hardware latency
                    certification. See docs/VALIDATION.md for the input-to-API
                    acceptance procedure.
                  </p>
                </section>
              )}
              {page === "General" && (
                <>
                  <section className="card">
                    <h2>Everyday preferences</h2>
                    <Toggle
                      label="Start minimized"
                      description="Launch quietly in the system tray."
                      checked={draft.start_minimized}
                      onChange={(start_minimized) =>
                        setDraft({ ...draft, start_minimized })
                      }
                    />
                    <Toggle
                      label="Start at Windows login"
                      description="Register the current application with Windows startup."
                      checked={draft.start_at_login}
                      onChange={(start_at_login) =>
                        setDraft({ ...draft, start_at_login })
                      }
                    />
                    <label className="binding">
                      <span>Appearance</span>
                      <select
                        aria-label="Appearance"
                        value={draft.theme}
                        onChange={(e) =>
                          setDraft({
                            ...draft,
                            theme: e.target.value as Config["theme"],
                          })
                        }
                      >
                        <option value="system">Follow system</option>
                        <option value="dark">Dark</option>
                        <option value="light">Light</option>
                      </select>
                    </label>
                  </section>
                  <section className="card">
                    <h2>Safety and recovery</h2>
                    <p>
                      <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Shift</kbd> +{" "}
                      <kbd>K</kbd> immediately disables the engine, releases
                      owned buttons and cancels grid navigation.
                    </p>
                    <p>
                      Keep a physical mouse available during setup. Closing
                      settings hides the window; use Quit KeyMouse in the tray
                      to exit.
                    </p>
                    <button
                      className="secondary"
                      disabled={busy}
                      onClick={() =>
                        void run(async () => {
                          accept(await invoke<Snapshot>("reset_config"));
                          setNotice("Defaults restored.");
                        })
                      }
                    >
                      <RotateCcw size={14} />
                      Restore all defaults
                    </button>
                  </section>
                  <section className="card about">
                    <div className="brand-icon">
                      <MousePointer2 />
                    </div>
                    <div>
                      <h2>KeyMouse {version}</h2>
                      <p>
                        Local, private cursor control for Windows. No telemetry,
                        accounts, or network services.
                      </p>
                    </div>
                  </section>
                </>
              )}
              {errors.length > 0 && (
                <div className="alert" role="alert">
                  <AlertCircle size={18} />
                  <div>
                    {errors.map((e) => (
                      <p key={e}>{e}</p>
                    ))}
                  </div>
                </div>
              )}
              <footer>
                <span>
                  {dirty
                    ? "Apply your changes to the native engine."
                    : "Settings are stored locally by Rust."}
                </span>
                <div>
                  {dirty && (
                    <button
                      className="secondary"
                      disabled={busy}
                      onClick={() => setDraft(saved)}
                    >
                      Discard changes
                    </button>
                  )}
                  <button
                    className="primary"
                    disabled={busy || !dirty || errors.length > 0}
                    onClick={() => void save()}
                  >
                    <Save size={15} />
                    {busy ? "Applying…" : "Save changes"}
                  </button>
                </div>
              </footer>
            </>
          )}
        </div>
      </main>
    </div>
  );
}
