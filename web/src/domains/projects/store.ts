import type { StateCreator } from "zustand";
import type { ProjectContext, ProjectProfile } from "./types";

export interface ProjectsSlice {
  registryProjects: ProjectProfile[];
  activeProjectContext: ProjectContext | null;
  contextCacheTime: number | null;
  onboardingComplete: boolean;

  setRegistryProjects: (projects: ProjectProfile[]) => void;
  setActiveProjectContext: (context: ProjectContext | null) => void;
  setContextCacheTime: (time: number | null) => void;
  setOnboardingComplete: (complete: boolean) => void;
}

export const createProjectsSlice: StateCreator<ProjectsSlice, [], [], ProjectsSlice> = (set) => ({
  registryProjects: [],
  activeProjectContext: null,
  contextCacheTime: null,
  onboardingComplete: false,

  setRegistryProjects: (projects) => set({ registryProjects: projects }),
  setActiveProjectContext: (context) => set({ activeProjectContext: context }),
  setContextCacheTime: (time) => set({ contextCacheTime: time }),
  setOnboardingComplete: (complete) => set({ onboardingComplete: complete }),
});
