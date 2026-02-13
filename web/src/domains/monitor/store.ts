import type { StateCreator } from "zustand";

export interface MonitorSlice {
  selectedProject: string | null;
  expandedDroneId: string | null;
  dronePanelCollapsed: boolean;
  dronePanelWidth: number;
  sessionPanelWidth: number;

  setSelectedProject: (path: string | null) => void;
  setExpandedDrone: (id: string | null) => void;
  toggleDronePanel: () => void;
  setDronePanelWidth: (width: number) => void;
  setSessionPanelWidth: (width: number) => void;
}

export const createMonitorSlice: StateCreator<MonitorSlice, [], [], MonitorSlice> = (set) => ({
  selectedProject: null,
  expandedDroneId: null,
  dronePanelCollapsed: false,
  dronePanelWidth: 320,
  sessionPanelWidth: 260,

  setSelectedProject: (path) => set({ selectedProject: path }),
  setExpandedDrone: (id) => set({ expandedDroneId: id }),
  toggleDronePanel: () => set((s) => ({ dronePanelCollapsed: !s.dronePanelCollapsed })),
  setDronePanelWidth: (width) => set({ dronePanelWidth: width }),
  setSessionPanelWidth: (width) => set({ sessionPanelWidth: width }),
});
