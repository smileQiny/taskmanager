import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

if (
  window.location.search.includes("window=cockpit")
  || window.location.pathname === "/cockpit"
) {
  document.documentElement.classList.add("cockpit-window");
  document.body.classList.add("cockpit-window");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
