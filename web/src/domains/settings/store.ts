import type { StateCreator } from "zustand";
import type { ThemeName } from "@/shared/theme/use-theme";
import type { StoredCustomTheme } from "@/shared/theme/custom-theme";

export type EffortLevel = "low" | "medium" | "high";

export interface SettingsSlice {
  selectedModel: string | null;
  theme: "light" | "dark";
  colorTheme: ThemeName;
  customThemes: StoredCustomTheme[];
  activeCustomThemeId: string | null;
  settingsOpen: boolean;
  commandPaletteOpen: boolean;
  sessionsModalOpen: boolean;
  statusPopoverOpen: boolean;
  effort: EffortLevel;

  setSelectedModel: (model: string | null) => void;
  toggleTheme: () => void;
  setColorTheme: (theme: ThemeName) => void;
  addCustomTheme: (theme: StoredCustomTheme) => void;
  removeCustomTheme: (id: string) => void;
  setActiveCustomTheme: (id: string | null) => void;
  setSettingsOpen: (open: boolean) => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setSessionsModalOpen: (open: boolean) => void;
  setStatusPopoverOpen: (open: boolean) => void;
  setEffort: (effort: EffortLevel) => void;
}

export const createSettingsSlice: StateCreator<SettingsSlice, [], [], SettingsSlice> = (set) => ({
  selectedModel: null,
  theme: "dark",
  colorTheme: "hive",
  customThemes: [],
  activeCustomThemeId: null,
  settingsOpen: false,
  commandPaletteOpen: false,
  sessionsModalOpen: false,
  statusPopoverOpen: false,
  effort: "medium",

  setSelectedModel: (model) => set({ selectedModel: model }),
  toggleTheme: () => set((s) => ({ theme: s.theme === "light" ? "dark" : "light" })),
  setColorTheme: (theme) => set({ colorTheme: theme, activeCustomThemeId: null }),
  addCustomTheme: (theme) =>
    set((s) => ({
      customThemes: [...s.customThemes, theme],
      activeCustomThemeId: theme.id,
    })),
  removeCustomTheme: (id) =>
    set((s) => ({
      customThemes: s.customThemes.filter((t) => t.id !== id),
      activeCustomThemeId: s.activeCustomThemeId === id ? null : s.activeCustomThemeId,
    })),
  setActiveCustomTheme: (id) => set({ activeCustomThemeId: id }),
  setSettingsOpen: (open) => set({ settingsOpen: open }),
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
  setSessionsModalOpen: (open) => set({ sessionsModalOpen: open }),
  setStatusPopoverOpen: (open) => set({ statusPopoverOpen: open }),
  setEffort: (effort) => set({ effort }),
});
