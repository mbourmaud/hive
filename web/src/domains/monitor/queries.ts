import { queryOptions, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";
import { apiClient } from "@/shared/api/client";
import { queryKeys } from "@/shared/api/query-keys";
import { MOCK_PROJECTS } from "@/shared/data/mock";
import type { DroneInfo, ProjectInfo } from "./types";

// ── Type guards ──────────────────────────────────────────────────────────────

function isProjectArray(data: unknown): data is ProjectInfo[] {
  if (!Array.isArray(data) || data.length === 0) return false;
  const first: unknown = data[0];
  return typeof first === "object" && first !== null && "drones" in first;
}

function isDroneArray(data: unknown): data is DroneInfo[] {
  if (!Array.isArray(data) || data.length === 0) return false;
  const first: unknown = data[0];
  return typeof first === "object" && first !== null && "liveness" in first;
}

// ── Query Options ────────────────────────────────────────────────────────────

export const projectsQueryOptions = queryOptions({
  queryKey: queryKeys.projects.all(),
  queryFn: () => apiClient.get<ProjectInfo[]>("/api/projects"),
  staleTime: Infinity, // SSE keeps data fresh
});

// ── SSE Bridge ───────────────────────────────────────────────────────────────

type ConnectionStatus = "connected" | "disconnected" | "mock";

const FORCE_MOCK = new URLSearchParams(window.location.search).has("mock");

function wrapDronesAsProject(drones: DroneInfo[]): ProjectInfo[] {
  if (drones.length === 0) return [];
  const totalCost = drones.reduce((sum, d) => sum + (d.cost?.total_usd ?? 0), 0);
  const activeCount = drones.filter((d) => d.liveness === "working").length;
  return [
    {
      name: "Current Project",
      path: "",
      drones,
      total_cost: totalCost,
      active_count: activeCount,
    },
  ];
}

/** JSON fetch at the API boundary — `as T` is acceptable here (see client.ts) */
async function fetchJson<T>(url: string, signal: AbortSignal): Promise<T> {
  const res = await fetch(url, { signal });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- API boundary
  const data: unknown = await res.json();
  return data as T;
}

/**
 * Hook that bridges SSE drone events into TanStack Query cache.
 */
export function useProjectsSSE(): {
  data: ProjectInfo[] | undefined;
  connectionStatus: ConnectionStatus;
  isLoading: boolean;
} {
  const queryClient = useQueryClient();
  const query = useQuery(projectsQueryOptions);

  useEffect(() => {
    if (FORCE_MOCK) {
      queryClient.setQueryData(queryKeys.projects.all(), MOCK_PROJECTS);
      return;
    }

    const timeouts: ReturnType<typeof setTimeout>[] = [];
    const controllers: AbortController[] = [];

    function timedFetch<T>(url: string, timeoutMs = 2000): Promise<T> {
      const ctrl = new AbortController();
      controllers.push(ctrl);
      const tid = setTimeout(() => ctrl.abort(), timeoutMs);
      timeouts.push(tid);
      return fetchJson<T>(url, ctrl.signal).finally(() => clearTimeout(tid));
    }

    // Try /api/projects, fallback to /api/drones, fallback to mock
    timedFetch<ProjectInfo[]>("/api/projects", 5000)
      .then((data) => {
        queryClient.setQueryData(queryKeys.projects.all(), data);
      })
      .catch(() =>
        timedFetch<DroneInfo[]>("/api/drones", 5000)
          .then((data) => {
            queryClient.setQueryData(queryKeys.projects.all(), wrapDronesAsProject(data));
          })
          .catch(() => {
            queryClient.setQueryData(queryKeys.projects.all(), MOCK_PROJECTS);
          }),
      );

    // SSE connection
    const es = new EventSource("/api/events");

    es.onmessage = (event) => {
      try {
        const parsed: unknown = JSON.parse(event.data);
        if (!Array.isArray(parsed)) return;

        if (isProjectArray(parsed)) {
          queryClient.setQueryData(queryKeys.projects.all(), parsed);
        } else if (isDroneArray(parsed)) {
          queryClient.setQueryData(queryKeys.projects.all(), wrapDronesAsProject(parsed));
        }
      } catch {
        // ignore parse errors
      }
    };

    es.onerror = () => {
      es.close();
    };

    return () => {
      es.close();
      for (const t of timeouts) clearTimeout(t);
      for (const c of controllers) c.abort();
    };
  }, [queryClient]);

  const connectionStatus: ConnectionStatus = useMemo(() => {
    if (FORCE_MOCK) return "mock";
    if (query.data && query.data.length > 0) return "connected";
    return "disconnected";
  }, [query.data]);

  return {
    data: query.data,
    connectionStatus,
    isLoading: query.isLoading,
  };
}
