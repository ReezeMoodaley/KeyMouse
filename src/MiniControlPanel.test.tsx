import { beforeEach, it, expect, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import MiniControlPanel from "./MiniControlPanel";
import { actions, defaultFeatures, type Config, type Snapshot } from "./types";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
let config: Config;
const snapshot = (): Snapshot => ({
  config: structuredClone(config),
  status: {
    normal_max_speed: 1400,
    running: true,
    enabled: config.enabled,
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
  version: "0.1.0",
  warning: null,
});
beforeEach(() => {
  config = {
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
  invoke.mockReset();
  invoke.mockImplementation(
    async (command: string, args?: { config: Config; enabled: boolean }) => {
      if (command === "presentation_snapshot")
        return {
          effective: config,
          displays: [],
          scale_factor: 1,
          executable: "C:\\app.exe",
          matching_profile: null,
          calibration_active: false,
        };
      if (command === "save_config") config = args!.config;
      if (command === "set_enabled") config.enabled = args!.enabled;
      return snapshot();
    },
  );
});
it("mini controls use saved configuration and real window routing", async () => {
  render(<MiniControlPanel />);
  await screen.findByRole("button", { name: "Engine enabled" });
  fireEvent.click(screen.getByRole("button", { name: "Open Theme Studio" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("open_page", { page: "Theme Studio" }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Open Full App" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("open_page", { page: "Overview" }),
  );
  fireEvent.click(screen.getByRole("switch", { name: "braking" }));
  await waitFor(() => expect(config.features.braking.enabled).toBe(true));
  expect(config.features.tap.enabled).toBe(false);
  expect(config.normal.max_speed).toBe(1400);
});
it("mini calibration starts explicitly with the selected group and respects availability", async () => {
  config.features.calibration.enabled = true;
  render(<MiniControlPanel />);
  await screen.findByRole("button", { name: "Engine enabled" });
  expect(invoke).not.toHaveBeenCalledWith(
    "calibration_command",
    expect.anything(),
  );
  fireEvent.change(screen.getByLabelText("Calibration group"), {
    target: { value: "experimental" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Start Calibration" }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("calibration_command", {
      action: "experimental",
    }),
  );
});
it("mini cannot start disabled calibration and can disable the engine", async () => {
  render(<MiniControlPanel />);
  await screen.findByRole("button", { name: "Engine enabled" });
  expect(
    screen.getByRole("button", { name: "Start Calibration" }),
  ).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Engine enabled" }));
  await screen.findByRole("button", { name: "Engine disabled" });
  expect(config.enabled).toBe(false);
});
it("mini reports IPC failures", async () => {
  invoke.mockRejectedValue(Error("Native service unavailable"));
  render(<MiniControlPanel />);
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Native service unavailable",
  );
});
