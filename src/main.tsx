import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

function applyTheme() {
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)");
  const html = document.documentElement;

  function update(dark: boolean) {
    if (dark) {
      html.classList.add("dark");
      html.classList.remove("light");
    } else {
      html.classList.add("light");
      html.classList.remove("dark");
    }
  }

  update(prefersDark.matches);
  prefersDark.addEventListener("change", (e) => update(e.matches));
}

applyTheme();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
