import type { StateCreator } from "zustand";
import type { ThemeName } from "@/shared/theme/use-theme";

export interface SettingsSlice {
  selectedModel: string | null;
  theme: "light" | "dark";
  colorTheme: ThemeName;
  settingsOpen: boolean;
  commandPaletteOpen: boolean;

  setSelectedModel: (model: string | null) => void;
  toggleTheme: () => void;
  setColorTheme: (theme: ThemeName) => void;
  setSettingsOpen: (open: boolean) => void;
  setCommandPaletteOpen: (open: boolean) => void;
}

export const createSettingsSlice: StateCreator<SettingsSlice, [], [], SettingsSlice> = (set) => ({
  selectedModel: null,
  theme: "dark",
  colorTheme: "hive",
  settingsOpen: false,
  commandPaletteOpen: false,

  setSelectedModel: (model) => set({ selectedModel: model }),
  toggleTheme: () => set((s) => ({ theme: s.theme === "light" ? "dark" : "light" })),
  setColorTheme: (theme) => set({ colorTheme: theme }),
  setSettingsOpen: (open) => set({ settingsOpen: open }),
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
});
