import { create } from "zustand";
import { devtools, persist } from "zustand/middleware";
import { type ChatSlice, createChatSlice } from "@/domains/chat/store";
import { createMonitorSlice, type MonitorSlice } from "@/domains/monitor/store";
import { createSettingsSlice, type SettingsSlice } from "@/domains/settings/store";

export type AppStore = ChatSlice & MonitorSlice & SettingsSlice;

export const useAppStore = create<AppStore>()(
  devtools(
    persist(
      (...a) => ({
        ...createChatSlice(...a),
        ...createMonitorSlice(...a),
        ...createSettingsSlice(...a),
      }),
      {
        name: "hive-store",
        partialize: (state) => ({
          selectedProject: state.selectedProject,
          selectedModel: state.selectedModel,
          theme: state.theme,
          colorTheme: state.colorTheme,
          dronePanelCollapsed: state.dronePanelCollapsed,
        }),
      },
    ),
    { name: "HiveStore" },
  ),
);
