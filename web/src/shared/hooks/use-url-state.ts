import { useEffect, useRef } from "react";
import type { ProjectInfo } from "@/domains/monitor/types";
import { useAppStore } from "@/store";

// ── URL parsing helpers ──────────────────────────────────────────────────────

interface ParsedPath {
  projectName: string | null;
  sessionId: string | null;
}

function parsePathname(pathname: string): ParsedPath {
  const segments = pathname.split("/").filter(Boolean);
  return {
    projectName: segments[0] ?? null,
    sessionId: segments[1] ?? null,
  };
}

function buildPathname(projectName: string | null, sessionId: string | null): string {
  if (!projectName) return "/";
  if (!sessionId) return `/${encodeURIComponent(projectName)}`;
  return `/${encodeURIComponent(projectName)}/${encodeURIComponent(sessionId)}`;
}

function findProjectByName(projects: ProjectInfo[], name: string | null): ProjectInfo | undefined {
  if (!name) return undefined;
  return projects.find((p) => p.name.toLowerCase() === name.toLowerCase());
}

function findProjectNameByPath(projects: ProjectInfo[], path: string | null): string | null {
  if (!path) return null;
  return projects.find((p) => p.path === path)?.name ?? null;
}

// ── Hook ─────────────────────────────────────────────────────────────────────

/**
 * Syncs the browser URL with Zustand's selectedProject + activeSessionId.
 *
 * - On mount: if the URL has path segments, they take priority over localStorage.
 * - On state changes: pushState updates the address bar.
 * - On popstate (back/forward): Zustand is updated from the URL.
 */
export function useUrlState(projects: ProjectInfo[]): void {
  const selectedProject = useAppStore((s) => s.selectedProject);
  const setSelectedProject = useAppStore((s) => s.setSelectedProject);
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const setActiveSessionId = useAppStore((s) => s.setActiveSession);

  const initializedRef = useRef(false);
  const suppressPushRef = useRef(false);
  const prevUrlRef = useRef(window.location.pathname);

  // ── 1. Initial mount: URL → Zustand (runs once when projects are loaded) ─
  useEffect(() => {
    if (initializedRef.current) return;
    if (projects.length === 0) return;
    initializedRef.current = true;

    const { projectName, sessionId } = parsePathname(window.location.pathname);

    if (!projectName) return;

    const project = findProjectByName(projects, projectName);
    if (!project) return;

    suppressPushRef.current = true;
    setSelectedProject(project.path);
    if (sessionId) {
      setActiveSessionId(sessionId);
    }
    // replaceState to normalize (e.g. fix casing)
    const normalized = buildPathname(project.name, sessionId);
    if (window.location.pathname !== normalized) {
      window.history.replaceState(null, "", normalized);
    }
  }, [projects, setSelectedProject, setActiveSessionId]);

  // ── 2. Zustand → URL (push/replaceState on state changes) ────────────────
  useEffect(() => {
    // Skip the pushState that would result from our own URL→Zustand sync
    if (suppressPushRef.current) {
      suppressPushRef.current = false;
      prevUrlRef.current = window.location.pathname;
      return;
    }

    const projectName = findProjectNameByPath(projects, selectedProject);
    const newPath = buildPathname(projectName, activeSessionId);

    if (newPath !== prevUrlRef.current) {
      // Use replaceState for the very first URL update (no history entry for auto-select),
      // pushState for user-initiated navigation.
      const method = prevUrlRef.current === "/" ? "replaceState" : "pushState";
      window.history[method](null, "", newPath);
      prevUrlRef.current = newPath;
    }
  }, [selectedProject, activeSessionId, projects]);

  // ── 3. popstate (browser back/forward) → Zustand ─────────────────────────
  useEffect(() => {
    function handlePopState() {
      const { projectName, sessionId } = parsePathname(window.location.pathname);
      prevUrlRef.current = window.location.pathname;
      suppressPushRef.current = true;

      const project = findProjectByName(projects, projectName);
      if (project) {
        setSelectedProject(project.path);
        setActiveSessionId(sessionId);
      } else {
        if (projects[0]) {
          setSelectedProject(projects[0].path);
        }
        setActiveSessionId(null);
      }
    }

    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  }, [projects, setSelectedProject, setActiveSessionId]);
}
