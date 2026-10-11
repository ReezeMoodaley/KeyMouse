import { useState } from "react";
import { useReducedMotion, useThemes } from "./ThemeProvider";
import {
  contrast,
  defaultTheme,
  exportTheme,
  importTheme,
  newCustomId,
  presets,
  themeStyle,
  type ColorToken,
  type Effects,
  type Theme,
} from "./themes";
import { TerminalPanel } from "./Terminal";

const groups = [
  "Backgrounds",
  "Text",
  "Accents",
  "Borders",
  "Status",
  "Controls",
  "Geometry",
] as const;
const prefixes = ["bg-", "text-", "accent-", "border-", "status-", "control-"];
function Slider({
  label,
  value,
  max = 100,
  step = 1,
  onChange,
}: {
  label: string;
  value: number;
  max?: number;
  step?: number;
  onChange: (v: number) => void;
}) {
  return (
    <label className="studio-slider">
      <span>
        {label}{" "}
        <span aria-hidden="true">
          {value}
          {max === 100 ? "%" : ""}
        </span>
      </span>
      <input
        aria-label={label}
        type="range"
        min="0"
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </label>
  );
}
function EffectToggle({
  label,
  value,
  onChange,
}: {
  label: string;
  value: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="toggle-row">
      <span>{label}</span>
      <input
        type="checkbox"
        role="switch"
        aria-label={label}
        checked={value}
        onChange={(e) => onChange(e.target.checked)}
      />
    </label>
  );
}
export function ThemePreview({ theme }: { theme: Theme }) {
  const reduced = useReducedMotion();
  const style = themeStyle(theme, reduced);
  return (
    <div className="theme-preview" style={style} data-testid="theme-preview">
      <div className="preview-interface effects-surface">
        <aside>
          <strong className="crt-text">KEYMOUSE_</strong>
          <span className="crt-text">› Dashboard</span>
          <span>Movement</span>
          <span>Theme Studio</span>
        </aside>
        <div>
          <h3>Cursor instrument</h3>
          <strong className="crt-readout">
            1,400 <small>px/s</small>
          </strong>
          <svg
            viewBox="0 0 200 65"
            role="img"
            aria-label="Illustrative acceleration preview"
          >
            <path
              d="M0 60 Q70 60 115 25 T200 5"
              fill="none"
              stroke="var(--km-accent-primary)"
              strokeWidth="2"
            />
          </svg>
          <button className="primary" type="button">
            Primary
          </button>{" "}
          <button className="secondary" type="button">
            Secondary
          </button>
          <label>
            Example value
            <input aria-label="Preview value" type="number" defaultValue={80} />
          </label>
          <label>
            <input type="checkbox" defaultChecked /> Example toggle
          </label>
          <span className="status-good crt-text">● Engine ready · preview</span>
        </div>
      </div>
      <div className="type-comparison">
        {(
          [
            "Clean Typography",
            "Phosphor Glow",
            "Glow + CRT Distortion",
          ] as const
        ).map((name, i) => (
          <div
            key={name}
            style={themeStyle(
              {
                ...theme,
                effects: {
                  ...theme.effects,
                  glow: i === 0 ? "off" : theme.effects.glow,
                  distortion: i === 2 && theme.effects.distortion,
                },
              },
              reduced,
            )}
          >
            <small>{name}</small>
            <strong className="crt-readout">Precision 0080</strong>
          </div>
        ))}
      </div>
      <p>
        Illustrative controls and readouts. Theme changes preview here
        immediately.
      </p>
    </div>
  );
}
export default function ThemeStudio() {
  const { state, commit, error: serviceError } = useThemes();
  const [draft, setDraft] = useState<Theme>(() =>
    structuredClone(state.active),
  );
  const [group, setGroup] = useState(0);
  const [token, setToken] = useState<ColorToken>("bg-app");
  const [name, setName] = useState(draft.name);
  const [json, setJson] = useState("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const edit = (key: keyof Effects, value: Effects[keyof Effects]) =>
    setDraft((t) => ({ ...t, effects: { ...t.effects, [key]: value } }));
  const choose = (t: Theme) => {
    setDraft(structuredClone(t));
    setName(t.name);
    setError("");
    setNotice("Preview updated. Apply to update every window.");
  };
  const work = async (task: () => Promise<void>) => {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await task();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const custom = state.customs.find((t) => t.id === draft.id);
  const saveCustom = (duplicate = false) =>
    work(async () => {
      const t = {
        ...draft,
        id: duplicate || !custom ? newCustomId() : draft.id,
        name: name.trim(),
      };
      importTheme(exportTheme(t));
      await commit({
        ...state,
        customs: [...state.customs.filter((c) => c.id !== t.id), t],
      });
      choose(t);
      setNotice("Custom theme saved. Apply to activate it.");
    });
  const colors = Object.keys(draft.colors).filter((k) =>
    k.startsWith(prefixes[group]),
  ) as ColorToken[];
  return (
    <div className="terminal-grid theme-studio">
      <TerminalPanel title="Theme presets">
        <div className="preset-selector">
          {[...presets, ...state.customs].map((t) => (
            <button
              key={t.id}
              className={draft.id === t.id ? "active" : ""}
              aria-pressed={draft.id === t.id}
              onClick={() => choose(t)}
            >
              {t.name}
            </button>
          ))}
        </div>
      </TerminalPanel>
      <TerminalPanel title="Token groups" span={3}>
        <div className="token-groups">
          {groups.map((g, i) => (
            <button
              key={g}
              aria-pressed={group === i}
              className={group === i ? "active" : ""}
              onClick={() => {
                setGroup(i);
                if (i < 6)
                  setToken(
                    Object.keys(draft.colors).find((k) =>
                      k.startsWith(prefixes[i]),
                    ) as ColorToken,
                  );
              }}
            >
              {g}
            </button>
          ))}
        </div>
        <p>
          Semantic tokens apply to every screen. Built-in presets always remain
          available.
        </p>
      </TerminalPanel>
      <TerminalPanel
        title="Live theme preview"
        span={5}
        className="preview-panel"
      >
        <ThemePreview theme={draft} />
      </TerminalPanel>
      <TerminalPanel title="Selected token editor" span={4}>
        {group < 6 ? (
          <>
            <label>
              Color token
              <select
                aria-label="Color token"
                value={token}
                onChange={(e) => setToken(e.target.value as ColorToken)}
              >
                {colors.map((k) => (
                  <option key={k} value={k}>
                    {k}
                  </option>
                ))}
              </select>
            </label>
            <label className="color-editor">
              Token color
              <input
                type="color"
                aria-label="Token color"
                value={draft.colors[token]}
                onChange={(e) =>
                  setDraft((t) => ({
                    ...t,
                    colors: { ...t.colors, [token]: e.target.value },
                  }))
                }
              />
              <code>{draft.colors[token]}</code>
            </label>
            <p>
              CSS variable <code>--km-{token}</code>
            </p>
          </>
        ) : (
          <>
            <Slider
              label="Panel radius"
              max={16}
              value={draft.radius}
              onChange={(radius) => setDraft((t) => ({ ...t, radius }))}
            />
            <label>
              Interface density
              <select
                aria-label="Interface density"
                value={draft.density}
                onChange={(e) =>
                  setDraft((t) => ({
                    ...t,
                    density: e.target.value as Theme["density"],
                  }))
                }
              >
                <option>compact</option>
                <option>comfortable</option>
                <option>spacious</option>
              </select>
            </label>
            <label>
              Font size
              <input
                aria-label="Font size"
                type="number"
                min={12}
                max={18}
                value={draft.fontSize}
                onChange={(e) =>
                  setDraft((t) => ({ ...t, fontSize: Number(e.target.value) }))
                }
              />
            </label>
          </>
        )}
        <p>
          Primary contrast:{" "}
          {contrast(
            draft.colors["text-primary"],
            draft.colors["bg-panel"],
          ).toFixed(1)}
          :1. Secondary:{" "}
          {contrast(
            draft.colors["text-secondary"],
            draft.colors["bg-panel"],
          ).toFixed(1)}
          :1.
        </p>
        {["text-primary", "text-secondary", "text-muted"].some(
          (k) =>
            contrast(draft.colors[k as ColorToken], draft.colors["bg-panel"]) <
            4.5,
        ) && (
          <p role="status">
            Low text contrast. Consider a brighter text color or darker panel.
          </p>
        )}
      </TerminalPanel>
      <TerminalPanel title="CRT Effects" span={7}>
        <label>
          Phosphor Glow
          <select
            aria-label="Phosphor Glow"
            value={draft.effects.glow}
            onChange={(e) => edit("glow", e.target.value as Effects["glow"])}
          >
            {["off", "subtle", "standard", "strong"].map((g) => (
              <option key={g}>{g}</option>
            ))}
          </select>
        </label>
        <EffectToggle
          label="CRT Distortion"
          value={draft.effects.distortion}
          onChange={(v) => edit("distortion", v)}
        />
        <Slider
          label="Distortion Intensity"
          value={draft.effects.intensity}
          onChange={(v) => edit("intensity", v)}
        />
        <EffectToggle
          label="Scanlines"
          value={draft.effects.scanlines}
          onChange={(v) => edit("scanlines", v)}
        />
        <EffectToggle
          label="Screen Noise"
          value={draft.effects.noise}
          onChange={(v) => edit("noise", v)}
        />
        <details>
          <summary>Advanced CRT Effects</summary>
          <Slider
            label="Ghosting intensity"
            value={draft.effects.ghost}
            onChange={(v) => edit("ghost", v)}
          />
          <Slider
            label="Ghost displacement"
            max={1}
            step={0.05}
            value={draft.effects.displacement}
            onChange={(v) => edit("displacement", v)}
          />
          <Slider
            label="Colour misalignment strength"
            value={draft.effects.misalignment}
            onChange={(v) => edit("misalignment", v)}
          />
          <Slider
            label="Bloom softness"
            value={draft.effects.bloom}
            onChange={(v) => edit("bloom", v)}
          />
          <EffectToggle
            label="Flicker"
            value={draft.effects.flicker}
            onChange={(v) => edit("flicker", v)}
          />
          <button
            onClick={() =>
              setDraft((t) => ({
                ...t,
                effects: {
                  ...t.effects,
                  ghost: 20,
                  displacement: 0.6,
                  misalignment: 8,
                  bloom: 25,
                  flicker: false,
                },
              }))
            }
          >
            Reset advanced settings
          </button>
        </details>
        <p>
          Master intensity scales all CRT shadow layers. Zero clears those
          layers; glow remains independent. Ghost and fringe displacement never
          exceed one pixel. Reduced motion disables flicker.
        </p>
      </TerminalPanel>
      <TerminalPanel title="Theme management" span={5}>
        <label>
          Theme name
          <input
            aria-label="Theme name"
            maxLength={80}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <div className="button-row">
          <button disabled={busy} onClick={() => void saveCustom()}>
            Save Custom Theme
          </button>
          <button disabled={busy} onClick={() => void saveCustom(true)}>
            Duplicate
          </button>
          <button
            disabled={busy || !custom}
            onClick={() =>
              void work(async () => {
                const next = { ...draft, name: name.trim() };
                importTheme(exportTheme(next));
                await commit({
                  ...state,
                  active:
                    state.active.id === draft.id
                      ? { ...state.active, name: next.name }
                      : state.active,
                  customs: state.customs.map((t) =>
                    t.id === draft.id ? { ...t, name: next.name } : t,
                  ),
                });
                setDraft(next);
                setNotice("Custom theme renamed.");
              })
            }
          >
            Rename
          </button>
          <button
            disabled={busy || !custom}
            onClick={() =>
              void work(async () => {
                await commit({
                  ...state,
                  active:
                    state.active.id === draft.id
                      ? structuredClone(defaultTheme)
                      : state.active,
                  customs: state.customs.filter((t) => t.id !== draft.id),
                });
                choose(defaultTheme);
                setNotice("Custom theme deleted.");
              })
            }
          >
            Delete
          </button>
        </div>
        <label>
          Theme JSON
          <textarea
            aria-label="Theme JSON"
            rows={5}
            value={json}
            onChange={(e) => setJson(e.target.value)}
            spellCheck={false}
          />
        </label>
        <div className="button-row">
          <button
            onClick={() => {
              try {
                const t = importTheme(json);
                choose({ ...t, id: newCustomId() });
                setNotice(
                  "JSON imported into the draft. Save or Apply to retain it.",
                );
              } catch (e) {
                setError(String(e));
              }
            }}
          >
            Import JSON
          </button>
          <button
            onClick={() => {
              try {
                setJson(exportTheme(draft));
                setNotice("JSON exported below. Copy it to save or share.");
              } catch (e) {
                setError(String(e));
              }
            }}
          >
            Export JSON
          </button>
        </div>
        <p>
          Version 1 JSON only. Imports accept bounded settings and hex colors,
          never CSS.
        </p>
      </TerminalPanel>
      <div className="col-12 studio-actions">
        <button
          className="primary"
          disabled={busy}
          onClick={() =>
            void work(async () => {
              await commit({
                ...state,
                active: importTheme(exportTheme(draft)),
              });
              setNotice("Theme applied to every window.");
            })
          }
        >
          Apply
        </button>
        <button
          disabled={busy}
          onClick={() => {
            choose(state.active);
            setNotice("Draft discarded.");
          }}
        >
          Cancel
        </button>
        <button
          disabled={busy}
          onClick={() => {
            choose(defaultTheme);
            setNotice("Default restored in preview. Apply to activate it.");
          }}
        >
          Reset
        </button>
      </div>
      {(error || serviceError) && (
        <p className="col-12 alert" role="alert">
          {error || serviceError}
        </p>
      )}
      {notice && (
        <p className="col-12 notice" role="status">
          {notice}
        </p>
      )}
    </div>
  );
}
