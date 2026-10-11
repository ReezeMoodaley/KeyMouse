import { useState } from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { beforeEach, describe, it, expect, vi } from "vitest";
import CursorTools from "./CursorTools";
import Profiles from "./Profiles";
import Calibration from "./Calibration";
import { defaultFeatures, actions, type Config } from "./types";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
const config = (): Config => ({
  version: 1,
  enabled: true,
  features: structuredClone(defaultFeatures),
  bookmarks: [],
  profiles: [],
  bindings: Object.fromEntries(
    actions.map((a, i) => [a, 0x7c + i]),
  ) as Config["bindings"],
  normal: { min_speed: 40, max_speed: 1400, ramp_ms: 900, exponent: 1.6 },
  precision: {
    min_speed: 8,
    max_speed: 80,
    ramp_ms: 700,
    exponent: 1.8,
    hold_ms: 160,
    step: 1,
  },
  turbo: { min_speed: 900, max_speed: 4000, ramp_ms: 350, exponent: 1.2 },
  scroll: { repeat: true, delay_ms: 300, interval_ms: 70, amount: 120 },
  grid: { key: 71, depth: 3 },
  start_minimized: false,
  start_at_login: false,
  theme: "system",
});
beforeEach(() => invoke.mockReset());
describe("tool vertical slices", () => {
  it("history and bookmarks have independent toggles and invoke real command boundaries", async () => {
    let current = config();
    current.features.history_enabled = true;
    current.features.bookmarks_enabled = true;
    invoke.mockImplementation(async (cmd: string) =>
      cmd === "cursor_tools_snapshot" || cmd === "cursor_tool"
        ? { history: [{}, {}], history_index: 1 }
        : {},
    );
    const reload = vi.fn(async () => {});
    function Host() {
      const [c, set] = useState(current);
      return (
        <CursorTools
          config={c}
          setConfig={(v) => {
            current = v;
            set(v);
          }}
          reload={reload}
        />
      );
    }
    render(<Host />);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Back" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "cursor_tool",
        expect.objectContaining({ action: "back" }),
      ),
    );
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Capture in 3 seconds" }),
      ).toBeEnabled(),
    );
    fireEvent.change(screen.getByLabelText("Bookmark name"), {
      target: { value: "Corner" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Capture in 3 seconds" }),
    );
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "cursor_tool",
        expect.objectContaining({ action: "capture", name: "Corner" }),
      ),
    );
    fireEvent.click(
      screen.getByRole("switch", { name: "Enable cursor history" }),
    );
    expect(current.features.history_enabled).toBe(false);
    expect(current.features.bookmarks_enabled).toBe(true);
  });
  it("calibration does not start measuring on mount and begins only explicitly", async () => {
    let current = config();
    invoke.mockResolvedValue({ session: null, results: [] });
    function Host() {
      const [c, set] = useState(current);
      return (
        <Calibration
          config={c}
          setConfig={(v) => {
            current = v;
            set(v);
          }}
        />
      );
    }
    render(<Host />);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("calibration_snapshot"),
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "calibration_command",
      expect.anything(),
    );
    fireEvent.click(screen.getByRole("switch", { name: "Enable calibration" }));
    fireEvent.change(screen.getByLabelText("Trials per session"), {
      target: { value: "8" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Begin baseline" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("calibration_command", {
        action: "baseline",
      }),
    );
    expect(current.features.calibration.trials).toBe(8);
  });
  it("profiles persist a path and independent movement overrides", () => {
    let current = config();
    function Host() {
      const [c, set] = useState(current);
      return (
        <Profiles
          config={c}
          setConfig={(v) => {
            current = v;
            set(v);
          }}
        />
      );
    }
    render(<Host />);
    fireEvent.click(
      screen.getByRole("switch", { name: "Enable application profiles" }),
    );
    fireEvent.change(screen.getByLabelText("Profile executable path"), {
      target: { value: "C:\\editor.exe" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Add profile from current movement" }),
    );
    fireEvent.change(screen.getByLabelText("Application maximum speed"), {
      target: { value: "700" },
    });
    expect(current.profiles[0].normal?.max_speed).toBe(700);
    expect(current.normal.max_speed).toBe(1400);
    fireEvent.click(
      screen.getByRole("switch", { name: "Enable application profiles" }),
    );
    expect(current.profiles).toHaveLength(1);
  });
});
