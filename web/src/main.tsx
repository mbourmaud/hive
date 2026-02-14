import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "@/app/App";
import { Providers } from "@/app/providers";
import "./globals.css";
import "./themes.css";

// Apply persisted font size before first paint
try {
  const raw = localStorage.getItem("hive-settings");
  if (raw) {
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed === "object" && parsed !== null && "fontSize" in parsed) {
      const fs = (parsed as Record<string, unknown>).fontSize;
      if (typeof fs === "number" && fs >= 12 && fs <= 18) {
        document.documentElement.style.setProperty("--font-size-base", `${fs}px`);
        document.documentElement.style.setProperty("--font-size-code", `${fs - 1}px`);
      }
    }
  }
} catch {
  // ignore — defaults from CSS will apply
}

const root = document.getElementById("root");
if (!root) throw new Error("Root element not found");

createRoot(root).render(
  <StrictMode>
    <Providers>
      <App />
    </Providers>
  </StrictMode>,
);
