import { create } from "zustand";
import { devtools, persist } from "zustand/middleware";
import { type ChatSlice, createChatSlice } from "@/domains/chat/store";
import { createMonitorSlice, type MonitorSlice } from "@/domains/monitor/store";
import { createProjectsSlice, type ProjectsSlice } from "@/domains/projects/store";
import { createSettingsSlice, type SettingsSlice } from "@/domains/settings/store";

export type AppStore = ChatSlice & MonitorSlice & SettingsSlice & ProjectsSlice;

export const useAppStore = create<AppStore>()(
  devtools(
    persist(
      (...a) => ({
        ...createChatSlice(...a),
        ...createMonitorSlice(...a),
        ...createSettingsSlice(...a),
        ...createProjectsSlice(...a),
      }),
      {
        name: "hive-store",
        partialize: (state) => ({
          selectedProject: state.selectedProject,
          selectedModel: state.selectedModel,
          theme: state.theme,
          colorTheme: state.colorTheme,
          customThemes: state.customThemes,
          activeCustomThemeId: state.activeCustomThemeId,
          rightSidebarTab: state.rightSidebarTab,
          rightSidebarCollapsed: state.rightSidebarCollapsed,
          activeSessionId: state.activeSessionId,
          effort: state.effort,
          chatMode: state.chatMode,
          onboardingComplete: state.onboardingComplete,
        }),
      },
    ),
    { name: "HiveStore" },
  ),
);
