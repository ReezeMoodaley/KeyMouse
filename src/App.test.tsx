import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import {
  actions,
  defaultFeatures,
  validate,
  curveSpeed,
  type Config,
  type Snapshot,
} from "./types";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
const defaults: Config = {
  profiles: [],
  bookmarks: [],
  features: structuredClone(defaultFeatures),
  version: 1,
  enabled: true,
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
};
let saved: Config;
const snapshot = (): Snapshot => ({
  config: structuredClone(saved),
  status: {
    running: true,
    enabled: saved.enabled,
    moving: false,
    mode: "Normal",
    last_action: null,
    error: null,
    devices: 2,
    ticks: 0,
    stalls: 0,
    max_tick_ms: 0,
    dispatch_p95_ms: null,
    dispatch_samples: 0,
  },
  warning: null,
  version: "0.1.0",
});
beforeEach(() => {
  saved = structuredClone(defaults);
  invoke.mockReset();
  invoke.mockImplementation(
    async (command: string, args?: { config?: Config; enabled?: boolean }) => {
      if (command === "default_config") return structuredClone(defaults);
      if (command === "save_config") saved = structuredClone(args!.config!);
      if (command === "set_enabled") saved.enabled = args!.enabled!;
      if (command === "reset_config") saved = structuredClone(defaults);
      return snapshot();
    },
  );
});
async function boot() {
  render(<App />);
  await screen.findByRole("heading", { name: "Engine status" });
}
describe("settings integration", () => {
  it("renders saved configuration and updates movement through Rust IPC", async () => {
    saved.normal.max_speed = 1550;
    await boot();
    fireEvent.click(screen.getByRole("button", { name: "Movement" }));
    expect(screen.getByLabelText("Maximum speed")).toHaveValue(1550);
    fireEvent.change(screen.getByLabelText("Maximum speed"), {
      target: { value: "1800" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    await screen.findByText("Settings saved and applied.");
    expect(saved.normal.max_speed).toBe(1800);
    expect(invoke).toHaveBeenCalledWith("save_config", {
      config: expect.objectContaining({
        normal: expect.objectContaining({ max_speed: 1800 }),
      }),
    });
  });
  it("rejects invalid speed before saving", async () => {
    await boot();
    fireEvent.click(screen.getByRole("button", { name: "Movement" }));
    fireEvent.change(screen.getByLabelText("Maximum speed"), {
      target: { value: "2" },
    });
    expect(screen.getByRole("alert")).toHaveTextContent("check speed");
    expect(screen.getByRole("button", { name: "Save changes" })).toBeDisabled();
  });
  it("shows binding conflicts and restores bindings", async () => {
    await boot();
    fireEvent.click(screen.getByRole("button", { name: "Keymapping" }));
    fireEvent.change(screen.getByLabelText("Cursor up"), {
      target: { value: "125" },
    });
    expect(screen.getByRole("alert")).toHaveTextContent("distinct binding");
    fireEvent.click(screen.getByRole("button", { name: "Default bindings" }));
    await waitFor(() =>
      expect(screen.queryByRole("alert")).not.toBeInTheDocument(),
    );
    expect(screen.getByLabelText("Cursor up")).toHaveValue("124");
  });
  it("disables the engine and restores all defaults", async () => {
    await boot();
    fireEvent.click(screen.getByRole("button", { name: "Engine enabled" }));
    await screen.findByRole("button", { name: "Engine disabled" });
    expect(saved.enabled).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "General" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Restore all defaults" }),
    );
    await screen.findByText("Defaults restored.");
    expect(saved).toEqual(defaults);
  });
  it("preserves edits and exposes backend save errors", async () => {
    await boot();
    fireEvent.click(screen.getByRole("button", { name: "Movement" }));
    fireEvent.change(screen.getByLabelText("Maximum speed"), {
      target: { value: "1700" },
    });
    invoke.mockImplementation(async (command: string) => {
      if (command === "save_config") throw new Error("Disk full");
      return snapshot();
    });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Disk full");
    expect(screen.getByLabelText("Maximum speed")).toHaveValue(1700);
    expect(saved.normal.max_speed).toBe(1400);
  });
  it("shows startup connection failures", async () => {
    invoke.mockRejectedValue("Native worker unavailable");
    render(<App />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Native worker unavailable",
    );
    expect(screen.getByRole("button", { name: "Retry" })).toBeEnabled();
  });
  it("the keyboard template edits the same binding draft as the matrix", async () => {
    await boot();
    fireEvent.click(screen.getByRole("button", { name: "Keymapping" }));
    fireEvent.click(screen.getByRole("button", { name: "Template position 2: Cursor down" }));
    fireEvent.change(screen.getByLabelText("Selected binding"), {target:{value:"127"}});
    expect(screen.getByLabelText("Cursor down")).toHaveValue("127");
    expect(screen.getByRole("alert")).toHaveTextContent("distinct binding");
    expect(screen.getByRole("button", {name:"Save changes"})).toBeDisabled();
  });
  it("refreshes unedited settings changed by the mini panel while preserving an edited draft", async () => {
    await boot();
    fireEvent.click(screen.getByRole("button", {name:"Movement"}));
    saved.normal.max_speed=1600;
    await waitFor(()=>expect(screen.getByLabelText("Maximum speed")).toHaveValue(1600),{timeout:2500});
    fireEvent.change(screen.getByLabelText("Maximum speed"),{target:{value:"1700"}});
    saved.normal.max_speed=1800;
    await waitFor(()=>expect(invoke.mock.calls.filter(([command])=>command==="get_snapshot").length).toBeGreaterThanOrEqual(3),{timeout:2500});
    expect(screen.getByLabelText("Maximum speed")).toHaveValue(1700);
  });
});
describe("curve and validation", () => {
  it("computes configured acceleration", () => {
    expect(curveSpeed(defaults.normal, 0)).toBe(40);
    expect(curveSpeed(defaults.normal, 900)).toBe(1400);
    expect(curveSpeed(defaults.normal, 450)).toBeCloseTo(
      40 + 1360 * Math.pow(0.5, 1.6),
    );
  });
  it("detects non-finite values and conflicting bindings", () => {
    const c = structuredClone(defaults);
    c.normal.min_speed = NaN;
    c.bindings.up = c.bindings.down;
    expect(validate(c)).toHaveLength(2);
  });
});
