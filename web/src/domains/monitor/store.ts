import type { StateCreator } from "zustand";

export type RightSidebarTab = "drones" | "plans" | "context" | "git";

export interface MonitorSlice {
  selectedProject: string | null;
  expandedDroneId: string | null;
  rightSidebarTab: RightSidebarTab;
  rightSidebarCollapsed: boolean;
  sessionPanelWidth: number;

  setSelectedProject: (path: string | null) => void;
  setExpandedDrone: (id: string | null) => void;
  setRightSidebarTab: (tab: RightSidebarTab) => void;
  toggleRightSidebar: () => void;
  openRightSidebar: (tab: RightSidebarTab) => void;
  setSessionPanelWidth: (width: number) => void;
}

export const createMonitorSlice: StateCreator<MonitorSlice, [], [], MonitorSlice> = (set) => ({
  selectedProject: null,
  expandedDroneId: null,
  rightSidebarTab: "drones",
  rightSidebarCollapsed: false,
  sessionPanelWidth: 260,

  setSelectedProject: (path) => set({ selectedProject: path }),
  setExpandedDrone: (id) => set({ expandedDroneId: id }),
  setRightSidebarTab: (tab) => set({ rightSidebarTab: tab }),
  toggleRightSidebar: () => set((s) => ({ rightSidebarCollapsed: !s.rightSidebarCollapsed })),
  openRightSidebar: (tab) => set({ rightSidebarTab: tab, rightSidebarCollapsed: false }),
  setSessionPanelWidth: (width) => set({ sessionPanelWidth: width }),
});
