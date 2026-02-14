import type { StateCreator } from "zustand";
import type { ThemeName } from "@/shared/theme/use-theme";

export type EffortLevel = "low" | "medium" | "high";

export interface SettingsSlice {
  selectedModel: string | null;
  theme: "light" | "dark";
  colorTheme: ThemeName;
  settingsOpen: boolean;
  commandPaletteOpen: boolean;
  sessionsModalOpen: boolean;
  effort: EffortLevel;

  setSelectedModel: (model: string | null) => void;
  toggleTheme: () => void;
  setColorTheme: (theme: ThemeName) => void;
  setSettingsOpen: (open: boolean) => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setSessionsModalOpen: (open: boolean) => void;
  setEffort: (effort: EffortLevel) => void;
}

export const createSettingsSlice: StateCreator<SettingsSlice, [], [], SettingsSlice> = (set) => ({
  selectedModel: null,
  theme: "dark",
  colorTheme: "hive",
  settingsOpen: false,
  commandPaletteOpen: false,
  sessionsModalOpen: false,
  effort: "medium",

  setSelectedModel: (model) => set({ selectedModel: model }),
  toggleTheme: () => set((s) => ({ theme: s.theme === "light" ? "dark" : "light" })),
  setColorTheme: (theme) => set({ colorTheme: theme }),
  setSettingsOpen: (open) => set({ settingsOpen: open }),
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
  setSessionsModalOpen: (open) => set({ sessionsModalOpen: open }),
  setEffort: (effort) => set({ effort }),
});
