import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Grid from "./Grid";
import { CalibrationOverlay } from "./Calibration";
import "./styles.css";
const grid = new URLSearchParams(location.search).has("grid");
const calibration = new URLSearchParams(location.search).has("calibration");
if (grid || calibration) document.documentElement.dataset.grid = "true";
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {grid ? <Grid /> : calibration ? <CalibrationOverlay /> : <App />}
  </React.StrictMode>,
);
