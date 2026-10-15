import "./styles.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";

// The window behaves like a program, not like a web page: no browser menu (except in the log,
// to copy), and the F1 to F12 keys neither reload nor search.
window.addEventListener("contextmenu", (event) => {
  if (!(event.target instanceof Element && event.target.closest(".log, input"))) {
    event.preventDefault();
  }
});
window.addEventListener("keydown", (event) => {
  if (/^F\d+$/.test(event.key)) {
    event.preventDefault();
  }
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
