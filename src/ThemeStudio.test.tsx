import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { beforeEach, describe, it, expect, vi } from "vitest";
import ThemeProvider, { useThemes } from "./ThemeProvider";
import ThemeStudio from "./ThemeStudio";
import {
  defaultTheme,
  exportTheme,
  initialThemeState,
  presets,
  type ThemeState,
} from "./themes";
const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listeners: new Map<string, Set<(event: { payload: unknown }) => void>>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    async (name: string, callback: (event: { payload: unknown }) => void) => {
      const group = mocks.listeners.get(name) ?? new Set();
      group.add(callback);
      mocks.listeners.set(name, group);
      return () => group.delete(callback);
    },
  ),
}));
let persisted: ThemeState;
beforeEach(() => {
  persisted = initialThemeState();
  mocks.listeners.clear();
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation(
    async (command: string, args?: { state: ThemeState }) => {
      if (command === "theme_load") return structuredClone(persisted);
      if (command === "theme_save") {
        if (args!.state.revision !== persisted.revision)
          throw Error("Stale theme revision");
        persisted = {
          ...structuredClone(args!.state),
          revision: persisted.revision + 1,
        };
        mocks.listeners
          .get("theme-state")
          ?.forEach((cb) => cb({ payload: structuredClone(persisted) }));
        return structuredClone(persisted);
      }
      throw Error("Unexpected command");
    },
  );
});
async function boot() {
  const host = render(
    <ThemeProvider>
      <ThemeStudio />
    </ThemeProvider>,
  );
  await waitFor(() => expect(mocks.invoke).toHaveBeenCalledWith("theme_load"));
  return host;
}
describe("Theme Studio behavior", () => {
  it("previews preset and color edits without changing active theme; Apply persists and Cancel discards", async () => {
    await boot();
    fireEvent.click(screen.getByRole("button", { name: "Amber Terminal" }));
    const preview = screen.getByTestId("theme-preview");
    expect(preview.style.getPropertyValue("--km-accent-primary")).toBe(
      presets[1].colors["accent-primary"],
    );
    expect(persisted.active.name).toBe("Phosphor Green");
    fireEvent.change(screen.getByLabelText("Token color"), {
      target: { value: "#123456" },
    });
    expect(preview.style.getPropertyValue("--km-bg-app")).toBe("#123456");
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(preview.style.getPropertyValue("--km-bg-app")).toBe(
      defaultTheme.colors["bg-app"],
    );
    fireEvent.click(screen.getByRole("button", { name: "Cyber Cyan" }));
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await screen.findByText("Theme applied to every window.");
    expect(persisted.active.name).toBe("Cyber Cyan");
    expect(
      document.documentElement.style.getPropertyValue("--km-accent-primary"),
    ).toBe(presets[2].colors["accent-primary"]);
  });
  it("saves, duplicates, renames and deletes custom themes without changing built-ins", async () => {
    await boot();
    fireEvent.change(screen.getByLabelText("Theme name"), {
      target: { value: "My terminal" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save Custom Theme" }));
    await screen.findByText("Custom theme saved. Apply to activate it.");
    expect(persisted.customs).toHaveLength(1);
    expect(persisted.active.id).toBe("phosphor-green");
    fireEvent.change(screen.getByLabelText("Theme name"), {
      target: { value: "Renamed" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Rename" }));
    await screen.findByText("Custom theme renamed.");
    expect(persisted.customs[0].name).toBe("Renamed");
    fireEvent.click(screen.getByRole("button", { name: "Duplicate" }));
    await waitFor(() => expect(persisted.customs).toHaveLength(2));
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await screen.findByText("Theme applied to every window.");
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await screen.findByText("Custom theme deleted.");
    expect(persisted.customs).toHaveLength(1);
    expect(persisted.active.id).toBe(defaultTheme.id);
    expect(presets[0]).toEqual(defaultTheme);
  });
  it("exports versioned JSON, imports into a draft, rejects CSS and resets without silently applying", async () => {
    await boot();
    fireEvent.click(screen.getByRole("button", { name: "Export JSON" }));
    const text = screen.getByLabelText("Theme JSON");
    expect(JSON.parse((text as HTMLTextAreaElement).value).version).toBe(1);
    fireEvent.change(text, { target: { value: exportTheme(presets[1]) } });
    fireEvent.click(screen.getByRole("button", { name: "Import JSON" }));
    expect(
      screen
        .getByTestId("theme-preview")
        .style.getPropertyValue("--km-accent-primary"),
    ).toBe(presets[1].colors["accent-primary"]);
    expect(persisted.active.id).toBe(defaultTheme.id);
    fireEvent.click(screen.getByRole("button", { name: "Reset" }));
    expect(
      screen
        .getByTestId("theme-preview")
        .style.getPropertyValue("--km-accent-primary"),
    ).toBe(defaultTheme.colors["accent-primary"]);
    fireEvent.change(text, { target: { value: '{"css":"body{}"}' } });
    fireEvent.click(screen.getByRole("button", { name: "Import JSON" }));
    expect(screen.getByRole("alert")).toHaveTextContent(
      "supported versioned fields",
    );
    expect(persisted.revision).toBe(0);
  });
  it("updates CRT settings live, clears all layers at zero, resets advanced settings and persists effects", async () => {
    await boot();
    const preview = screen.getByTestId("theme-preview");
    fireEvent.change(screen.getByLabelText("Distortion Intensity"), {
      target: { value: "100" },
    });
    fireEvent.change(screen.getByLabelText("Ghost displacement"), {
      target: { value: "1" },
    });
    expect(preview.style.getPropertyValue("--km-crt-ghost-offset")).toBe("1px");
    fireEvent.change(screen.getByLabelText("Colour misalignment strength"), {
      target: { value: "80" },
    });
    expect(preview.style.getPropertyValue("--km-crt-channel-offset")).toBe(
      "0.8px",
    );
    fireEvent.click(screen.getByRole("switch", { name: "Scanlines" }));
    fireEvent.click(screen.getByRole("switch", { name: "Screen Noise" }));
    fireEvent.click(screen.getByRole("switch", { name: "Flicker" }));
    fireEvent.change(screen.getByLabelText("Distortion Intensity"), {
      target: { value: "0" },
    });
    expect(preview.style.getPropertyValue("--km-crt-shadow")).toBe(
      "0 0 0 transparent",
    );
    expect(preview.style.getPropertyValue("--km-glow-text")).not.toBe(
      "0 0 0 transparent",
    );
    fireEvent.change(screen.getByLabelText("Phosphor Glow"), {
      target: { value: "off" },
    });
    expect(preview.style.getPropertyValue("--km-glow-text")).toBe(
      "0 0 0 transparent",
    );
    expect(preview.style.getPropertyValue("--km-scanline-display")).toBe(
      "block",
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Reset advanced settings" }),
    );
    expect(screen.getByLabelText("Ghost displacement")).toHaveValue("0.6");
    expect(screen.getByRole("switch", { name: "Flicker" })).not.toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await screen.findByText("Theme applied to every window.");
    expect(persisted.active.effects).toMatchObject({
      glow: "off",
      intensity: 0,
      scanlines: true,
      noise: true,
      displacement: 0.6,
    });
  });
  it("persists custom geometry and density and reloads them after remount", async () => {
    const host = await boot();
    fireEvent.click(screen.getByRole("button", { name: "Geometry" }));
    fireEvent.change(screen.getByLabelText("Panel radius"), {
      target: { value: "12" },
    });
    fireEvent.change(screen.getByLabelText("Interface density"), {
      target: { value: "compact" },
    });
    fireEvent.change(screen.getByLabelText("Font size"), {
      target: { value: "16" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await screen.findByText("Theme applied to every window.");
    host.unmount();
    render(
      <ThemeProvider>
        <Probe />
      </ThemeProvider>,
    );
    await screen.findByText("Phosphor Green / compact / 12");
    expect(
      document.documentElement.style.getPropertyValue("--km-font-size"),
    ).toBe("16px");
  });
  it("broadcasts accepted settings to other window providers and ignores stale events", async () => {
    await boot();
    const other = render(
      <ThemeProvider>
        <Probe />
      </ThemeProvider>,
    );
    await waitFor(() =>
      expect(mocks.listeners.get("theme-state")?.size).toBe(2),
    );
    fireEvent.click(screen.getByRole("button", { name: "Amber Terminal" }));
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await within(other.container).findByText(
      "Amber Terminal / comfortable / 4",
    );
    await act(async () => {
      mocks.listeners
        .get("theme-state")
        ?.forEach((cb) => cb({ payload: initialThemeState() }));
    });
    expect(
      within(other.container).getByText("Amber Terminal / comfortable / 4"),
    ).toBeInTheDocument();
  });
  it("keeps reset accessible after service failure, reports save errors and never duplicates accessible characters", async () => {
    mocks.invoke.mockRejectedValue(Error("Preferences unavailable"));
    await boot();
    await screen.findByRole("alert");
    expect(screen.getByRole("button", { name: "Reset" })).toBeEnabled();
    expect(screen.getAllByText("KEYMOUSE_")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "Apply" }));
    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent(
        "Preferences unavailable",
      ),
    );
  });
});
function Probe() {
  const { state } = useThemes();
  return (
    <p>
      {state.active.name} / {state.active.density} / {state.active.radius}
    </p>
  );
}
