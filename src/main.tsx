import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Grid from "./Grid";
import "./styles.css";
const grid = new URLSearchParams(location.search).has("grid");
if (grid) document.documentElement.dataset.grid = "true";
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{grid ? <Grid /> : <App />}</React.StrictMode>,
);
