import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import ThemeProvider from "./ThemeProvider";
import MiniControlPanel from "./MiniControlPanel";
import Grid from "./Grid";
import { CalibrationOverlay } from "./Calibration";
import "./styles.css";
import "./terminal.css";
import { defaultTheme, themeStyle } from "./themes";
for (const [key, value] of Object.entries(themeStyle(defaultTheme)))
  document.documentElement.style.setProperty(key, String(value));
const mini = new URLSearchParams(location.search).has("mini");
const grid = new URLSearchParams(location.search).has("grid");
const calibration = new URLSearchParams(location.search).has("calibration");
if (grid || calibration) document.documentElement.dataset.grid = "true";
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ThemeProvider>
      {grid ? (
        <Grid />
      ) : calibration ? (
        <CalibrationOverlay />
      ) : mini ? (
        <MiniControlPanel />
      ) : (
        <App />
      )}
    </ThemeProvider>
  </React.StrictMode>,
);
