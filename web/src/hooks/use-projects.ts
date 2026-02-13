import { useState, useEffect, useRef, useCallback } from "react";
import type { ProjectInfo, DroneInfo } from "@/types/api";
import { MOCK_PROJECTS } from "@/data/mock";

type ConnectionStatus = "connected" | "disconnected" | "mock";

export function useProjects() {
  const [projects, setProjects] = useState<ProjectInfo[]>([]);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>("disconnected");
  const eventSourceRef = useRef<EventSource | null>(null);
  const retryTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const connectSSE = useCallback(() => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
    }

    const es = new EventSource("/api/events");
    eventSourceRef.current = es;

    es.onopen = () => {
      setConnectionStatus("connected");
    };

    es.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        // Detect format: ProjectInfo[] vs DroneInfo[]
        if (Array.isArray(data) && data.length > 0 && "drones" in data[0]) {
          // New format: ProjectInfo[]
          setProjects(data as ProjectInfo[]);
        } else if (Array.isArray(data)) {
          // Legacy format: DroneInfo[] — wrap in a single project
          setProjects(wrapDronesAsProject(data as DroneInfo[]));
        }
        setConnectionStatus("connected");
      } catch {
        // ignore parse errors
      }
    };

    es.onerror = () => {
      es.close();
      setConnectionStatus("disconnected");
      retryTimeoutRef.current = setTimeout(connectSSE, 3000);
    };
  }, []);

  useEffect(() => {
    // Force mock mode with ?mock query param
    const forceMock = new URLSearchParams(window.location.search).has("mock");
    if (forceMock) {
      console.info("Mock mode forced via ?mock query param");
      setProjects(MOCK_PROJECTS);
      setConnectionStatus("mock");
      return;
    }

    // Try new /api/projects endpoint first, fallback to /api/drones
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 2000);

    fetch("/api/projects", { signal: controller.signal })
      .then((res) => {
        clearTimeout(timeout);
        if (!res.ok) throw new Error("No backend");
        return res.json();
      })
      .then((data: ProjectInfo[]) => {
        setProjects(data);
        connectSSE();
      })
      .catch(() => {
        clearTimeout(timeout);
        // Try legacy /api/drones
        const controller2 = new AbortController();
        const timeout2 = setTimeout(() => controller2.abort(), 2000);

        fetch("/api/drones", { signal: controller2.signal })
          .then((res) => {
            clearTimeout(timeout2);
            if (!res.ok) throw new Error("No backend");
            return res.json();
          })
          .then((data: DroneInfo[]) => {
            setProjects(wrapDronesAsProject(data));
            connectSSE();
          })
          .catch(() => {
            clearTimeout(timeout2);
            console.info("No backend detected, using mock data for preview");
            setProjects(MOCK_PROJECTS);
            setConnectionStatus("mock");
          });
      });

    return () => {
      eventSourceRef.current?.close();
      if (retryTimeoutRef.current) {
        clearTimeout(retryTimeoutRef.current);
      }
    };
  }, [connectSSE]);

  return { projects, connectionStatus };
}

function wrapDronesAsProject(drones: DroneInfo[]): ProjectInfo[] {
  if (drones.length === 0) return [];
  const totalCost = drones.reduce((sum, d) => sum + (d.cost?.total_usd || 0), 0);
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
