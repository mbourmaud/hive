import { useCallback, useEffect, useRef } from "react";
import { useProjectRegistryQuery } from "@/domains/projects/queries";
import type { ProjectProfile } from "@/domains/projects/types";
import { useDetection } from "@/domains/projects/use-detection";
import { THEMES, useTheme } from "@/shared/theme/use-theme";
import { useAppStore } from "@/store";

const CACHE_TTL = 5 * 60 * 1000; // 5 minutes

/**
 * Manages project detection lifecycle: registry sync, auto-detection on
 * project switch, caching detection results, and onboarding completion.
 */
export function useProjectDetection() {
  const selectedProject = useAppStore((s) => s.selectedProject);
  const setSelectedProject = useAppStore((s) => s.setSelectedProject);
  const registryProjects = useAppStore((s) => s.registryProjects);
  const setRegistryProjects = useAppStore((s) => s.setRegistryProjects);
  const activeProjectContext = useAppStore((s) => s.activeProjectContext);
  const setActiveProjectContext = useAppStore((s) => s.setActiveProjectContext);
  const contextCacheTime = useAppStore((s) => s.contextCacheTime);
  const setContextCacheTime = useAppStore((s) => s.setContextCacheTime);
  const onboardingComplete = useAppStore((s) => s.onboardingComplete);
  const setOnboardingComplete = useAppStore((s) => s.setOnboardingComplete);

  const { setThemeName } = useTheme();
  const { context: detectedContext, isDetecting, startDetection } = useDetection();
  const { data: registryData } = useProjectRegistryQuery();

  // ── Sync registry data to store ────────────────────────────────────────
  useEffect(() => {
    if (registryData && registryData.length > 0) {
      setRegistryProjects(registryData);
    }
  }, [registryData, setRegistryProjects]);

  // ── Auto-select project when none is selected ─────────────────────────
  useEffect(() => {
    const first = registryProjects[0];
    if (!first) return;
    // No project selected → pick first
    if (!selectedProject) {
      setSelectedProject(first.path);
      return;
    }
    // Selected project no longer exists in registry → pick first
    const stillExists = registryProjects.some((p) => p.path === selectedProject);
    if (!stillExists) {
      setSelectedProject(first.path);
    }
  }, [registryProjects, selectedProject, setSelectedProject]);

  // ── Auto-detect context on project switch ──────────────────────────────
  const prevDetectProjectRef = useRef<string | null>(null);
  useEffect(() => {
    if (!selectedProject) return;

    if (
      selectedProject === prevDetectProjectRef.current &&
      activeProjectContext &&
      contextCacheTime &&
      Date.now() - contextCacheTime < CACHE_TTL
    ) {
      return;
    }

    const regProject = registryProjects.find((p) => p.path === selectedProject);
    if (!regProject) return;

    prevDetectProjectRef.current = selectedProject;
    setActiveProjectContext(null);
    startDetection(regProject.id);
  }, [
    selectedProject,
    registryProjects,
    activeProjectContext,
    contextCacheTime,
    startDetection,
    setActiveProjectContext,
  ]);

  // ── Cache detection results ────────────────────────────────────────────
  useEffect(() => {
    if (detectedContext) {
      setActiveProjectContext(detectedContext);
      setContextCacheTime(Date.now());
    }
  }, [detectedContext, setActiveProjectContext, setContextCacheTime]);

  // ── Onboarding completion handler ──────────────────────────────────────
  const handleOnboardingComplete = useCallback(
    (project: ProjectProfile) => {
      setSelectedProject(project.path);
      if (project.color_theme) {
        const themeInfo = THEMES.find((t) => t.name === project.color_theme);
        if (themeInfo) {
          setThemeName(themeInfo.name);
        }
      }
      setOnboardingComplete(true);
    },
    [setSelectedProject, setThemeName, setOnboardingComplete],
  );

  return {
    registryProjects,
    activeProjectContext,
    isDetecting,
    onboardingComplete,
    handleOnboardingComplete,
  };
}
