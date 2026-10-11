import { useState } from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import Refinements from "./Refinements";
import { defaultFeatures, type Config } from "./types";
describe("independent movement refinements", () => {
  it("toggles each feature independently and retains parameters", () => {
    let current = { features: structuredClone(defaultFeatures) } as Config;
    function Host() {
      const [c, set] = useState(current);
      return (
        <Refinements
          config={c}
          setConfig={(next) => {
            current = next;
            set(next);
          }}
        />
      );
    }
    render(<Host />);
    for (const label of [
      "Enable Tap versus Hold",
      "Enable acceleration strategy",
      "Enable Adaptive Braking",
      "Enable Per-Axis Acceleration",
      "Enable precision magnifier",
    ]) {
      fireEvent.click(screen.getByRole("switch", { name: label }));
      expect(screen.getByRole("switch", { name: label })).toBeChecked();
    }
    fireEvent.change(screen.getByLabelText("Tap step (pixels)"), {
      target: { value: "3" },
    });
    fireEvent.click(
      screen.getByRole("switch", { name: "Enable Tap versus Hold" }),
    );
    expect(current.features.tap).toMatchObject({ enabled: false, step: 3 });
    expect(current.features.braking.enabled).toBe(true);
    expect(current.features.axes.enabled).toBe(true);
    expect(current.features.acceleration.enabled).toBe(true);
  });
});
