import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import Grid from "./Grid";
import type { GridSession } from "./types";
const { invoke, listen, off } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  off: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
const session: GridSession = {
  monitors: [
    { x: -1920, y: 0, width: 1920, height: 1080 },
    { x: 0, y: 0, width: 2560, height: 1440 },
  ],
  monitor: 0,
  region: { x: -1920, y: 0, width: 1920, height: 1080 },
  level: 0,
  depth: 3,
};
beforeEach(() => {
  invoke.mockReset();
  listen.mockReset();
  off.mockReset();
  listen.mockResolvedValue(off);
  invoke.mockImplementation(async (command: string) =>
    command === "grid_snapshot" ? session : null,
  );
});
it("registers events before reading physical monitor geometry and refines with keys", async () => {
  const view = render(<Grid />);
  await screen.findByText(/Monitor 1\/2/);
  expect(listen).toHaveBeenCalledWith("grid-state", expect.any(Function));
  expect(view.container.querySelector("svg")).toHaveAttribute(
    "viewBox",
    "0 0 1920 1080",
  );
  invoke.mockResolvedValue({
    ...session,
    level: 1,
    region: { x: -1280, y: 360, width: 640, height: 360 },
  });
  fireEvent.keyDown(window, { key: "5" });
  await screen.findByText(/Level 2\/3/);
  expect(invoke).toHaveBeenCalledWith("grid_command", {
    action: "select",
    region: 5,
  });
  expect(view.container.querySelector("rect")).toHaveAttribute("x", "640");
  view.unmount();
  expect(off).toHaveBeenCalledOnce();
});
it("ignores key repeat and supports cancel, restart and monitor switching", async () => {
  render(<Grid />);
  await screen.findByText(/Monitor 1\/2/);
  fireEvent.keyDown(window, { key: "1", repeat: true });
  expect(invoke).not.toHaveBeenCalledWith("grid_command", expect.anything());
  invoke.mockResolvedValue(session);
  for (const [key, action] of [
    ["Backspace", "restart"],
    ["Tab", "monitor"],
    ["Escape", "cancel"],
  ]) {
    fireEvent.keyDown(window, { key });
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("grid_command", {
        action,
        region: null,
      }),
    );
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
});
it("exposes placement errors and keeps cancellation available", async () => {
  render(<Grid />);
  await screen.findByText(/Monitor 1\/2/);
  invoke.mockRejectedValue("Cursor placement denied");
  fireEvent.keyDown(window, { key: "Enter" });
  expect(await screen.findByRole("alert")).toHaveTextContent(
    "Cursor placement denied",
  );
  expect(screen.getByRole("button", { name: "Cancel" })).toBeEnabled();
});
