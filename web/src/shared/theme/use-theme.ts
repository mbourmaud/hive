import { useEffect } from "react";
import { useAppStore } from "@/store";

// ── Types ────────────────────────────────────────────────────────────────────

export type ThemeName =
  | "hive"
  | "catppuccin"
  | "dracula"
  | "gruvbox"
  | "onedark"
  | "tokyonight"
  | "monokai"
  | "flexoki"
  | "tron";

export interface ThemeInfo {
  name: ThemeName;
  label: string;
  accent: string;
  bg: string;
}

// ── Theme registry ──────────────────────────────────────────────────────────

export const THEMES: ThemeInfo[] = [
  { name: "hive", label: "Hive", accent: "oklch(0.77 0.19 70)", bg: "oklch(0.145 0.005 286)" },
  {
    name: "catppuccin",
    label: "Catppuccin",
    accent: "oklch(0.74 0.12 230)",
    bg: "oklch(0.24 0.015 270)",
  },
  {
    name: "dracula",
    label: "Dracula",
    accent: "oklch(0.72 0.18 320)",
    bg: "oklch(0.23 0.025 290)",
  },
  { name: "gruvbox", label: "Gruvbox", accent: "oklch(0.72 0.16 55)", bg: "oklch(0.22 0.02 60)" },
  {
    name: "onedark",
    label: "One Dark",
    accent: "oklch(0.68 0.16 250)",
    bg: "oklch(0.24 0.012 260)",
  },
  {
    name: "tokyonight",
    label: "Tokyo Night",
    accent: "oklch(0.72 0.14 275)",
    bg: "oklch(0.21 0.025 275)",
  },
  { name: "monokai", label: "Monokai", accent: "oklch(0.72 0.2 340)", bg: "oklch(0.23 0.01 80)" },
  { name: "flexoki", label: "Flexoki", accent: "oklch(0.68 0.12 55)", bg: "oklch(0.20 0.015 55)" },
  { name: "tron", label: "Tron", accent: "oklch(0.75 0.18 195)", bg: "oklch(0.12 0.005 210)" },
];

// ── Hook (reads from Zustand, syncs DOM attributes) ─────────────────────────

export function useTheme() {
  const theme = useAppStore((s) => s.theme);
  const colorTheme = useAppStore((s) => s.colorTheme);
  const toggleTheme = useAppStore((s) => s.toggleTheme);
  const setColorTheme = useAppStore((s) => s.setColorTheme);

  useEffect(() => {
    const root = document.documentElement;
    root.setAttribute("data-theme", theme);

    if (colorTheme === "hive") {
      root.removeAttribute("data-color-theme");
    } else {
      root.setAttribute("data-color-theme", colorTheme);
    }
  }, [theme, colorTheme]);

  return { theme, toggleTheme, themeName: colorTheme, setThemeName: setColorTheme };
}
